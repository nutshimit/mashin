use anyhow::{anyhow, bail, Result};
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_core::error::AnyError;
use deno_core::futures::FutureExt;
use deno_core::serde_json::json;
use deno_core::{
    include_js_files, resolve_import, resolve_path, Extension, JsRuntime, ModuleLoader,
    ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType, OpDecl, ResolutionKind,
    RuntimeOptions,
};
use deno_fetch::FetchPermissions;
use deno_web::BlobStore;
use deno_web::TimersPermission;
use deno_websocket::WebSocketPermissions;
use reqwest::Url;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
mod builtin;

pub enum Subcommand {
    Run { main_module: String, dry_run: bool },
    Destroy { main_module: String },
}

struct AllowAllPermissions;

impl FetchPermissions for AllowAllPermissions {
    fn check_net_url(&mut self, _url: &deno_core::url::Url, _api_name: &str) -> Result<()> {
        Ok(())
    }

    fn check_read(&mut self, _path: &Path, _api_name: &str) -> Result<()> {
        Ok(())
    }
}

impl TimersPermission for AllowAllPermissions {
    fn allow_hrtime(&mut self) -> bool {
        true
    }

    fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
        // allow unstable apis
    }
}

impl WebSocketPermissions for AllowAllPermissions {
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> std::result::Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

impl mashin_ffi::FfiPermissions for AllowAllPermissions {
    fn check(&mut self, _path: Option<&Path>) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

pub fn create_js_runtime() -> JsRuntime {
    let extension = Extension::builder("mashin_core")
        .esm(include_js_files!(
            mashin_core dir "js",
            "01_errors.js",
            "06_util.js",
            "30_os.js",
            "98_global_scope.js",
            "99_main.js",
        ))
        .ops(stdlib())
        .state(move |state| {
            state.put(AllowAllPermissions {});
            state.put(mashin_ffi::Unstable(true));
        })
        .build();

    JsRuntime::new(RuntimeOptions {
        extensions: vec![
            deno_console::deno_console::init_ops_and_esm(),
            deno_webidl::deno_webidl::init_ops_and_esm(),
            deno_url::deno_url::init_ops_and_esm(),
            deno_web::deno_web::init_ops_and_esm::<AllowAllPermissions>(BlobStore::default(), None),
            deno_fetch::deno_fetch::init_ops_and_esm::<AllowAllPermissions>(deno_fetch::Options {
                user_agent: format!("mashin_core/{}", env!("CARGO_PKG_VERSION")),
                ..Default::default()
            }),
            deno_websocket::deno_websocket::init_ops_and_esm::<AllowAllPermissions>(
                format!("mashin_core/{}", env!("CARGO_PKG_VERSION")),
                None,
                None,
            ),
            mashin_ffi::deno_ffi::init_ops_and_esm::<AllowAllPermissions>(
                true, // not unstable
            ),
            extension,
        ],
        module_loader: Some(Rc::new(TypescriptModuleLoader)),

        ..Default::default()
    })
}

pub async fn execute_with_custom_runtime(command: Subcommand, raw_args: Vec<String>) -> Result<()> {
    let main_module_path = match command {
        Subcommand::Run { main_module, .. } | Subcommand::Destroy { main_module } => main_module,
    };

    let mut js_runtime = create_js_runtime();
    let main_module = resolve_path(&main_module_path, current_dir()?.as_path())?;
    let mod_id = js_runtime.load_main_module(&main_module, None).await?;

    js_runtime.execute_script(
        "name",
        format!(
            r#"globalThis.bootstrap.mainRuntime({})"#,
            json!({
                "args": raw_args,
                "target": env!("TARGET")
            })
        ),
    )?;

    let result_main = js_runtime.mod_evaluate(mod_id);
    // execute main module
    js_runtime.run_event_loop(false).await?;

    // run mashin engine in a side module
    let specifier = deno_core::resolve_url(&format!("file:///__mashin.js")).unwrap();
    let side_mod_id = js_runtime
        .load_side_module(
            &specifier,
            Some(
                r#"
                    if (!globalThis.__mashin) {
                        throw new Error("Mashin engine not initialized")
                    }
                    await globalThis.__mashin.engine.finished();
                "#
                .into(),
            ),
        )
        .await?;

    let result_side = js_runtime.mod_evaluate(side_mod_id);

    js_runtime.run_event_loop(false).await?;

    // execute result on both modules
    result_main.await??;
    result_side.await??;

    Ok(())
}

fn stdlib() -> Vec<OpDecl> {
    let mut ops = vec![];
    ops.extend(builtin::mashin_core_client::op_decls());
    ops
}

// From: https://github.com/denoland/deno/blob/main/core/examples/ts_module_loader.rs
struct TypescriptModuleLoader;

impl TypescriptModuleLoader {
    async fn load_from_deno_std(path: &Url) -> Result<String> {
        println!("Load: {}", path);
        let response = reqwest::get(path.clone()).await?;
        response.text().await.map_err(Into::into)
    }
    async fn load_from_filesystem(path: &Url) -> Result<String> {
        std::fs::read_to_string(
            &path
                .to_file_path()
                .map_err(|_| anyhow!("{path:?}: is not a path"))?,
        )
        .map_err(Into::into)
    }
}

impl ModuleLoader for TypescriptModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _is_main: ResolutionKind,
    ) -> Result<ModuleSpecifier> {
        Ok(resolve_import(specifier, referrer)?)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        async move {
            let media_type = MediaType::from_str(module_specifier.path());
            let (module_type, should_transpile) = match media_type {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (ModuleType::JavaScript, false)
                }
                MediaType::Jsx => (ModuleType::JavaScript, true),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (ModuleType::JavaScript, true),
                MediaType::Json => (ModuleType::Json, false),
                _ => bail!("Unknown extension {:?}", module_specifier),
            };

            let code = match module_specifier.scheme() {
                "file" => TypescriptModuleLoader::load_from_filesystem(&module_specifier).await?,
                "https" => TypescriptModuleLoader::load_from_deno_std(&module_specifier).await?,
                _ => {
                    return Err(anyhow!(
                        "Unsupported module specifier: {}",
                        module_specifier
                    ))
                }
            };
            let code = if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.to_string(),
                    text_info: SourceTextInfo::from_string(code),
                    media_type,
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                })?;
                parsed.transpile(&Default::default())?.text
            } else {
                code
            };
            let module = ModuleSource {
                code: code.into(),
                module_type,
                module_url_specified: module_specifier.to_string(),
                module_url_found: module_specifier.to_string(),
            };
            Ok(module)
        }
        .boxed_local()
    }
}
