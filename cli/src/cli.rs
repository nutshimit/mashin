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

use std::{cell::RefCell, rc::Rc, str::FromStr, sync::Arc, time::Instant};

use crate::{
	cache::HttpCache, http_client::HttpClient, module_loader::TypescriptModuleLoader,
	progress_manager::ProgressManager, Result,
};
use clap::Parser;
use console::Emoji;
use dialoguer::Confirm;
use indicatif::HumanDuration;
use mashin_runtime::{
	BackendState, ExecutedResources, MashinBuilder, MashinDir, MashinEngine, Runtime,
	RuntimeCommand,
};
use mashin_sdk::{ResourceAction, Urn};

pub enum Config {}

#[derive(Clone)]
pub enum CacheConfig {}

impl mashin_runtime::Config for Config {
	type ProgressManager = ProgressManager;
	type HttpClient = HttpClient;
}

#[derive(Debug, Parser)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcommand: Subcommand,
}

#[derive(Debug, Parser)]
pub enum Subcommand {
	/// Run a JavaScript or TypeScript program.
	Run(RunCmd),
	/// Destroy all resources in the current state.
	Destroy(DestroyCmd),
}

#[derive(Debug, Parser)]
#[group(skip)]
pub struct RunCmd {
	pub main_module: String,
	#[arg(long, default_value_t = false)]
	pub dry_run: bool,
}

#[derive(Debug, Parser)]
pub struct DestroyCmd {
	pub main_module: String,
}

impl RunCmd {
	pub async fn run(&self, args: Vec<String>) -> Result<()> {
		let started = Instant::now();

		let mashin_dir = MashinDir::new(None)?;
		let backend_state = BackendState::new(&mashin_dir)?;
		let backend = Rc::new(RefCell::new(backend_state));

		let create_runtime = |command, executed_resource, maybe_count, progress_bar| {
			let BuiltEngine { engine, module_loader } = build_engine(
				command,
				progress_bar,
				executed_resource,
				maybe_count,
				backend.clone(),
				mashin_dir.clone(),
			)?;
			Runtime::new(&self.main_module, engine, module_loader, args.clone())
		};

		let mut progress_manager = ProgressManager::new();

		log::info!("    Starting the engine");

		let isolated_pm = progress_manager.clone();
		let total_resources = create_runtime(RuntimeCommand::Prepare, None, None, &isolated_pm)?
			.prepare()
			.await?;

		progress_manager.set_resource_progress(total_resources)?;

		log::info!("    Reading {} resources", total_resources);

		let isolated_pm = progress_manager.clone();
		let runtime_result =
			create_runtime(RuntimeCommand::Read, None, Some(total_resources), &isolated_pm)?
				.run()
				.await?;

		progress_manager.maybe_finish_resource_progress();

		// clone our resource to prevent `BorrowMutError` on the engine on the second run
		let executed_resouces = runtime_result.executed_resources.borrow().clone();

		// FIXME: Move to cli print_diff
		//print_diff(&executed_resouces)?;
		executed_resouces.print_diff_plan();

		if !self.dry_run &&
			!executed_resouces.actions().is_empty() &&
			Confirm::new().with_prompt("\n    Do you want to apply?").interact()?
		{
			progress_manager.set_resource_progress(total_resources)?;
			log::info!("    Applying changes");

			// delete non-present resources that will not receive any hooks
			// probably removed within the client code (TS)
			for (urn, resource) in executed_resouces.iter() {
				if resource.required_change == Some(ResourceAction::Delete) {
					let urn = Urn::from_str(urn)?;
					backend.borrow().delete(&urn)?;
				}
			}

			create_runtime(
				RuntimeCommand::Apply,
				Some(runtime_result.executed_resources),
				Some(total_resources),
				&progress_manager,
			)?
			.run()
			.await?;
		}

		progress_manager.maybe_finish_resource_progress();
		log::info!("{} Done in {}", Emoji("✨ ", "* "), HumanDuration(started.elapsed()));

		Ok(())
	}
}

impl DestroyCmd {
	pub async fn run(&self, _args: Vec<String>) -> Result<()> {
		todo!()
	}
}

pub struct BuiltEngine {
	engine: Rc<MashinEngine<Config>>,
	module_loader: Rc<dyn deno_core::ModuleLoader>,
}

fn build_engine(
	command: RuntimeCommand,
	progress_manager: &ProgressManager,
	executed_resources: Option<Rc<RefCell<ExecutedResources>>>,
	maybe_resources_count: Option<u64>,
	backend: Rc<RefCell<BackendState>>,
	mashin_dir: MashinDir,
) -> Result<BuiltEngine> {
	let http_client = HttpClient::new(
		HttpCache::new(&mashin_dir.deps_folder_path()),
		None,
		true,
		log::Level::Info,
		Some(progress_manager.http_progress.clone()),
	)?;
	let http_client_rc = Rc::new(http_client.clone());
	let module_loader = Rc::new(TypescriptModuleLoader { http_client: Arc::new(http_client) });

	let mashin_engine = MashinBuilder::<Config>::new()
		.with_passphrase(b"mysuperpassword")
		.with_salt(&[
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
			24, 25, 26, 27, 28, 29, 30, 31,
		])
		.with_mashin_dir(mashin_dir)
		.with_state_handler(backend)
		.with_runtime_command(command)
		.with_executed_resources(executed_resources)
		.with_progress_manager(Rc::new(progress_manager.clone()))
		.with_resources_count(maybe_resources_count.unwrap_or_default())
		.with_http_client(http_client_rc)
		.build()?;
	Ok(BuiltEngine { engine: Rc::new(mashin_engine), module_loader })
}
