use clap::Parser;

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

impl Into<mashin_runtime::Subcommand> for RunCmd {
    fn into(self) -> mashin_runtime::Subcommand {
        mashin_runtime::Subcommand::Run {
            main_module: self.main_module,
            dry_run: self.dry_run,
        }
    }
}

impl Into<mashin_runtime::Subcommand> for DestroyCmd {
    fn into(self) -> mashin_runtime::Subcommand {
        mashin_runtime::Subcommand::Destroy {
            main_module: self.main_module,
        }
    }
}
