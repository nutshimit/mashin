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
use console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[derive(Clone, Debug, Default)]
pub struct ProgressManager {
	pub http_progress: MultiProgress,
	pub resource_progress: Option<ProgressBar>,
}

impl ProgressManager {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn maybe_finish_resource_progress(&self) {
		if let Some(resource_progress) = &self.resource_progress {
			resource_progress.finish_and_clear();
		}
	}

	pub fn set_resource_progress(&mut self, len: u64) -> Result<()> {
		let pb = ProgressBar::new(len);
		pb.set_style(
			ProgressStyle::with_template(
				// note that bar size is fixed unlike cargo which is dynamic
				// and also the truncation in cargo uses trailers (`...`)
				if Term::stdout().size().1 > 80 {
					"{spinner:.green} [{elapsed_precise}] [{bar:57}] {pos}/{len} {wide_msg}"
				} else {
					"{spinner:.green} [{elapsed_precise}] [{bar:57}] {pos}/{len}"
				},
			)?
			.progress_chars("#>-"),
		);
		self.resource_progress = Some(pb);
		Ok(())
	}
}

impl mashin_runtime::ProgressManager for ProgressManager {
	fn println(&self, msg: &str) {
		println!("{msg}");
	}

	fn progress_bar(&self) -> Option<indicatif::ProgressBar> {
		self.resource_progress.clone()
	}
}
