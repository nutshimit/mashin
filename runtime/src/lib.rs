use anyhow::{anyhow, bail, Result};
use cache::{get_source_from_bytes, HttpCache, SourceFile};
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
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
use http_client::{fetch_once, FetchOnceArgs, FetchOnceResult, HttpClient};
pub use mashin_core::{
	colors,
	sdk::{ResourceAction, Urn},
	ExecutedResource,
};
use mashin_core::{mashin_dir::MashinDir, BackendState, ExecutedResources, MashinEngine};
use std::{
	cell::RefCell,
	env::current_dir,
	fs,
	future::Future,
	ops::{Deref, DerefMut},
	path::Path,
	pin::Pin,
	rc::Rc,
	sync::Arc,
};

mod builtin;
mod cache;
mod http_client;

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
	mashin_dir: MashinDir,
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
		let mashin_dir = MashinDir::new(None)?;
		let is_first_run = executed_resources.is_none();
		let backend_state = BackendState::new(&mashin_dir)?;
		let http_cache = HttpCache::new(&mashin_dir.deps_folder_path());
		let http_client = HttpClient::new(http_cache, None)?;
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
			module_loader: Some(Rc::new(TypescriptModuleLoader {
				http_client: Arc::new(http_client),
			})),

			..Default::default()
		});

		let mut runtime =
			Self { command, main_module: main_module.to_string(), runtime, raw_args, mashin_dir };

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

#[derive(Debug, Clone)]
struct TypescriptModuleLoader {
	http_client: Arc<HttpClient>,
}

impl TypescriptModuleLoader {
	fn load_from_remote_url(
		&self,
		path: &ModuleSpecifier,
		redirect_limit: i64,
	) -> Pin<Box<dyn Future<Output = Result<SourceFile>> + Send>> {
		match self.http_client.cache().fetch_cached(path, redirect_limit) {
			Ok(Some(file)) => return futures::future::ok(file).boxed(),
			Ok(None) => {},
			Err(err) => return futures::future::err(err).boxed(),
		}
		let http_client = self.http_client.clone();
		let http_cache = http_client.cache().clone();
		let file_fetcher = self.clone();
		let path = path.clone();
		async move {
			match fetch_once(
				&http_client.clone(),
				FetchOnceArgs { url: path.clone(), maybe_accept: None, maybe_etag: None },
			)
			.await?
			{
				FetchOnceResult::NotModified => {
					let file = http_cache.fetch_cached(&path, 10)?.unwrap();
					Ok(file)
				},
				FetchOnceResult::Redirect(redirect_url, headers) => {
					http_cache.set(&path, headers, &[])?;
					file_fetcher.load_from_remote_url(&redirect_url, redirect_limit - 1).await
				},
				FetchOnceResult::Code(bytes, headers) => {
					http_cache.set(&path, headers.clone(), &bytes)?;
					let file = http_cache.build_remote_file(&path, bytes, &headers)?;
					Ok(file)
				},
			}
		}
		.boxed()
	}

	async fn load_from_filesystem(path: &ModuleSpecifier) -> Result<SourceFile> {
		let local = path
			.to_file_path()
			.map_err(|_| uri_error(format!("Invalid file path.\n  Specifier: {path}")))?;
		let bytes = fs::read(&local)?;
		let charset = detect_charset(&bytes).to_string();
		let source = get_source_from_bytes(bytes, Some(charset))?;
		let media_type = MediaType::from_specifier(path);

		Ok(SourceFile {
			local,
			maybe_types: None,
			media_type,
			source: source.into(),
			specifier: path.clone(),
			maybe_headers: None,
		})
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
		let file_fetcher = self.clone();

		async move {
			let source_file = match module_specifier.scheme() {
				"file" => TypescriptModuleLoader::load_from_filesystem(&module_specifier).await?,
				"https" => file_fetcher.load_from_remote_url(&module_specifier, 10).await?,
				_ => return Err(anyhow!("Unsupported module specifier: {}", module_specifier)),
			};

			let (module_type, should_transpile) = match source_file.media_type {
				MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs =>
					(ModuleType::JavaScript, false),
				MediaType::Jsx => (ModuleType::JavaScript, true),
				MediaType::TypeScript |
				MediaType::Mts |
				MediaType::Cts |
				MediaType::Dts |
				MediaType::Dmts |
				MediaType::Dcts |
				MediaType::Tsx => (ModuleType::JavaScript, true),
				MediaType::Json => (ModuleType::Json, false),
				_ => bail!("Unknown extension {:?}", module_specifier),
			};

			let code = if should_transpile {
				let parsed = deno_ast::parse_module(ParseParams {
					specifier: module_specifier.to_string(),
					text_info: SourceTextInfo::from_string(source_file.source.to_string()),
					media_type: source_file.media_type,
					capture_tokens: false,
					scope_analysis: false,
					maybe_syntax: None,
				})?;
				parsed.transpile(&Default::default())?.text
			} else {
				source_file.source.to_string()
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

pub fn detect_charset(bytes: &'_ [u8]) -> &'static str {
	const UTF16_LE_BOM: &[u8] = b"\xFF\xFE";
	const UTF16_BE_BOM: &[u8] = b"\xFE\xFF";

	if bytes.starts_with(UTF16_LE_BOM) {
		"utf-16le"
	} else if bytes.starts_with(UTF16_BE_BOM) {
		"utf-16be"
	} else {
		// Assume everything else is utf-8
		"utf-8"
	}
}
