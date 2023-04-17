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

use super::{atomic_write_file, get_source_from_bytes, SourceFile, CACHE_PERM};
use crate::Result;
use deno_ast::{MediaType, ModuleSpecifier};
use deno_core::{
	error::{custom_error, generic_error},
	serde_json,
};
use reqwest::Url;
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	fs,
	fs::File,
	io,
	io::Read,
	path::{Path, PathBuf},
	time::SystemTime,
};

pub type HeadersMap = HashMap<String, String>;

/// Turn base of url (scheme, hostname, port) into a valid filename.
/// This method replaces port part with a special string token (because
/// ":" cannot be used in filename on some platforms).
/// Ex: $DENO_DIR/deps/https/deno.land/
fn base_url_to_filename(url: &Url) -> Option<PathBuf> {
	let mut out = PathBuf::new();

	let scheme = url.scheme();
	out.push(scheme);

	match scheme {
		"http" | "https" => {
			let host = url.host_str().unwrap();
			let host_port = match url.port() {
				Some(port) => format!("{host}_PORT{port}"),
				None => host.to_string(),
			};
			out.push(host_port);
		},
		"data" | "blob" => (),
		scheme => {
			log::debug!("Don't know how to create cache name for scheme: {}", scheme);
			return None
		},
	};

	Some(out)
}

/// Turn provided `url` into a hashed filename.
/// URLs can contain a lot of characters that cannot be used
/// in filenames (like "?", "#", ":"), so in order to cache
/// them properly they are deterministically hashed into ASCII
/// strings.
///
/// NOTE: this method is `pub` because it's used in integration_tests
pub fn url_to_filename(url: &Url) -> Option<PathBuf> {
	let mut cache_filename = base_url_to_filename(url)?;

	let mut rest_str = url.path().to_string();
	if let Some(query) = url.query() {
		rest_str.push('?');
		rest_str.push_str(query);
	}
	// NOTE: fragment is omitted on purpose - it's not taken into
	// account when caching - it denotes parts of webpage, which
	// in case of static resources doesn't make much sense
	let hashed_filename = checksum(&[rest_str.as_bytes()]);
	cache_filename.push(hashed_filename);
	Some(cache_filename)
}

pub fn checksum(v: &[impl AsRef<[u8]>]) -> String {
	let mut ctx = Context::new(&SHA256);
	for src in v {
		ctx.update(src.as_ref());
	}
	let digest = ctx.finish();
	let out: Vec<String> = digest.as_ref().iter().map(|byte| format!("{byte:02x}")).collect();
	out.join("")
}

#[derive(Debug, Clone)]
pub struct HttpCache {
	pub location: PathBuf,
}

impl HttpCache {
	/// Returns a new instance.
	///
	/// `location` must be an absolute path.
	pub fn new(location: &Path) -> Self {
		assert!(location.is_absolute());
		Self { location: location.to_owned() }
	}

	/// Ensures the location of the cache.
	pub fn ensure_dir_exists(&self, path: &Path) -> io::Result<()> {
		if path.is_dir() {
			return Ok(())
		}
		fs::create_dir_all(path).map_err(|e| {
      io::Error::new(
        e.kind(),
        format!(
          "Could not create remote modules cache location: {path:?}\nCheck the permission of the directory."
        ),
      )
    })
	}

	pub fn get_cache_filename(&self, url: &Url) -> Option<PathBuf> {
		Some(self.location.join(url_to_filename(url)?))
	}

	pub fn get(&self, url: &Url) -> Result<(File, HeadersMap, SystemTime)> {
		let cache_filename = self.location.join(
			url_to_filename(url).ok_or_else(|| generic_error("Can't convert url to filename."))?,
		);
		let metadata_filename = CachedUrlMetadata::filename(&cache_filename);
		let file = File::open(cache_filename)?;
		let metadata = fs::read_to_string(metadata_filename)?;
		let metadata: CachedUrlMetadata = serde_json::from_str(&metadata)?;
		Ok((file, metadata.headers, metadata.now))
	}

	pub fn set(&self, url: &Url, headers_map: HeadersMap, content: &[u8]) -> Result<PathBuf> {
		let cache_filename = self.location.join(
			url_to_filename(url).ok_or_else(|| generic_error("Can't convert url to filename."))?,
		);
		// Create parent directory
		let parent_filename =
			cache_filename.parent().expect("Cache filename should have a parent dir");
		self.ensure_dir_exists(parent_filename)?;
		// Cache content
		atomic_write_file(&cache_filename, content, CACHE_PERM)?;

		let metadata = CachedUrlMetadata {
			now: SystemTime::now(),
			url: url.to_string(),
			headers: headers_map,
		};
		metadata.write(&cache_filename)?;

		Ok(cache_filename)
	}

