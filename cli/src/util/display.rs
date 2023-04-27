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
 *   see LICENSE-* for license details.                     *
 *                                                          *
\* ---------------------------------------------------------*/

use deno_core::{error::AnyError, serde_json};
use std::io::Write;

pub fn write_to_stdout_ignore_sigpipe(bytes: &[u8]) -> Result<(), std::io::Error> {
	use std::io::ErrorKind;

	match std::io::stdout().write_all(bytes) {
		Ok(()) => Ok(()),
		Err(e) => match e.kind() {
			ErrorKind::BrokenPipe => Ok(()),
			_ => Err(e),
		},
	}
}

#[allow(dead_code)]
pub fn write_json_to_stdout<T>(value: &T) -> Result<(), AnyError>
where
	T: ?Sized + serde::ser::Serialize,
{
	let mut writer = std::io::BufWriter::new(std::io::stdout());
	serde_json::to_writer_pretty(&mut writer, value)?;
	writeln!(&mut writer)?;
	Ok(())
}
