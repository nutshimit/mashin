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
use anyhow::anyhow;
use deno_ast::MediaType;
use deno_core::ModuleSpecifier;
use encoding_rs::Encoding;
pub use http_cache::HttpCache;
use std::{
	borrow::Cow,
	collections::HashMap,
	fs::OpenOptions,
	io::{Error, ErrorKind, Write},
	path::{Path, PathBuf},
	sync::Arc,
};

mod http_cache;

/// A structure representing a source file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SourceFile {
	/// The path to the local version of the source file.  For local files this
	/// will be the direct path to that file.  For remote files, it will be the
	/// path to the file in the HTTP cache.
	pub local: PathBuf,
	/// For remote files, if there was an `X-TypeScript-Type` header, the parsed
	/// out value of that header.
	pub maybe_types: Option<String>,
	/// The resolved media type for the file.
	pub media_type: MediaType,
	/// The source of the file as a string.
	pub source: Arc<str>,
	/// The _final_ specifier for the file.  The requested specifier and the final
	/// specifier maybe different for remote files that have been redirected.
	pub specifier: ModuleSpecifier,

	pub maybe_headers: Option<HashMap<String, String>>,
}

/// Permissions used to save a file in the disk caches.
pub const CACHE_PERM: u32 = 0o644;

pub fn atomic_write_file<T: AsRef<[u8]>>(
	filename: &Path,
	data: T,
	mode: u32,
) -> std::io::Result<()> {
	let rand: String = (0..4).map(|_| format!("{:02x}", rand::random::<u8>())).collect();
	let extension = format!("{rand}.tmp");
	let tmp_file = filename.with_extension(extension);
	write_file(&tmp_file, data, mode)?;
	std::fs::rename(tmp_file, filename)?;
	Ok(())
}

pub fn write_file<T: AsRef<[u8]>>(filename: &Path, data: T, mode: u32) -> std::io::Result<()> {
	write_file_2(filename, data, true, mode, true, false)
}

pub fn write_file_2<T: AsRef<[u8]>>(
	filename: &Path,
	data: T,
	update_mode: bool,
	mode: u32,
	is_create: bool,
	is_append: bool,
) -> std::io::Result<()> {
	let mut file = OpenOptions::new()
		.read(false)
		.write(true)
		.append(is_append)
		.truncate(!is_append)
		.create(is_create)
		.open(filename)?;

	if update_mode {
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let mode = mode & 0o777;
			let permissions = PermissionsExt::from_mode(mode);
			file.set_permissions(permissions)?;
		}
		#[cfg(not(unix))]
		let _ = mode;
	}

	file.write_all(data.as_ref())
}

pub fn get_source_from_bytes(bytes: Vec<u8>, maybe_charset: Option<String>) -> Result<String> {
	let source = if let Some(charset) = maybe_charset {
		convert_to_utf8(&bytes, &charset)?.to_string()
	} else {
		String::from_utf8(bytes)?
	};

	Ok(source)
}

fn convert_to_utf8<'a>(bytes: &'a [u8], charset: &'_ str) -> Result<Cow<'a, str>> {
	match Encoding::for_label(charset.as_bytes()) {
		Some(encoding) => encoding
			.decode_without_bom_handling_and_without_replacement(bytes)
			.ok_or_else(|| anyhow!("invalid data")),
		None =>
			Err(Error::new(ErrorKind::InvalidInput, format!("Unsupported charset: {charset}"))
				.into()),
	}
}