	pub fn fetch_cached(
		&self,
		specifier: &ModuleSpecifier,
		redirect_limit: i64,
	) -> Result<Option<SourceFile>> {
		if redirect_limit < 0 {
			return Err(custom_error("Http", "Too many redirects."))
		}

		let (mut source_file, headers, _) = match self.get(specifier) {
			Err(err) => {
				if let Some(err) = err.downcast_ref::<std::io::Error>() {
					if err.kind() == std::io::ErrorKind::NotFound {
						return Ok(None)
					}
				}
				return Err(err)
			},
			Ok(cache) => cache,
		};
		if let Some(redirect_to) = headers.get("location") {
			let redirect = deno_core::resolve_import(redirect_to, specifier.as_str())?;
			return self.fetch_cached(&redirect, redirect_limit - 1)
		}
		let mut bytes = Vec::new();
		source_file.read_to_end(&mut bytes)?;
		let file = self.build_remote_file(specifier, bytes, &headers)?;

		Ok(Some(file))
	}

	pub fn fetch_cached_path(
		&self,
		specifier: &ModuleSpecifier,
		redirect_limit: i64,
	) -> Result<Option<PathBuf>> {
		if redirect_limit < 0 {
			return Err(custom_error("Http", "Too many redirects."))
		}

		if let Some(cache_filename) = self.get_cache_filename(specifier) {
			if cache_filename.exists() {
				let metadata = CachedUrlMetadata::read(&cache_filename)?;
				if let Some(redirect_to) = metadata.headers.get("location") {
					let redirect = deno_core::resolve_import(redirect_to, specifier.as_str())?;
					return self.fetch_cached_path(&redirect, redirect_limit - 1)
				}

				return Ok(Some(cache_filename))
			}
		}

		Ok(None)
	}

	pub fn build_remote_file(
		&self,
		specifier: &ModuleSpecifier,
		bytes: Vec<u8>,
		headers: &HashMap<String, String>,
	) -> Result<SourceFile> {
		let local = self
			.get_cache_filename(specifier)
			.ok_or_else(|| generic_error("Cannot convert specifier to cached filename."))?;
		let maybe_content_type = headers.get("content-type");
		let (media_type, maybe_charset) = map_content_type(specifier, maybe_content_type);
		let source = get_source_from_bytes(bytes, maybe_charset)?;
		let maybe_types = match media_type {
			MediaType::JavaScript | MediaType::Cjs | MediaType::Mjs | MediaType::Jsx =>
				headers.get("x-typescript-types").cloned(),
			_ => None,
		};

		Ok(SourceFile {
			local,
			maybe_types,
			media_type,
			source: source.into(),
			specifier: specifier.clone(),
			maybe_headers: Some(headers.clone()),
		})
	}
}

#[derive(Serialize, Deserialize)]
pub struct CachedUrlMetadata {
	pub headers: HeadersMap,
	pub url: String,
	#[serde(default = "SystemTime::now")]
	pub now: SystemTime,
}

impl CachedUrlMetadata {
	pub fn write(&self, cache_filename: &Path) -> Result<()> {
		let metadata_filename = Self::filename(cache_filename);
		let json = serde_json::to_string_pretty(self)?;
		atomic_write_file(&metadata_filename, json, CACHE_PERM)?;
		Ok(())
	}

	pub fn read(cache_filename: &Path) -> Result<Self> {
		let metadata_filename = Self::filename(cache_filename);
		let metadata = fs::read_to_string(metadata_filename)?;
		let metadata: Self = serde_json::from_str(&metadata)?;
		Ok(metadata)
	}

	pub fn filename(cache_filename: &Path) -> PathBuf {
		cache_filename.with_extension("metadata.json")
	}
}

pub fn map_content_type(
	specifier: &ModuleSpecifier,
	maybe_content_type: Option<&String>,
) -> (MediaType, Option<String>) {
	if let Some(content_type) = maybe_content_type {
		let mut content_types = content_type.split(';');
		let content_type = content_types.next().unwrap();
		let media_type = MediaType::from_content_type(specifier, content_type);
		let charset = content_types
			.map(str::trim)
			.find_map(|s| s.strip_prefix("charset="))
			.map(String::from);

		(media_type, charset)
	} else {
		(MediaType::from_specifier(specifier), None)
	}
}
