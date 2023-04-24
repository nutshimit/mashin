/* -------------------------------------------------------- *\
 *                                                          *
 *      ███╗░░░███╗░█████╗░░██████╗██╗░░██╗██╗███╗░░██╗     *
 *      ████╗░████║██╔══██╗██╔════╝██║░░██║██║████╗░██║     *
 *      ██╔████╔██║███████║╚█████╗░███████║██║██╔██╗██║     *
 *      ██║╚██╔╝██║██╔══██║░╚═══██╗██╔══██║██║██║╚████║     *
 *      ██║░╚═╝░██║██║░░██║██████╔╝██║░░██║██║██║░╚███║     *
 *      ╚═╝░░░░░╚═╝╚═╝░░╚═╝╚═════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝     *
 *                                         by Nutshimit     *
 * -------------------------------------------------------- *
 *                                                          *
 *   This file is dual-licensed as Apache-2.0 or GPL-3.0.   *
 *   see LICENSE for license details.                       *
 *                                                          *
\* ---------------------------------------------------------*/

use anyhow::Result;
use deno_core::{
	include_js_files, resolve_path, serde_json::json, Extension, JsRuntime, ModuleLoader,
	ModuleSpecifier, OpDecl, RuntimeOptions,
};
use deno_fetch::FetchPermissions;
use deno_web::{BlobStore, TimersPermission};
use deno_websocket::WebSocketPermissions;
use mashin_core::sdk::ResourceAction;
pub use mashin_core::{
	mashin_dir::MashinDir, BackendState, Config, ExecutedResource, ExecutedResources, HeadersMap,
	HttpCache, HttpClient, MashinBuilder, MashinEngine, ProgressManager, RuntimeCommand,
};
use std::{
	cell::RefCell,
	env::current_dir,
	ops::{Deref, DerefMut},
	path::Path,
	rc::Rc,
	str::FromStr,
};

mod builtin;

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

pub struct Runtime<T: Config> {
	runtime: JsRuntime,
	main_module: String,
	engine: Rc<MashinEngine<T>>,
	raw_args: Vec<String>,
}

pub struct RuntimeResult {
	pub executed_resources: Rc<RefCell<ExecutedResources>>,
}

impl<T: Config> Deref for Runtime<T> {
	type Target = JsRuntime;

	fn deref(&self) -> &Self::Target {
		&self.runtime
	}
}

impl<T: Config> DerefMut for Runtime<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.runtime
	}
}

impl<T: Config> Runtime<T> {
	pub fn new(
		main_module: &str,
		mashin_engine: Rc<MashinEngine<T>>,
		module_loader: Rc<dyn ModuleLoader>,
		raw_args: Vec<String>,
	) -> Result<Self> {
		let isolated_engine = mashin_engine.clone();
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
			.ops(stdlib::<T>())
			.state(move |state| {
				// fixme: init in cli?
				state.put(isolated_engine);
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
			module_loader: Some(module_loader),
			..Default::default()
		});

		let mut runtime =
			Self { engine: mashin_engine, main_module: main_module.to_string(), runtime, raw_args };

		// bootstrap the engine
		runtime.bootstrap()?;
		Ok(runtime)
	}

	// do a complete dry run to prepare all resources
	// and mainly count how many we have total
	pub async fn prepare(&mut self) -> Result<u64> {
		self.run_main_module().await?;
		let rc_op_state = self.runtime.op_state();
		let op_state = rc_op_state.borrow();
		let engine = op_state.borrow::<Rc<MashinEngine<T>>>();
		Ok(engine.resources_count())
	}

	pub async fn run(&mut self) -> Result<RuntimeResult> {
		self.run_main_module().await?;

		let rc_op_state = self.runtime.op_state();
		let op_state = rc_op_state.borrow();
		let engine = op_state.borrow::<Rc<MashinEngine<T>>>();
		let executed_resources_rc = &engine.executed_resources;
		let all_resources_in_state = engine.state_handler.borrow().resources()?;
		let mut executed_resources = executed_resources_rc.borrow_mut();

		// add all missing ressource to be deleted
		// they are available within the state but not in the code
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

		Ok(RuntimeResult { executed_resources: executed_resources_rc.clone() })
	}

	// trigger `bootstrapMainRuntime` in `js/99_main.js`
	fn bootstrap(&mut self) -> Result<()> {
		self.runtime.execute_script(
			"file://__bootstrap.js",
			format!(
				r#"globalThis.bootstrap.mainRuntime({})"#,
				json!({
					// display console.log() only on read
					"isFirstRun": self.engine.command == RuntimeCommand::Read,
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
		let main_module_path = &self.main_module;
		let main_module = if main_module_path.starts_with("https") {
			ModuleSpecifier::from_str(main_module_path)?
		} else {
			resolve_path(&self.main_module, current_dir()?.as_path())?
		};
		if self.engine.command == RuntimeCommand::Prepare {
			log::info!("    Fetching dependencies");
		}
		let mod_id = self.runtime.load_main_module(&main_module, None).await?;
		let result_main = self.runtime.mod_evaluate(mod_id);
		self.runtime.run_event_loop(false).await?;
		result_main.await?
	}
}

fn stdlib<T: Config>() -> Vec<OpDecl> {
	let mut ops = vec![];
	ops.extend(builtin::mashin_core_client::op_decls::<T>());
	ops
}
