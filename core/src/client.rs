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
	backend::BackendState,
	config::Config,
	mashin_dir::MashinDir,
	state::{derive_key, StateDiff},
	DynamicLibraryResource, RawState, Result, RuntimeCommand,
};
use anyhow::anyhow;
use console::style;
use deno_core::Resource;
use mashin_sdk::{ResourceAction, Urn};
use sodiumoxide::crypto::{pwhash::Salt, secretbox};
use std::{
	cell::RefCell,
	collections::{BTreeMap, HashMap},
	ffi::c_void,
	ops::Deref,
	rc::Rc,
	sync::{
		atomic::{AtomicU64, Ordering},
		Arc,
	},
};

pub type RegisteredProviders = HashMap<String, RegisteredProvider>;
#[derive(Debug, Default, Clone)]
pub struct ExecutedResources {
	resources: BTreeMap<String, ExecutedResource>,
}

impl Deref for ExecutedResources {
	type Target = BTreeMap<String, ExecutedResource>;

	fn deref(&self) -> &Self::Target {
		&self.resources
	}
}

impl ExecutedResources {
	/// Returns the inner value
	pub fn inner(&self) -> BTreeMap<String, ExecutedResource> {
		self.resources.clone()
	}

	/// Returns true if the map contains a value for the specified key.
	pub fn contains_key(&self, urn: &Urn) -> bool {
		self.resources.contains_key(&urn.to_string())
	}

	/// Inserts a key-value pair into the map.
	pub fn insert(&mut self, urn: &Urn, resource: ExecutedResource) -> Option<ExecutedResource> {
		self.resources.insert(urn.to_string(), resource)
	}

	/// Removes a key from the map, returning the value at the key if the key was previously in the map.
	pub fn remove(&mut self, urn: &Urn) -> Option<ExecutedResource> {
		self.resources.remove(&urn.to_string())
	}

	/// Removes a key from the map, returning the value at the key if the key was previously in the map.
	pub fn get(&mut self, urn: &Urn) -> Option<&ExecutedResource> {
		self.resources.get(&urn.to_string())
	}

	pub fn actions(&self) -> Vec<ResourceAction> {
		self.resources.iter().filter_map(|(_, s)| s.required_change.clone()).collect()
	}

	pub fn print_diff_plan(&self) {
		let mut to_add = 0;
		let mut to_update = 0;
		let mut to_remove = 0;
		let all_pending_actions = self.actions();

		if !all_pending_actions.is_empty() {
			log::info!("\n\nResource actions are indicated with the following symbols:");
			for action in &all_pending_actions {
				match action {
					ResourceAction::Create => {
						to_add += 1;
					},
					ResourceAction::Delete => {
						to_remove += 1;
					},
					ResourceAction::Update { .. } => {
						to_update += 1;
					},
					_ => {},
				}
			}
		}

		if to_add > 0 {
			log::info!("  {} create", style("+").green().bold())
		}

		if to_remove > 0 {
			log::info!("  {} delete", style("-").red().bold())
		}

		if to_update > 0 {
			log::info!("  {} update", style("*").cyan().bold())
		}

		if to_add > 0 || to_remove > 0 || to_update > 0 {
			log::info!("\nMashin will perform the following actions:\n");

			for (urn, executed_resource) in self.iter() {
				if let Err(err) = executed_resource.print_diff(urn) {
					// mainly failing because there is no diff to apply
					log::trace!("{err}")
				}
			}
		}

		log::info!(
			"\n    Plan: {} to add, {} to change, {} to destroy.",
			to_add,
			to_update,
			to_remove
		);
	}
}

pub struct RegisteredProvider {
	pub dylib: DynamicLibraryResource,
	// fixme: use Rc to a dyn Resource
	/// pointer to the provider initialized into the cdylib
	pub ptr: *mut c_void,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutedResource {
	// provider name
	pub provider: String,
	// initial args Rc::into_raw(Rc::new(ResourceArgs))
	//pub args: Option<Rc<ResourceArgs>>,
	// diff
	pub required_change: Option<ResourceAction>,

	pub diff: Option<StateDiff>,
}

impl ExecutedResource {
	pub fn new(
		provider_name: String,
		//args: Rc<ResourceArgs>,
		current_state: &RawState,
		new_state: &RawState,
	) -> Self {
		let diff = new_state.compare_with(current_state);

		// doing some checkup here, so we dont have to borrow the both state, so they can be dropped from here
		// as we only need the diff state and the next action needed
		let required_change = if current_state.is_null() {
			Some(ResourceAction::Create)
		} else if current_state.inner() == new_state.inner() {
			None
		} else {
			Some(ResourceAction::Update { diff: Rc::new(diff.provider_resource_diff()) })
		};

		ExecutedResource { provider: provider_name, diff: Some(diff), required_change }
	}

