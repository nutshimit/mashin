use crate::{http_client::HttpClient, util::time, version};
use console::style;
use deno_core::{
	anyhow::{bail, Context},
	error::AnyError,
	futures::{future::BoxFuture, FutureExt},
};
use deno_semver::Version;
use mashin_runtime::HttpClient as _;
use once_cell::sync::Lazy;
use std::{
	borrow::Cow,
	env, fs,
	ops::Sub,
	path::{Path, PathBuf},
	process::Command,
	str::FromStr,
	sync::Arc,
	time::Duration,
};

static ARCHIVE_NAME: Lazy<String> = Lazy::new(|| format!("mashin-{}.zip", env!("TARGET")));

const RELEASE_URL: &str = "https://github.com/nutshimit/mashin/releases";

// How often query server for new version. In hours.
const UPGRADE_CHECK_INTERVAL: i64 = 24;

const UPGRADE_CHECK_FETCH_DELAY: Duration = Duration::from_millis(500);

/// Environment necessary for doing the update checker.
/// An alternate trait implementation can be provided for testing purposes.
trait UpdateCheckerEnvironment: Clone + Send + Sync {
	fn latest_version(&self) -> BoxFuture<'static, Result<String, AnyError>>;
	fn current_version(&self) -> Cow<str>;
	fn read_check_file(&self) -> String;
	fn write_check_file(&self, text: &str);
	fn current_time(&self) -> chrono::DateTime<chrono::Utc>;
}

#[derive(Clone)]
struct RealUpdateCheckerEnvironment {
	http_client: Arc<HttpClient>,
	cache_file_path: PathBuf,
	current_time: chrono::DateTime<chrono::Utc>,
}

impl RealUpdateCheckerEnvironment {
	pub fn new(http_client: Arc<HttpClient>, cache_file_path: PathBuf) -> Self {
		Self {
			http_client,
			cache_file_path,
			// cache the current time
			current_time: time::utc_now(),
		}
	}
}

impl UpdateCheckerEnvironment for RealUpdateCheckerEnvironment {
	fn latest_version(&self) -> BoxFuture<'static, Result<String, AnyError>> {
		let http_client = self.http_client.clone();
		async move { get_latest_release_version(&http_client).await }.boxed()
	}

	fn current_version(&self) -> Cow<str> {
		Cow::Borrowed(version::release_version_or_canary_commit_hash())
	}

	fn read_check_file(&self) -> String {
		std::fs::read_to_string(&self.cache_file_path).unwrap_or_default()
	}

	fn write_check_file(&self, text: &str) {
		let _ = std::fs::write(&self.cache_file_path, text);
	}

	fn current_time(&self) -> chrono::DateTime<chrono::Utc> {
		self.current_time
	}
}

struct UpdateChecker<TEnvironment: UpdateCheckerEnvironment> {
	env: TEnvironment,
	maybe_file: Option<CheckVersionFile>,
}

impl<TEnvironment: UpdateCheckerEnvironment> UpdateChecker<TEnvironment> {
	pub fn new(env: TEnvironment) -> Self {
		let maybe_file = CheckVersionFile::parse(env.read_check_file());
		Self { env, maybe_file }
	}

	pub fn should_check_for_new_version(&self) -> bool {
		match &self.maybe_file {
			Some(file) => {
				let last_check_age =
					self.env.current_time().signed_duration_since(file.last_checked);
				last_check_age > chrono::Duration::hours(UPGRADE_CHECK_INTERVAL)
			},
			None => true,
		}
	}

