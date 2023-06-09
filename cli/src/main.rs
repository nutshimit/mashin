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
 *  This file is licensed as MIT. See LICENSE for details.  *
 *                                                          *
\* ---------------------------------------------------------*/

pub(crate) use anyhow::Result;
use clap::Parser;
use cli::{Cli, Subcommand};
use std::env;

mod cache;
mod cli;
mod http_client;
mod logger;
mod module_loader;
mod progress_manager;
mod tools;
mod util;
mod version;

const MASHIN: &str = r#"
                                     ███╗░░░███╗░█████╗░░██████╗██╗░░██╗██╗███╗░░██╗
                                     ████╗░████║██╔══██╗██╔════╝██║░░██║██║████╗░██║
                                     ██╔████╔██║███████║╚█████╗░███████║██║██╔██╗██║
                                     ██║╚██╔╝██║██╔══██║░╚═══██╗██╔══██║██║██║╚████║
                                     ██║░╚═╝░██║██║░░██║██████╔╝██║░░██║██║██║░╚███║
                                     ╚═╝░░░░░╚═╝╚═╝░░╚═╝╚═════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝
                                                                        by Nutshimit
"#;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), anyhow::Error> {
	setup_panic_hook();
	let args: Vec<String> = env::args().collect();
	let cli = Cli::parse();

	logger::init();

	match cli.subcommand {
		Subcommand::Bindgen(cmd) => cmd.run().await,
		Subcommand::Doc(cmd) => cmd.run().await,
		Subcommand::Destroy(cmd) => cmd.run(args).await,
		Subcommand::Run(cmd) => cmd.run(args).await,
		Subcommand::Upgrade(cmd) => cmd.run().await,
		Subcommand::Version(cmd) => cmd.run().await,
	}
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
