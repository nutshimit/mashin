pub(crate) use anyhow::Result;
use clap::Parser;
use cli::{Cli, Subcommand};
use std::env;

mod cli;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();
    let cli = Cli::parse();

    setup_log_output();
    setup_panic_hook();

    match cli.subcommand {
        Subcommand::Run(cmd) => cmd.run(args).await,
        Subcommand::Destroy(cmd) => cmd.run(args).await,
    }
}

fn setup_log_output() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("rustyline", log::LevelFilter::Off)
        // FIXME: We should look why they throw lot of output
        .filter_module("swc_ecma_codegen", log::LevelFilter::Off)
        .filter_module("swc_ecma_transforms_base", log::LevelFilter::Error)
        // wgpu crates (gfx_backend), have a lot of useless INFO and WARN logs
        .filter_module("wgpu", log::LevelFilter::Error)
        .filter_module("gfx", log::LevelFilter::Error)
        // used to make available the lsp_debug which is then filtered out at runtime
        // in the cli logger
        .filter_module("deno::lsp::performance", log::LevelFilter::Debug)
        .init();
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
