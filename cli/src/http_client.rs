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

use crate::{cache::HttpCache, version::get_user_agent, Result};
use anyhow::bail;
use deno_core::{
	error::{custom_error, generic_error},
	futures::StreamExt,
};
use deno_fetch::create_http_client;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use mashin_runtime::HeadersMap;
use mashin_sdk::ext::async_trait::async_trait;
use reqwest::{
	header::{HeaderValue, ACCEPT, IF_NONE_MATCH, LOCATION},
	Response, StatusCode, Url,
};
use std::{collections::HashMap, fmt::Write};

#[derive(Debug, Clone)]
pub struct HttpClient {
	client: reqwest::Client,
	pub http_cache: HttpCache,
	pub progress_bar: Option<MultiProgress>,
	pub allow_remote: bool,
	pub download_log_level: log::Level,
}

#[async_trait]
impl mashin_runtime::HttpClient for HttpClient {
	type Cache = HttpCache;

	async fn download_with_headers(&self, url: &reqwest::Url) -> Result<(Vec<u8>, HeadersMap)> {
		let maybe_bytes = self.inner_download(url, None).await?;
		match maybe_bytes {
			(Some(bytes), headers) => Ok((bytes, headers)),
			(None, _) => Err(custom_error("Http", "Not found.")),
		}
	}

	fn cache(&self) -> &Self::Cache {
		&self.http_cache
	}

	async fn download_with_progress(&self, url: &reqwest::Url) -> Result<(Vec<u8>, HeadersMap)> {
		let maybe_bytes = self.inner_download(url, self.progress_bar.as_ref()).await?;
		match maybe_bytes {
			(Some(bytes), headers) => Ok((bytes, headers)),
			(None, _) => Err(custom_error("Http", "Not found.")),
		}
	}
}

impl HttpClient {
	pub fn new(
		http_cache: HttpCache,
		unsafely_ignore_certificate_errors: Option<Vec<String>>,
		allow_remote: bool,
		download_log_level: log::Level,
		progress_bar: Option<MultiProgress>,
	) -> Result<Self> {
		Ok(Self {
			client: create_http_client(
				get_user_agent(),
				None,
				vec![],
				None,
				unsafely_ignore_certificate_errors,
				None,
			)?,
			http_cache,
			allow_remote,
			download_log_level,
			progress_bar,
		})
	}

	/// Do a GET request without following redirects.
	pub fn get_no_redirect(&self, url: &reqwest::Url) -> reqwest::RequestBuilder {
		self.client.get(url.clone())
	}

	pub async fn download(&self, url: &reqwest::Url) -> Result<Vec<u8>> {
		let maybe_bytes = self.inner_download(url, None).await?;
		match maybe_bytes {
			(Some(bytes), _) => Ok(bytes),
			(None, _) => Err(custom_error("Http", "Not found.")),
		}
	}

	pub async fn download_text(&self, url: &reqwest::Url) -> Result<String> {
		let bytes = self.download(url).await?;
		Ok(String::from_utf8(bytes)?)
	}

	async fn inner_download(
		&self,
		url: &reqwest::Url,
		progress_guard: Option<&MultiProgress>,
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

		let bytes = get_response_body_with_progress(response, progress_guard).await.map(Some)?;
		Ok((bytes, result_headers))
	}

	pub async fn get_redirected_response(&self, base_url: &reqwest::Url) -> Result<Response> {
		let mut url = base_url.clone();
		let mut response = self.get_no_redirect(&url).send().await?;
		let status = response.status();
		if status.is_redirection() {
			for _ in 0..5 {
				let new_url = resolve_redirect_from_response(&url, &response)?;
				let new_response = self.get_no_redirect(&new_url).send().await?;
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
	pub url: reqwest::Url,
	pub maybe_accept: Option<String>,
	pub maybe_etag: Option<String>,
	pub multi_progress: Option<MultiProgress>,
}

pub async fn fetch_once(http_client: &HttpClient, args: FetchOnceArgs) -> Result<FetchOnceResult> {
	let mut request = http_client.get_no_redirect(&args.url);

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

	let body = get_response_body_with_progress(response, args.multi_progress.as_ref()).await?;

	Ok(FetchOnceResult::Code(body.to_vec(), result_headers))
}

pub async fn get_response_body_with_progress(
	response: reqwest::Response,
	multi_progress: Option<&MultiProgress>,
) -> Result<Vec<u8>> {
	if let Some(multi_progress) = multi_progress {
		if let Some(total_size) = response.content_length() {
			let progress_bar = multi_progress.add(ProgressBar::new(total_size));
			progress_bar.set_message(response.url().to_string());
			progress_bar.set_style(
				ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap().with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()).progress_chars("#>-")
			);
			let mut current_size = 0;
			let mut data = Vec::with_capacity(total_size as usize);
			let mut stream = response.bytes_stream();
			while let Some(item) = stream.next().await {
				let bytes = item?;
				current_size += bytes.len() as u64;
				progress_bar.set_position(current_size);
				data.extend(bytes.into_iter());
			}
			progress_bar.finish_and_clear();
			return Ok(data)
		}
	}
	let bytes = response.bytes().await?;
	Ok(bytes.into())
}
