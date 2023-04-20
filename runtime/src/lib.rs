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

use anyhow::{anyhow, bail, Result};
use deno_core::{
	error::uri_error,
	futures::{self, FutureExt},
	include_js_files, resolve_import, resolve_path,
	serde_json::json,
	Extension, JsRuntime, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier,
	ModuleType, OpDecl, ResolutionKind, RuntimeOptions,
};
use deno_fetch::FetchPermissions;
use deno_web::{BlobStore, TimersPermission};
use deno_websocket::WebSocketPermissions;
pub use mashin_core::{colors, mashin_dir::MashinDir};
use mashin_core::{
	sdk::ResourceAction, BackendState, ExecutedResource, ExecutedResources, MashinEngine,
};
use std::{
	cell::RefCell,
	env::current_dir,
	ops::{Deref, DerefMut},
	path::Path,
	rc::Rc,
	sync::Arc,
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
		http_client: Rc<dyn mashin_core::sdk::HttpClient>,
		module_loader: Rc<dyn ModuleLoader>,
		mashin_dir: &MashinDir,
	) -> Result<Self> {
		let is_first_run = executed_resources.is_none();
		let backend_state = BackendState::new(&mashin_dir)?;
		let isolated_mashin_dir = mashin_dir.clone();
		let isolated_http_client = http_client.clone();

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
				let backend = Rc::new(RefCell::new(backend_state));
				state.put(
					MashinEngine::new(backend, b"mysuperpassword", executed_resources)
						.expect("valid engine"),
				);
				state.put(isolated_mashin_dir);
				state.put(isolated_http_client);
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

		let mut runtime = Self { command, main_module: main_module.to_string(), runtime, raw_args };

		// bootstrap the engine
		runtime.bootstrap(is_first_run)?;
		Ok(runtime)
	}

	pub async fn run(&mut self) -> Result<RuntimeResult> {
		match self.command {
			RuntimeCommand::Run => {
				self.run_main_module().await?;
			},
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

		Ok(RuntimeResult { executed_resources: executed_resources_rc.clone() })
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
