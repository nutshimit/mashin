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

use crate::Result;
use clap::Parser;
use dialoguer::Confirm;
use mashin_runtime::Runtime;

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
		let create_runtime = |executed_resource| {
			Runtime::new(
				&self.main_module,
				mashin_runtime::RuntimeCommand::Run,
				args.clone(),
				executed_resource,
			)
		};

		let runtime_result = create_runtime(None)?.run().await?;

		// clone our resource to prevent `BorrowMutError` on the engine on the second run
		let executed_resouces = runtime_result.executed_resources.borrow().clone();

		executed_resouces.print_diff_plan();

		if !self.dry_run && Confirm::new().with_prompt("Do you want to apply?").interact()? {
			println!("Looks like you want to continue");
			create_runtime(Some(runtime_result.executed_resources))?.run().await?;
		}

		Ok(())
	}
}

impl DestroyCmd {
	pub async fn run(&self, args: Vec<String>) -> Result<()> {
		Runtime::new(&self.main_module, mashin_runtime::RuntimeCommand::Destroy, args, None)?
			.run()
			.await?;
		Ok(())
	}
}
