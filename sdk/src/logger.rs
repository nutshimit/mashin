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

/// `CliLogger` is a wrapper around the `env_logger::Logger` that provides a convenient way to
/// pass a single logger instance to providers, facilitating log output to the console and
/// improving traceability.
///
/// The `CliLogger` is automatically injected into Providers and Resources by the Mashin macros,
/// making it easy to use and ensuring consistent logging behavior across the system.
pub struct CliLogger(env_logger::Logger);

impl CliLogger {
	/// Constructs a new `CliLogger` from the given `env_logger::Logger`.
	///
	/// ### Arguments
	///
	/// * `logger` - The `env_logger::Logger` instance to be wrapped.
	pub fn new(logger: env_logger::Logger) -> Self {
		Self(logger)
	}

	/// Retrieves the current log level filter of the `CliLogger`.
	///
	/// ### Returns
	///
	/// The `log::LevelFilter` representing the current log level filter.
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
