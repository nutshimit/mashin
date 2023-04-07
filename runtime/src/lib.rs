use anyhow::{anyhow, bail, Result};
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_core::error::AnyError;
use deno_core::futures::channel::oneshot::Receiver;
use deno_core::futures::FutureExt;
use deno_core::serde_json::json;
use deno_core::{
    include_js_files, resolve_import, resolve_path, Extension, JsRuntime, ModuleCode, ModuleLoader,
    ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType, OpDecl, ResolutionKind,
    RuntimeOptions,
};
use deno_fetch::FetchPermissions;
use deno_web::BlobStore;
use deno_web::TimersPermission;
use deno_websocket::WebSocketPermissions;
use reqwest::Url;
use std::env::current_dir;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
mod builtin;

#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: "mashin::core",
            $patter $(, $values)*
		)
	};
}

#[macro_export]
macro_rules! js_log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: "mashin::js",
            $patter $(, $values)*
		)
	};
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

pub struct Runtime {
    runtime: JsRuntime,
    main_module: String,
    command: RuntimeCommand,
    raw_args: Vec<String>,
}

pub enum RuntimeCommand {
    Run { dry_run: bool },
    Destroy,
}

impl Deref for Runtime {
    type Target = JsRuntime;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

impl DerefMut for Runtime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime
    }
}

impl Runtime {
    pub fn new(main_module: &str, command: RuntimeCommand, raw_args: Vec<String>) -> Self {
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

        let runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![
                deno_console::deno_console::init_ops_and_esm(),
                deno_webidl::deno_webidl::init_ops_and_esm(),
                deno_url::deno_url::init_ops_and_esm(),
                deno_web::deno_web::init_ops_and_esm::<AllowAllPermissions>(
                    BlobStore::default(),
                    None,
                ),
                deno_fetch::deno_fetch::init_ops_and_esm::<AllowAllPermissions>(
                    deno_fetch::Options {
                        user_agent: format!("mashin_core/{}", env!("CARGO_PKG_VERSION")),
                        ..Default::default()
                    },
                ),
                mashin_ffi::mashin_ffi::init_ops_and_esm::<AllowAllPermissions>(
                    true, // not unstable
                ),
                extension,
            ],
            module_loader: Some(Rc::new(TypescriptModuleLoader)),

            ..Default::default()
        });

        Self {
            command,
            main_module: main_module.to_string(),
            runtime,
            raw_args,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // bootstrap the engine
        self.bootstrap()?;

        // dry run, get state of all resources
        // from each providers and save into the gothamstate
        self.dry_run().await?;

        // compare with current local state
        // and apply pending changes
        self.apply().await
    }

    // trigger `bootstrapMainRuntime` in `js/99_main.js`
    fn bootstrap(&mut self) -> Result<()> {
        self.runtime.execute_script(
            "file:///__bootstrap.js",
            format!(
                r#"globalThis.bootstrap.mainRuntime({})"#,
                json!({
                    // allow parsing env with Deno.args
                    "args": self.raw_args,
                    // allow target with Deno.env
                    "target": env!("TARGET")
                })
            ),
        )?;
        Ok(())
    }

    // run the main module, evaluating each resource
    async fn dry_run(&mut self) -> Result<()> {
        let main_module = resolve_path(&self.main_module, current_dir()?.as_path())?;
        let mod_id = self.runtime.load_main_module(&main_module, None).await?;
        let result_main = self.runtime.mod_evaluate(mod_id);
        self.runtime.run_event_loop(false).await?;
        result_main.await?
    }

    // trigger the `as__client_apply` ops
    async fn apply(&mut self) -> Result<()> {
        let specifier = deno_core::resolve_url(&format!("file:///__apply.js"))?;
        let mod_id = self
            .runtime
            .load_side_module(
                &specifier,
                Some(
                    r#"
                    if (!globalThis.__mashin) {
                        throw new Error("Mashin engine not initialized")
                    }
                    await globalThis.__mashin.engine.apply();
                    "#
                    .into(),
                ),
            )
            .await?;

        let mod_evaluate = self.runtime.mod_evaluate(mod_id);
        self.runtime.run_event_loop(false).await?;
        mod_evaluate.await??;

        Ok(())
    }
}

fn stdlib() -> Vec<OpDecl> {
    let mut ops = vec![];
    ops.extend(builtin::mashin_core_client::op_decls());
    ops
}

// From: https://github.com/denoland/deno/blob/main/core/examples/ts_module_loader.rs
struct TypescriptModuleLoader;

impl TypescriptModuleLoader {
    async fn load_from_remote_url(path: &Url) -> Result<String> {
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
                "https" => TypescriptModuleLoader::load_from_remote_url(&module_specifier).await?,
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