	/// Returns the version if a new one is available and it should be prompted about.
	pub fn should_prompt(&self) -> Option<String> {
		let file = self.maybe_file.as_ref()?;
		// If the current version saved is not the actualy current version of the binary
		// It means
		// - We already check for a new version today
		// - The user have probably upgraded today
		// So we should not prompt and wait for tomorrow for the latest version to be updated again
		if file.current_version != self.env.current_version() {
			return None
		}
		if file.latest_version == self.env.current_version() {
			return None
		}

		if let Ok(current) = Version::parse_standard(&self.env.current_version()) {
			if let Ok(latest) = Version::parse_standard(&file.latest_version) {
				if current >= latest {
					return None
				}
			}
		}

		let last_prompt_age = self.env.current_time().signed_duration_since(file.last_prompt);
		if last_prompt_age > chrono::Duration::hours(UPGRADE_CHECK_INTERVAL) {
			Some(file.latest_version.clone())
		} else {
			None
		}
	}

	/// Store that we showed the update message to the user.
	pub fn store_prompted(self) {
		if let Some(file) = self.maybe_file {
			self.env
				.write_check_file(&file.with_last_prompt(self.env.current_time()).serialize());
		}
	}
}

fn get_minor_version(version: &str) -> &str {
	version.rsplitn(2, '.').collect::<Vec<&str>>()[1]
}

fn print_release_notes(current_version: &str, new_version: &str) {
	if get_minor_version(current_version) != get_minor_version(new_version) {
		log::info!(
			"{}{}",
			"Release notes: https://github.com/nutshimit/mashin/releases/tag/v",
			&new_version,
		);
	}
}

pub fn check_for_upgrades(http_client: Arc<HttpClient>, cache_file_path: PathBuf) {
	if env::var("MASHIN_NO_UPDATE_CHECK").is_ok() {
		return
	}

	let env = RealUpdateCheckerEnvironment::new(http_client, cache_file_path);
	let update_checker = UpdateChecker::new(env);

	if update_checker.should_check_for_new_version() {
		let env = update_checker.env.clone();
		// do this asynchronously on a separate task
		tokio::spawn(async move {
			// Sleep for a small amount of time to not unnecessarily impact startup
			// time.
			tokio::time::sleep(UPGRADE_CHECK_FETCH_DELAY).await;

			fetch_and_store_latest_version(&env).await;
		});
	}

	// Print a message if an update is available
	if let Some(upgrade_version) = update_checker.should_prompt() {
		if log::log_enabled!(log::Level::Info) && atty::is(atty::Stream::Stderr) {
			if version::is_canary() {
				eprint!("{} ", style("A new canary release of Mashin is available.").green());
				eprintln!(
					"{}",
					style("Run `mashin upgrade --canary` to install it.").italic().bold()
				);
			} else {
				eprint!(
					"{} {} → {} ",
					style("A new release of Mashin is available:").green(),
					style(version::mashin()).cyan(),
					style(&upgrade_version).cyan()
				);
				eprintln!("{}", style("Run `mashin upgrade` to install it.").italic().bold());
				print_release_notes(version::mashin(), &upgrade_version);
			}

			update_checker.store_prompted();
		}
	}
}

async fn fetch_and_store_latest_version<TEnvironment: UpdateCheckerEnvironment>(
	env: &TEnvironment,
) {
	// Fetch latest version or commit hash from server.
	let latest_version = match env.latest_version().await {
		Ok(latest_version) => latest_version,
		Err(_) => return,
	};

	env.write_check_file(
		&CheckVersionFile {
			// put a date in the past here so that prompt can be shown on next run
			last_prompt: env
				.current_time()
				.sub(chrono::Duration::hours(UPGRADE_CHECK_INTERVAL + 1)),
			last_checked: env.current_time(),
			current_version: env.current_version().to_string(),
			latest_version,
		}
		.serialize(),
	);
}

