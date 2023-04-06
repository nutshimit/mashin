use clap::Parser;
use cli::{Cli, Subcommand};
use mashin_runtime::execute_with_custom_runtime;
use std::env;

mod cli;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();
    let cli = Cli::parse();

    setup_panic_hook();

    match cli.subcommand {
        Subcommand::Run(run) => execute_with_custom_runtime(run.into(), args),
        Subcommand::Destroy(destroy) => execute_with_custom_runtime(destroy.into(), args),
    }
    .await
}

fn setup_panic_hook() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("\n============================================================");
        eprintln!("Mashin has panicked. This is a bug in Mashin. Please report this");
        eprintln!("at https://github.com/nutshimit/mashin/issues/new.");
        eprintln!("If you can reliably reproduce this panic, include the");
        eprintln!("reproduction steps and re-run with the RUST_BACKTRACE=1 env");
        eprintln!("var set and include the backtrace in your report.");
        eprintln!();
        eprintln!("Platform: {} {}", env::consts::OS, env::consts::ARCH);
        eprintln!("Version: {}", env!("CARGO_PKG_VERSION"));
        eprintln!("Args: {:?}", env::args().collect::<Vec<_>>());
        eprintln!();
        orig_hook(panic_info);
        std::process::exit(1);
    }));
}
