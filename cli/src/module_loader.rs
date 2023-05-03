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

use crate::{
	cache::{get_source_from_bytes, SourceFile},
	http_client::{fetch_once, FetchOnceArgs, FetchOnceResult, HttpClient},
	Result,
};
use anyhow::{anyhow, bail};
use console::style;
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_core::{
	error::uri_error,
	futures::{self, FutureExt},
	resolve_import, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType,
	ResolutionKind,
};
use mashin_runtime::HttpCache as _;
use std::{fs, future::Future, pin::Pin, sync::Arc};

#[derive(Debug, Clone)]
pub struct TypescriptModuleLoader {
	pub http_client: Arc<HttpClient>,
}

impl TypescriptModuleLoader {
	pub(crate) fn load_from_remote_url(
		&self,
		path: &ModuleSpecifier,
		redirect_limit: i64,
	) -> Pin<Box<dyn Future<Output = Result<SourceFile>> + Send>> {
		let http_client = self.http_client.clone();
		let http_cache = http_client.http_cache.clone();
		let module_loader = self.clone();
		let path = path.clone();

		match http_cache.fetch_cached(&path, redirect_limit) {
			Ok(Some(file)) => return futures::future::ok(file).boxed(),
			Ok(None) => {},
			Err(err) => return futures::future::err(err).boxed(),
		}
		let mut multi_progress = None;
		if let Some(mp) = http_client.progress_bar.as_ref() {
			multi_progress = Some(mp.clone());
		} else {
			log::log!(http_client.download_log_level, "{} {}", style("Download").green(), path);
		}
		async move {
			match fetch_once(
				&http_client.clone(),
				FetchOnceArgs {
					url: path.clone(),
					maybe_accept: None,
					maybe_etag: None,
					multi_progress,
				},
			)
			.await?
			{
				FetchOnceResult::NotModified =>
					http_cache.fetch_cached(&path, 10)?.ok_or(anyhow!("nuable to fetch cache")),
				FetchOnceResult::Redirect(redirect_url, headers) => {
					http_cache.set(&path, headers, &[])?;
					module_loader.load_from_remote_url(&redirect_url, redirect_limit - 1).await
				},
				FetchOnceResult::Code(bytes, headers) => {
					http_cache.set(&path, headers.clone(), &bytes)?;
					http_cache.build_remote_file(&path, bytes, &headers)
				},
			}
		}
		.boxed()
	}

	pub(crate) async fn load_from_filesystem(path: &ModuleSpecifier) -> Result<SourceFile> {
		let local = path
			.to_file_path()
			.map_err(|_| uri_error(format!("Invalid file path.\n  Specifier: {path}")))?;
		let bytes = fs::read(&local)?;
		let charset = detect_charset(&bytes).to_string();
		let source = get_source_from_bytes(bytes, Some(charset))?;
		let media_type = MediaType::from_specifier(path);

		Ok(SourceFile {
			local,
			maybe_types: None,
			media_type,
			source: source.into(),
			specifier: path.clone(),
			maybe_headers: None,
		})
	}
}

impl ModuleLoader for TypescriptModuleLoader {
	fn resolve(
		&self,
		specifier: &str,
		referrer: &str,
		_is_main: ResolutionKind,
	) -> Result<ModuleSpecifier> {
		Ok(resolve_import(specifier, referrer)?)
	}

	fn load(
		&self,
		module_specifier: &ModuleSpecifier,
		_maybe_referrer: Option<ModuleSpecifier>,
		_is_dyn_import: bool,
	) -> Pin<Box<ModuleSourceFuture>> {
		let module_specifier = module_specifier.clone();
		let file_fetcher = self.clone();

		async move {
			let source_file = match module_specifier.scheme() {
				"file" => TypescriptModuleLoader::load_from_filesystem(&module_specifier).await?,
				"https" => file_fetcher.load_from_remote_url(&module_specifier, 10).await?,
				_ => return Err(anyhow!("Unsupported module specifier: {}", module_specifier)),
			};

			let (module_type, should_transpile) = match source_file.media_type {
				MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs =>
					(ModuleType::JavaScript, false),
				MediaType::Jsx => (ModuleType::JavaScript, true),
				MediaType::TypeScript |
				MediaType::Mts |
				MediaType::Cts |
				MediaType::Dts |
				MediaType::Dmts |
				MediaType::Dcts |
				MediaType::Tsx => (ModuleType::JavaScript, true),
				MediaType::Json => (ModuleType::Json, false),
				_ => bail!("Unknown extension {:?}", module_specifier),
			};

			let code = if should_transpile {
				let parsed = deno_ast::parse_module(ParseParams {
					specifier: module_specifier.to_string(),
					text_info: SourceTextInfo::from_string(source_file.source.to_string()),
					media_type: source_file.media_type,
					capture_tokens: false,
					scope_analysis: false,
					maybe_syntax: None,
				})?;
				parsed.transpile(&Default::default())?.text
			} else {
				source_file.source.to_string()
			};
			let module = ModuleSource {
				code: code.into(),
				module_type,
				module_url_specified: module_specifier.to_string(),
				module_url_found: module_specifier.to_string(),
			};
			Ok(module)
		}
		.boxed_local()
	}
}

pub fn detect_charset(bytes: &'_ [u8]) -> &'static str {
	const UTF16_LE_BOM: &[u8] = b"\xFF\xFE";
	const UTF16_BE_BOM: &[u8] = b"\xFE\xFF";

	if bytes.starts_with(UTF16_LE_BOM) {
		"utf-16le"
	} else if bytes.starts_with(UTF16_BE_BOM) {
		"utf-16be"
	} else {
		// Assume everything else is utf-8
		"utf-8"
	}
}