pub async fn upgrade(client: &HttpClient) -> Result<(), AnyError> {
	let current_exe_path = std::env::current_exe()?;
	let metadata = fs::metadata(&current_exe_path)?;
	let permissions = metadata.permissions();

	if permissions.readonly() {
		bail!("You do not have write permission to {}", current_exe_path.display());
	}
	#[cfg(unix)]
	if std::os::unix::fs::MetadataExt::uid(&metadata) == 0 &&
		!nix::unistd::Uid::effective().is_root()
	{
		bail!(
			concat!(
				"You don't have write permission to {} because it's owned by root.\n",
				"Consider updating mashin through your package manager if its installed from it.\n",
				"Otherwise run `mashin upgrade` as root.",
			),
			current_exe_path.display()
		);
	}

	let install_version = {
		let latest_version = {
			log::info!("Looking up latest version");
			get_latest_release_version(client).await?
		};

		let current_is_most_recent = if !crate::version::is_canary() {
			let current = Version::parse_standard(crate::version::mashin()).unwrap();
			let latest = Version::parse_standard(&latest_version).unwrap();
			current >= latest
		} else {
			false
		};

		if current_is_most_recent {
			log::info!(
				"Local mashin version {} is the most recent release",
				crate::version::mashin()
			);
			return Ok(())
		} else {
			log::info!("Found latest version {}", latest_version);
			latest_version
		}
	};

	let download_url = format!("{}/download/v{}/{}", RELEASE_URL, install_version, *ARCHIVE_NAME);

	let archive_data = download_package(client, &download_url)
		.await
		.with_context(|| format!("Failed downloading {download_url}"))?;

	log::info!("Mashin is upgrading to version {}", &install_version);

	let temp_dir = tempfile::TempDir::new()?;
	let new_exe_path = unpack_into_dir(archive_data, cfg!(windows), &temp_dir)?;
	fs::set_permissions(&new_exe_path, permissions)?;
	check_exe(&new_exe_path)?;

	let output_result = replace_exe(&new_exe_path, &current_exe_path);

	if let Err(err) = output_result {
		const WIN_ERROR_ACCESS_DENIED: i32 = 5;
		if cfg!(windows) && err.raw_os_error() == Some(WIN_ERROR_ACCESS_DENIED) {
			return Err(err).with_context(|| {
				format!(
					concat!(
						"Could not replace the mashin executable. This may be because an ",
						"existing mashin process is running. Please ensure there are no ",
						"running mashin processes (ex. Stop-Process -Name mashin ; mashin {}), ",
						"close any editors before upgrading, and ensure you have ",
						"sufficient permission to '{}'."
					),
					// skip the first argument, which is the executable path
					std::env::args().skip(1).collect::<Vec<_>>().join(" "),
					current_exe_path.display(),
				)
			})
		} else {
			return Err(err.into())
		}
	}
	log::info!("Upgraded successfully");
	print_release_notes(version::mashin(), &install_version);

	drop(temp_dir); // delete the temp dir
	Ok(())
}

async fn get_latest_release_version(client: &HttpClient) -> Result<String, AnyError> {
	let text = client
		.download_text(&reqwest::Url::from_str("https://get.mashin.land/release-latest.txt")?)
		.await?;
	let version = text.trim().to_string();
	Ok(version.replace('v', ""))
}

async fn download_package(client: &HttpClient, download_url: &str) -> Result<Vec<u8>, AnyError> {
	log::info!("Downloading {}", &download_url);
	let (maybe_bytes, _) =
		{ client.download_with_progress(&reqwest::Url::from_str(download_url)?).await? };
	Ok(maybe_bytes)
}

