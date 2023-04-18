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

use crate::{
	backend::BackendState,
	colors,
	state::{derive_key, StateDiff},
	DynamicLibraryResource, RawState, Result,
};
use anyhow::anyhow;
use deno_core::Resource;
use mashin_sdk::{ResourceAction, Urn};
use sodiumoxide::crypto::{pwhash::Salt, secretbox};
use std::{
	cell::RefCell,
	collections::{BTreeMap, HashMap},
	ffi::c_void,
	ops::Deref,
	rc::Rc,
};

pub type RegisteredProviders = HashMap<String, RegisteredProvider>;
#[derive(Default, Clone)]
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
						log::info!("  {} create", colors::green_bold("+"))
					},
					ResourceAction::Delete => {
						to_remove += 1;
						log::info!("  {} delete", colors::red_bold("-"))
					},
					ResourceAction::Update { .. } => {
						to_update += 1;
						log::info!("  {} update", colors::cyan_bold("*"))
					},
					_ => {},
				}
			}
		}

		log::info!("\nMashin will perform the following actions:\n");

		for (urn, executed_resource) in self.iter() {
			if let Err(err) = executed_resource.print_diff(urn) {
				// mainly failing because there is no diff to apply
				log::trace!("{err}")
			}
		}

		log::info!("Plan: {} to add, {} to change, {} to destroy.", to_add, to_update, to_remove);
	}
}

pub struct RegisteredProvider {
	pub dylib: DynamicLibraryResource,
	// fixme: use Rc to a dyn Resource
	/// pointer to the provider initialized into the cdylib
	pub ptr: *mut c_void,
}

#[derive(Clone, Default)]
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
			ResourceAction::Update { .. } => colors::cyan_bold("-->").to_string(),
			ResourceAction::Create => colors::green_bold("-->").to_string(),
			ResourceAction::Delete => colors::red_bold("-->").to_string(),
			_ => "".to_string(),
		};

		//    --> [aws:s3:bucket?=test1234atmos1000]: Need to be created
		log::info!(
			"   {arrow} [{}]: Need to be {}",
			colors::bold(urn.replace("urn:provider:", "")),
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

/// Instance of a single client for an Mashin consumer.
pub struct MashinEngine {
	pub state_handler: Rc<RefCell<BackendState>>,
	pub key: secretbox::Key,
	pub providers: Rc<RefCell<RegisteredProviders>>,
	pub executed_resources: Rc<RefCell<ExecutedResources>>,
}

impl Resource for MashinEngine {} // Blank impl

impl MashinEngine {
	pub fn new(
		state_handler: Rc<RefCell<BackendState>>,
		passphrase: &[u8],
		executed_resources: Option<Rc<RefCell<ExecutedResources>>>,
	) -> Result<Self> {
		// FIXME: use dynamic salt
		let salt = Salt([
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
			24, 25, 26, 27, 28, 29, 30, 31,
		]);

		let key = derive_key(passphrase, salt)?;

		Ok(Self {
			state_handler,
			key,
			providers: Default::default(),
			executed_resources: executed_resources.unwrap_or_default(),
		})
	}
}

impl Drop for MashinEngine {
	fn drop(&mut self) {
		let drop_provider = |(_, provider): (_, &RegisteredProvider)| {
			if let Err(err) = provider.dylib.call_drop(provider.ptr) {
				log::error!("unable to drop provider; {err}");
			}
		};

		self.providers.borrow_mut().iter().for_each(drop_provider)
	}
}
