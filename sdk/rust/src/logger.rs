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

pub struct CliLogger(env_logger::Logger);

impl CliLogger {
	pub fn new(logger: env_logger::Logger) -> Self {
		Self(logger)
	}

	pub fn filter(&self) -> log::LevelFilter {
		self.0.filter()
	}
}

impl log::Log for CliLogger {
	fn enabled(&self, metadata: &log::Metadata) -> bool {
		self.0.enabled(metadata)
	}

	fn log(&self, record: &log::Record) {
		if self.enabled(record.metadata()) {
			self.0.log(record);
		}
	}

	fn flush(&self) {
		self.0.flush();
	}
}