pub fn unpack_into_dir(
	archive_data: Vec<u8>,
	is_windows: bool,
	temp_dir: &tempfile::TempDir,
) -> Result<PathBuf, std::io::Error> {
	const EXE_NAME: &str = "mashin";
	let temp_dir_path = temp_dir.path();
	let exe_ext = if is_windows { "exe" } else { "" };
	let archive_path = temp_dir_path.join(EXE_NAME).with_extension("zip");
	let exe_path = temp_dir_path.join(EXE_NAME).with_extension(exe_ext);
	assert!(!exe_path.exists());

	let archive_ext = Path::new(&*ARCHIVE_NAME).extension().and_then(|ext| ext.to_str()).unwrap();
	let unpack_status = match archive_ext {
		"zip" if cfg!(windows) => {
			fs::write(&archive_path, &archive_data)?;
			Command::new("powershell.exe")
				.arg("-NoLogo")
				.arg("-NoProfile")
				.arg("-NonInteractive")
				.arg("-Command")
				.arg(
					"& {
            param($Path, $DestinationPath)
            trap { $host.ui.WriteErrorLine($_.Exception); exit 1 }
            Add-Type -AssemblyName System.IO.Compression.FileSystem
            [System.IO.Compression.ZipFile]::ExtractToDirectory(
              $Path,
              $DestinationPath
            );
          }",
				)
				.arg("-Path")
				.arg(format!("'{}'", &archive_path.to_str().unwrap()))
				.arg("-DestinationPath")
				.arg(format!("'{}'", &temp_dir_path.to_str().unwrap()))
				.spawn()
				.map_err(|err| {
					if err.kind() == std::io::ErrorKind::NotFound {
						std::io::Error::new(
							std::io::ErrorKind::NotFound,
							"`powershell.exe` was not found in your PATH",
						)
					} else {
						err
					}
				})?
				.wait()?
		},
		"zip" => {
			fs::write(&archive_path, &archive_data)?;
			Command::new("unzip")
				.current_dir(temp_dir_path)
				.arg(&archive_path)
				.spawn()
				.map_err(|err| {
					if err.kind() == std::io::ErrorKind::NotFound {
						std::io::Error::new(
							std::io::ErrorKind::NotFound,
							"`unzip` was not found in your PATH, please install `unzip`",
						)
					} else {
						err
					}
				})?
				.wait()?
		},
		ext => panic!("Unsupported archive type: '{ext}'"),
	};
	assert!(unpack_status.success());
	assert!(exe_path.exists());
	fs::remove_file(&archive_path)?;
	Ok(exe_path)
}

fn replace_exe(from: &Path, to: &Path) -> Result<(), std::io::Error> {
	if cfg!(windows) {
		// On windows you cannot replace the currently running executable.
		// so first we rename it to mashin.old.exe
		fs::rename(to, to.with_extension("old.exe"))?;
	} else {
		fs::remove_file(to)?;
	}
	// Windows cannot rename files across device boundaries, so if rename fails,
	// we try again with copy.
	fs::rename(from, to).or_else(|_| fs::copy(from, to).map(|_| ()))?;
	Ok(())
}

fn check_exe(exe_path: &Path) -> Result<(), AnyError> {
	let output = Command::new(exe_path)
		.arg("version")
		.stderr(std::process::Stdio::inherit())
		.output()?;
	assert!(output.status.success());
	Ok(())
}

#[derive(Debug)]
struct CheckVersionFile {
	pub last_prompt: chrono::DateTime<chrono::Utc>,
	pub last_checked: chrono::DateTime<chrono::Utc>,
	pub current_version: String,
	pub latest_version: String,
}

impl CheckVersionFile {
	pub fn parse(content: String) -> Option<Self> {
		let split_content = content.split('!').collect::<Vec<_>>();

		if split_content.len() != 4 {
			return None
		}

		let latest_version = split_content[2].trim().to_owned();
		if latest_version.is_empty() {
			return None
		}
		let current_version = split_content[3].trim().to_owned();
		if current_version.is_empty() {
			return None
		}

		let last_prompt = chrono::DateTime::parse_from_rfc3339(split_content[0])
			.map(|dt| dt.with_timezone(&chrono::Utc))
			.ok()?;
		let last_checked = chrono::DateTime::parse_from_rfc3339(split_content[1])
			.map(|dt| dt.with_timezone(&chrono::Utc))
			.ok()?;

		Some(CheckVersionFile { last_prompt, last_checked, current_version, latest_version })
	}

	fn serialize(&self) -> String {
		format!(
			"{}!{}!{}!{}",
			self.last_prompt.to_rfc3339(),
			self.last_checked.to_rfc3339(),
			self.latest_version,
			self.current_version,
		)
	}

	fn with_last_prompt(self, dt: chrono::DateTime<chrono::Utc>) -> Self {
		Self { last_prompt: dt, ..self }
	}
}
