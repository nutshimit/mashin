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

use crate::{cache::HttpCache, Result};
use anyhow::bail;
use deno_core::error::{custom_error, generic_error};
use deno_fetch::create_http_client;
use reqwest::{
	header::{HeaderValue, ACCEPT, IF_NONE_MATCH, LOCATION},
	Response, StatusCode, Url,
};
use std::collections::HashMap;

pub type HeadersMap = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct HttpClient {
	client: reqwest::Client,
	http_cache: HttpCache,
}

impl HttpClient {
	pub fn new(
		http_cache: HttpCache,
		unsafely_ignore_certificate_errors: Option<Vec<String>>,
	) -> Result<Self> {
		Ok(HttpClient::from_client(
			create_http_client(
				format!("mashin_core/{}", env!("CARGO_PKG_VERSION")),
				None,
				vec![],
				None,
				unsafely_ignore_certificate_errors,
				None,
			)?,
			http_cache,
		))
	}

	pub fn cache(&self) -> &HttpCache {
		&self.http_cache
	}

	pub fn from_client(client: reqwest::Client, http_cache: HttpCache) -> Self {
		Self { client, http_cache }
	}

	/// Do a GET request without following redirects.
	pub fn get_no_redirect<U: reqwest::IntoUrl>(&self, url: U) -> reqwest::RequestBuilder {
		self.client.get(url)
	}

	pub async fn _download<U: reqwest::IntoUrl>(&self, url: U) -> Result<Vec<u8>> {
		let maybe_bytes = self.inner_download(url).await?;
		match maybe_bytes {
			(Some(bytes), _) => Ok(bytes),
			(None, _) => Err(custom_error("Http", "Not found.")),
		}
	}

	pub async fn download_with_headers<U: reqwest::IntoUrl>(
		&self,
		url: U,
	) -> Result<(Vec<u8>, HeadersMap)> {
		let maybe_bytes = self.inner_download(url).await?;
		match maybe_bytes {
			(Some(bytes), headers) => Ok((bytes, headers)),
			(None, _) => Err(custom_error("Http", "Not found.")),
		}
	}

	async fn inner_download<U: reqwest::IntoUrl>(
		&self,
		url: U,
	) -> Result<(Option<Vec<u8>>, HeadersMap)> {
		let response = self.get_redirected_response(url).await?;

		let response_headers = response.headers();
		let mut result_headers = HashMap::new();

		for key in response_headers.keys() {
			let key_str = key.to_string();
			let values = response_headers.get_all(key);
			let values_str = values
				.iter()
				.map(|e| e.to_str().unwrap().to_string())
				.collect::<Vec<String>>()
				.join(",");
			result_headers.insert(key_str, values_str);
		}

		if response.status() == 404 {
			return Ok((None, result_headers))
		} else if !response.status().is_success() {
			let status = response.status();
			let maybe_response_text = response.text().await.ok();
			bail!(
				"Bad response: {:?}{}",
				status,
				match maybe_response_text {
					Some(text) => format!("\n\n{text}"),
					None => String::new(),
				}
			);
		}

		let bytes = response.bytes().await?.to_vec();
		Ok((Some(bytes), result_headers))
	}

	pub async fn get_redirected_response<U: reqwest::IntoUrl>(&self, url: U) -> Result<Response> {
		let mut url = url.into_url()?;
		let mut response = self.get_no_redirect(url.clone()).send().await?;
		let status = response.status();
		if status.is_redirection() {
			for _ in 0..5 {
				let new_url = resolve_redirect_from_response(&url, &response)?;
				let new_response = self.get_no_redirect(new_url.clone()).send().await?;
				let status = new_response.status();
				if status.is_redirection() {
					response = new_response;
					url = new_url;
				} else {
					return Ok(new_response)
				}
			}
			Err(custom_error("Http", "Too many redirects."))
		} else {
			Ok(response)
		}
	}
}

pub fn resolve_redirect_from_response(request_url: &Url, response: &Response) -> Result<Url> {
	debug_assert!(response.status().is_redirection());
	if let Some(location) = response.headers().get(LOCATION) {
		let location_string = location.to_str()?;
		log::debug!("Redirecting to {:?}...", &location_string);
		let new_url = resolve_url_from_location(request_url, location_string);
		Ok(new_url)
	} else {
		Err(generic_error(format!(
			"Redirection from '{request_url}' did not provide location header"
		)))
	}
}

fn resolve_url_from_location(base_url: &Url, location: &str) -> Url {
	if location.starts_with("http://") || location.starts_with("https://") {
		// absolute uri
		Url::parse(location).expect("provided redirect url should be a valid url")
	} else if location.starts_with("//") {
		// "//" authority path-abempty
		Url::parse(&format!("{}:{}", base_url.scheme(), location))
			.expect("provided redirect url should be a valid url")
	} else if location.starts_with('/') {
		// path-absolute
		base_url.join(location).expect("provided redirect url should be a valid url")
	} else {
		// assuming path-noscheme | path-empty
		let base_url_path_str = base_url.path().to_owned();
		// Pop last part or url (after last slash)
		let segs: Vec<&str> = base_url_path_str.rsplitn(2, '/').collect();
		let new_path = format!("{}/{}", segs.last().unwrap_or(&""), location);
		base_url.join(&new_path).expect("provided redirect url should be a valid url")
	}
}

#[derive(Debug, Eq, PartialEq)]
pub enum FetchOnceResult {
	Code(Vec<u8>, HeadersMap),
	NotModified,
	Redirect(Url, HeadersMap),
}

#[derive(Debug)]
pub struct FetchOnceArgs {
	pub url: Url,
	pub maybe_accept: Option<String>,
	pub maybe_etag: Option<String>,
}

pub async fn fetch_once(http_client: &HttpClient, args: FetchOnceArgs) -> Result<FetchOnceResult> {
	let mut request = http_client.get_no_redirect(args.url.clone());

	if let Some(etag) = args.maybe_etag {
		let if_none_match_val = HeaderValue::from_str(&etag)?;
		request = request.header(IF_NONE_MATCH, if_none_match_val);
	}

	if let Some(accept) = args.maybe_accept {
		let accepts_val = HeaderValue::from_str(&accept)?;
		request = request.header(ACCEPT, accepts_val);
	}
	let response = request.send().await?;

	if response.status() == StatusCode::NOT_MODIFIED {
		return Ok(FetchOnceResult::NotModified)
	}

	let mut result_headers = HashMap::new();
	let response_headers = response.headers();

	for key in response_headers.keys() {
		let key_str = key.to_string();
		let values = response_headers.get_all(key);
		let values_str = values
			.iter()
			.map(|e| e.to_str().unwrap().to_string())
			.collect::<Vec<String>>()
			.join(",");
		result_headers.insert(key_str, values_str);
	}

	if response.status().is_redirection() {
		let new_url = resolve_redirect_from_response(&args.url, &response)?;
		return Ok(FetchOnceResult::Redirect(new_url, result_headers))
	}

	if response.status().is_client_error() || response.status().is_server_error() {
		let err = if response.status() == StatusCode::NOT_FOUND {
			custom_error("NotFound", format!("Import '{}' failed, not found.", args.url))
		} else {
			generic_error(format!("Import '{}' failed: {}", args.url, response.status()))
		};
		return Err(err)
	}

	let body = response.bytes().await?;
	Ok(FetchOnceResult::Code(body.to_vec(), result_headers))
}