	pub fn print_diff(&self, urn: &str) -> Result<()> {
		let resource_action = self.required_change.clone().ok_or(anyhow!("no changes required"))?;
		let resource_diff = self.diff.clone().ok_or(anyhow!("no resource diff"))?;

		let total_changes = resource_diff.len();
		let mut total_changes_processed = 0;

		let arrow = match &resource_action {
			ResourceAction::Update { .. } => style("-->").cyan().bold().to_string(),
			ResourceAction::Create => style("-->").green().bold().to_string(),
			ResourceAction::Delete => style("-->").red().bold().to_string(),
			_ => "".to_string(),
		};

		//    --> [aws:s3:bucket?=test1234atmos1000]: Need to be created
		log::info!(
			"   {arrow} [{}]: Need to be {}",
			style(urn.replace("urn:provider:", "")).bold(),
			resource_action.action_past_str().to_lowercase()
		);

		for resource_diff in resource_diff.iter() {
			if resource_diff.is_eq() {
				continue
			}

			let closing_line = resource_diff.print_diff()?.unwrap_or_default();

			total_changes_processed += 1;

			if total_changes_processed == total_changes {
				log::info!("   {}", closing_line,);
			}
		}

		Ok(())
	}
}

#[derive(Default)]
pub struct MashinBuilder<'a, T: Config> {
	state_handler: Option<Rc<RefCell<BackendState>>>,
	passphrase: Option<&'a [u8]>,
	executed_resources: Option<Rc<RefCell<ExecutedResources>>>,
	progress_manager: Option<Rc<T::ProgressManager>>,
	http_client: Option<Rc<T::HttpClient>>,
	mashin_dir: Option<MashinDir>,
	runtime_command: Option<RuntimeCommand>,
	resources_count: Option<u64>,
	salt: Option<&'a [u8; 32]>,
}
impl<'a, T: Config> MashinBuilder<'a, T> {
	pub fn new() -> Self {
		MashinBuilder {
			state_handler: None,
			passphrase: None,
			executed_resources: None,
			progress_manager: None,
			http_client: None,
			mashin_dir: None,
			runtime_command: None,
			resources_count: None,
			salt: None,
		}
	}

	pub fn with_state_handler(&mut self, handler: Rc<RefCell<BackendState>>) -> &mut Self {
		self.state_handler = Some(handler);
		self
	}

	pub fn with_passphrase(&mut self, passphrase: &'a [u8]) -> &mut Self {
		self.passphrase = Some(passphrase);
		self
	}

	pub fn with_executed_resources(
		&mut self,
		executed_resources: Option<Rc<RefCell<ExecutedResources>>>,
	) -> &mut Self {
		self.executed_resources = executed_resources;
		self
	}

	pub fn with_progress_manager(&mut self, progress_manager: Rc<T::ProgressManager>) -> &mut Self {
		self.progress_manager = Some(progress_manager);
		self
	}

	pub fn with_http_client(&mut self, http_client: Rc<T::HttpClient>) -> &mut Self {
		self.http_client = Some(http_client);
		self
	}

	pub fn with_mashin_dir(&mut self, mashin_dir: MashinDir) -> &mut Self {
		self.mashin_dir = Some(mashin_dir);
		self
	}

	pub fn with_runtime_command(&mut self, runtime_command: RuntimeCommand) -> &mut Self {
		self.runtime_command = Some(runtime_command);
		self
	}

	pub fn with_resources_count(&mut self, resources_count: u64) -> &mut Self {
		self.resources_count = Some(resources_count);
		self
	}

	pub fn with_salt(&mut self, salt: &'a [u8; 32]) -> &mut Self {
		self.salt = Some(salt);
		self
	}

	pub fn build(&self) -> Result<MashinEngine<T>> {
		let mashin_dir = self.mashin_dir.clone().unwrap_or_default();
		let salt = Salt(*self.salt.unwrap_or(&[
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
			24, 25, 26, 27, 28, 29, 30, 31,
		]));
		let key = derive_key(self.passphrase.unwrap_or_default(), salt)?;

		Ok(MashinEngine {
			resources_count: Arc::new(AtomicU64::new(self.resources_count.unwrap_or_default())),
			command: self.runtime_command.clone().unwrap_or(RuntimeCommand::Prepare),
			mashin_dir,
			state_handler: self
				.state_handler
				.clone()
				.ok_or(anyhow!("State handler is required"))?,
			key,
			executed_resources: self.executed_resources.clone().unwrap_or_default(),
			progress_manager: self
				.progress_manager
				.clone()
				.ok_or(anyhow!("Progress manager is required"))?,
			http_client: self.http_client.clone().ok_or(anyhow!("HTTP Client is required"))?,
			providers: Default::default(),
		})
	}
}

/// Instance of a single client for an Mashin consumer.
pub struct MashinEngine<T: Config> {
	pub resources_count: Arc<AtomicU64>,
	pub command: RuntimeCommand,
	pub mashin_dir: MashinDir,
	pub state_handler: Rc<RefCell<BackendState>>,
	pub key: secretbox::Key,
	pub executed_resources: Rc<RefCell<ExecutedResources>>,
	pub progress_manager: Rc<T::ProgressManager>,
	pub http_client: Rc<T::HttpClient>,
	pub providers: Rc<RefCell<RegisteredProviders>>,
}

impl<T: Config> Resource for MashinEngine<T> {} // Blank impl

impl<T: Config> MashinEngine<T> {
	pub fn resources_count(&self) -> u64 {
		self.resources_count.load(Ordering::Relaxed)
	}

	pub fn inc_resources_count(&self) {
		let current_value = self.resources_count();
		self.resources_count.store(current_value.saturating_add(1), Ordering::Relaxed);
	}
}

impl<T: Config> Drop for MashinEngine<T> {
	fn drop(&mut self) {
		let drop_provider = |(_, provider): (_, &RegisteredProvider)| {
			if let Err(err) = provider.dylib.call_drop(provider.ptr) {
				log::error!("unable to drop provider; {err}");
			}
		};

		self.providers.borrow_mut().iter().for_each(drop_provider)
	}
}
