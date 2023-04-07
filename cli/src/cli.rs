use crate::Result;
use clap::Parser;
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
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Parser)]
pub struct DestroyCmd {
    pub main_module: String,
}

impl RunCmd {
    pub async fn run(&self, args: Vec<String>) -> Result<()> {
        Runtime::new(
            &self.main_module,
            mashin_runtime::RuntimeCommand::Run {
                dry_run: self.dry_run,
            },
            args,
        )
        .run()
        .await
    }
}

impl DestroyCmd {
    pub async fn run(&self, args: Vec<String>) -> Result<()> {
        Runtime::new(
            &self.main_module,
            mashin_runtime::RuntimeCommand::Destroy {},
            args,
        )
        .run()
        .await
    }
}
