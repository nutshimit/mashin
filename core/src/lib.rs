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

pub use crate::{
	backend::BackendState,
	client::{
		ExecutedResource, ExecutedResources, MashinBuilder, MashinEngine, RegisteredProvider,
		RegisteredProviders,
	},
	config::Config,
	ffi::{DynamicLibraryResource, ForeignFunction, NativeType, NativeValue, Symbol},
	state::{EncryptedState, FileState, ProjectState, RawState, StateHandler},
};
use async_trait::async_trait;
use deno_core::ModuleSpecifier;
pub use mashin_sdk as sdk;
pub(crate) use sdk::Result;
use std::{collections::HashMap, fs::File, path::PathBuf, time::SystemTime};

mod backend;
mod client;
mod config;
mod ffi;
pub mod mashin_dir;
mod state;

#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: "mashin::core",
            $patter $(, $values)*
		)
	};
}

#[derive(Clone)]
pub struct StateInner {
	pub get_symbol: Symbol,
	pub save_symbol: Symbol,
}

#[derive(PartialEq, Clone)]
pub enum RuntimeCommand {
	/// First run, mainly used to get the total count of resources
	Prepare,
	/// Read all resources via the assigned providers
	Read,
	/// Apply changes
	Apply,
}

pub type HeadersMap = HashMap<String, String>;

pub trait ProgressManager: Default + Clone {
	fn println(&self, msg: &str);
	/// Progress bar for the resources, do not use for anything else
	fn progress_bar(&self) -> Option<indicatif::ProgressBar>;
}

#[async_trait]
pub trait HttpClient {
	type Cache: HttpCache;
	async fn download_with_headers(&self, url: &reqwest::Url) -> Result<(Vec<u8>, HeadersMap)>;
	async fn download_with_progress(&self, url: &reqwest::Url) -> Result<(Vec<u8>, HeadersMap)>;
	fn cache(&self) -> &Self::Cache;
}

pub trait HttpCache: Send + Sync + Clone {
	type SourceFile: Clone;
	fn fetch_cached_path(
		&self,
		specifier: &reqwest::Url,
		redirect_limit: i64,
	) -> Result<Option<PathBuf>>;
	fn set(&self, url: &reqwest::Url, headers_map: HeadersMap, content: &[u8]) -> Result<PathBuf>;
	fn get(&self, url: &reqwest::Url) -> Result<(File, HeadersMap, SystemTime)>;
	fn fetch_cached(
		&self,
		specifier: &ModuleSpecifier,
		redirect_limit: i64,
	) -> Result<Option<Self::SourceFile>>;
	fn build_remote_file(
		&self,
		specifier: &ModuleSpecifier,
		bytes: Vec<u8>,
		headers: &HashMap<String, String>,
	) -> Result<Self::SourceFile>;
}
