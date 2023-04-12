use anyhow::{anyhow, bail, Result};
pub use colors::*;
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_core::futures::FutureExt;
use deno_core::serde_json::json;
use deno_core::{
    include_js_files, resolve_import, resolve_path, Extension, JsRuntime, ModuleCode, ModuleLoader,
    ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType, OpDecl, OpState, ResolutionKind,
    RuntimeOptions,
};
use deno_fetch::FetchPermissions;
use deno_web::BlobStore;
use deno_web::TimersPermission;
use deno_websocket::WebSocketPermissions;
pub use mashin_core::sdk::{ResourceAction, Urn};
pub use mashin_core::ExecutedResource;
use mashin_core::{BackendState, ExecutedResources, MashinEngine};
use reqwest::Url;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::env::current_dir;
use std::ffi::c_void;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;

mod builtin;
mod colors;

#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: "mashin::runtime",
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

pub struct Runtime {
    runtime: JsRuntime,
    main_module: String,
    command: RuntimeCommand,
    raw_args: Vec<String>,
}

pub struct RuntimeResult {
    pub executed_resources: Rc<RefCell<ExecutedResources>>,
}

pub enum RuntimeCommand {
    Run,
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
    pub fn new(
        main_module: &str,
        command: RuntimeCommand,
        raw_args: Vec<String>,
        executed_resources: Option<Rc<RefCell<ExecutedResources>>>,
    ) -> Result<Self> {
        let is_first_run = executed_resources.is_none();
        let extension = Extension::builder("mashin_core")
            .esm(include_js_files!(
                mashin_core dir "js",
                "01_errors.js",
                "06_util.js",
                "30_os.js",
                "40_ffi.js",
                "98_global_scope.js",
                "99_main.js",
            ))
            .ops(stdlib())
            .state(move |state| {
                // fixme: init in cli?
                let backend = Rc::new(RefCell::new(BackendState::default()));
                state.put(
                    MashinEngine::new(backend, b"mysuperpassword", executed_resources)
                        .expect("test"),
                );

                state.put(AllowAllPermissions {});
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
                extension,
            ],
            module_loader: Some(Rc::new(TypescriptModuleLoader)),

            ..Default::default()
        });

        let mut runtime = Self {
            command,
            main_module: main_module.to_string(),
            runtime,
            raw_args,
        };

        // bootstrap the engine
        runtime.bootstrap(is_first_run)?;
        Ok(runtime)
    }

    pub async fn run(&mut self) -> Result<RuntimeResult> {
        match self.command {
            RuntimeCommand::Run => {
                self.run_main_module().await?;
            }
            RuntimeCommand::Destroy => todo!(),
        };

        let rc_op_state = self.runtime.op_state();
        let op_state = rc_op_state.borrow();
        let engine = op_state.borrow::<MashinEngine>();
        let executed_resources_rc = engine.executed_resources.clone();

        let all_resources_in_state = engine.state_handler.borrow().resources()?;
        let mut executed_resources = executed_resources_rc.borrow_mut();

        for urn in &all_resources_in_state {
            if !executed_resources.contains_key(urn) {
                executed_resources.insert(
                    urn,
                    ExecutedResource {
                        diff: None,
                        provider: urn.as_provider()?,
                        required_change: Some(ResourceAction::Delete),
                    },
                );
            }
        }

        Ok(RuntimeResult {
            executed_resources: executed_resources_rc.clone(),
        })
    }

    // trigger `bootstrapMainRuntime` in `js/99_main.js`
    fn bootstrap(&mut self, is_first_run: bool) -> Result<()> {
        self.runtime.execute_script(
            "file://__bootstrap.js",
            format!(
                r#"globalThis.bootstrap.mainRuntime({})"#,
                json!({
                    "isFirstRun": is_first_run,
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
    async fn run_main_module(&mut self) -> Result<()> {
        let main_module = resolve_path(&self.main_module, current_dir()?.as_path())?;
        let mod_id = self.runtime.load_main_module(&main_module, None).await?;
        let result_main = self.runtime.mod_evaluate(mod_id);
        self.runtime.run_event_loop(false).await?;
        result_main.await?
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
