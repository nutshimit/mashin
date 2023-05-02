use std::env::current_dir;

pub use anyhow::Result;
use clap::Parser;
use cli::Cli;
use glue::write_mod;

mod cli;
mod glue;
mod ts;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.subcommand {
		cli::Subcommand::Ts(ts) => {
			let glue = glue::get_glue(ts.bindings)?;
			let output = ts::generate_ts(&glue).await?;
			let out_path =
				ts.out.unwrap_or(current_dir()?.to_str().unwrap_or_default().to_string());

			write_mod(out_path, output)?;
		},
	}

	Ok(())
}
