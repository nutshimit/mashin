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

//! The Mashin SDK is a library designed to facilitate the management of resources and providers
//! within the Mashin engine for infrastructure-as-code solutions. It provides a powerful and
//! flexible way to define, create, update, and delete resources across various cloud services,
//! streamlining the process of creating and managing infrastructure components.
//!
//! The `construct_provider!` and `resource` macros are essential for developers to create custom
//! providers and resources, which can then be exposed to the Mashin engine, enabling Ops teams to
//! efficiently manage infrastructure components.
//!
//! - **`construct_provider!` macro**: This macro simplifies the process of creating custom
//!   providers by generating the necessary boilerplate code for implementing the `Provider` trait.
//!   Users only need to provide the provider-specific configuration and logic for handling
//!   resources.
//!
//! - **`resource` macro**: This macro generates the required code to implement the `Resource` trait
//!   for custom resources. Users only need to define the resource's properties and implement the
//!   logic for creating, updating, and deleting the resource using the provider.
//!
//!
//! # Key concepts
//!
//! - **Provider**: A struct that represents a cloud service, such as AWS, Azure, or GCP, and
//!   implements the logic for creating, updating, and deleting resources on that service.
//!
//! - **Resource**: A struct that represents an individual infrastructure component, such as a
//!   virtual machine, a network, or a database.
//!
//! # Re-exports
//!
//! This module re-exports some helpers from other libraries, such as `serde`, `async_trait`,
//! `parking_lot`, `serde_json`, and `tokio`. These re-exports are available under the `ext`
//! submodule.
//!
//! # Example
//! ```no_run
//! mashin_sdk::construct_provider!(
//! 	test_provider,
//! 	resources = [my_resource],
//! );
//!
//! #[mashin_sdk::resource]
//! pub mod my_resource {
//! 	#[mashin::config]
//! 	pub struct Config {}
//!
//! 	#[mashin::resource]
//! 	pub struct Resource {}
//!
//! 	#[mashin::calls]
//! 	impl mashin_sdk::Resource for Resource { ... }
//! }

pub use crate::urn::Urn;
pub use anyhow::Result;
use async_trait::async_trait;
pub use build::build;
pub use deserialize::deserialize_state_field;
pub use logger::CliLogger;
pub use mashin_macro::{provider, resource};
use parking_lot::Mutex;
pub use provider_state::ProviderState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{any::Any, cell::RefCell, fmt::Debug, rc::Rc, sync::Arc};

mod build;
mod deserialize;
mod logger;
mod provider;
mod provider_state;
mod urn;

pub const KEY_CONFIG: &str = "__config";
pub const KEY_URN: &str = "__urn";
pub const KEY_NAME: &str = "__name";
pub const KEY_SENSITIVE: &str = "__sensitive";
// keys to skip
pub const KEYS_CORE: [&str; 1] = [KEY_SENSITIVE];
pub const KEY_VALUE: &str = "__value";

/// Re-exports some helpers from other libraries
pub mod ext {
	pub use anyhow;
	pub use async_trait;
	pub use parking_lot;
	pub use serde;
	pub use serde_json;
	pub use tokio;
}

/// Unique resource id within a provider
pub type ResourceId = u32;

/// A struct that holds the input arguments for resource actions, such as the resource's URN,
/// the raw configuration, and the raw state.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceArgs {
	pub action: Rc<ResourceAction>,
	pub urn: Rc<Urn>,
	pub raw_config: Rc<Value>,
	pub raw_state: Rc<RefCell<Value>>,
}

/// An enum that defines the possible actions that can be performed on a
/// resource, such as creating, updating, or deleting
#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ResourceAction {
	Update {
		diff: Rc<ResourceDiff>,
	},
	Create,
	Delete,
	#[default]
	Get,
}

impl ResourceAction {
	/// Present participe of the action
	pub fn action_present_participe_str(&self) -> &str {
		match self {
			ResourceAction::Update { .. } => "Updating",
			ResourceAction::Create => "Creating",
			ResourceAction::Delete => "Deleting",
			ResourceAction::Get => "Reading",
		}
	}
	/// Simple present of the action
	pub fn action_present_str(&self) -> &str {
		match self {
			ResourceAction::Update { .. } => "Update",
			ResourceAction::Create => "Create",
			ResourceAction::Delete => "Delete",
			ResourceAction::Get => "Read",
		}
	}

	pub fn action_past_str(&self) -> &str {
		match self {
			ResourceAction::Update { .. } => "Updated",
			ResourceAction::Create => "Created",
			ResourceAction::Delete => "Deleted",
			ResourceAction::Get => "Read",
		}
	}
}

/// A struct that holds information about the differences between two resource states,
/// such as the properties that have changed during an update operation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceDiff(Vec<String>);

impl ResourceDiff {
	pub fn new(diff: Vec<String>) -> Self {
		Self(diff)
	}

	pub fn has_change(&self, key: impl ToString) -> bool {
		self.0.contains(&key.to_string())
	}
}

/// `ResourceResult` represents the serialized state of a resource after it has been processed
/// by a provider. The Mashin engine uses this data to compare the actual state with the desired
/// state, determining whether any changes have occurred.
///
/// When updating a resource, `ResourceResult` should also include any changes to the resource's
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceResult(serde_json::Value);

impl ResourceResult {
	pub fn new(raw_state_as_json: serde_json::Value) -> Self {
		ResourceResult(raw_state_as_json)
	}

	pub fn inner(&self) -> serde_json::Value {
		self.0.clone()
	}
}

/// A trait for resource equality comparisons.
pub trait ResourceEq {
	/// Returns the resource as a `&dyn Any`.
	fn as_any(&self) -> &dyn Any;
	/// Compares two resources for equality.
	///
	/// Returns `true` if the resources are equal, `false` otherwise.
	fn is_eq(&self, other: &dyn Resource) -> bool;
}

/// A trait for serializing a resource to its raw state.
pub trait ResourceSerialize {
	/// Converts the resource to its raw state as a `serde_json::Value`.
	fn to_raw_state(&self) -> Result<serde_json::Value>;
}

/// A trait representing default behavior for a resource.
pub trait ResourceDefault {
	fn new(name: &str, urn: &str) -> Self
	where
		Self: Sized;
	fn set_raw_config(&mut self, config: &Rc<Value>);
	fn from_current_state(
		name: &str,
		urn: &str,
		raw_state: Rc<RefCell<Value>>,
	) -> Result<Rc<RefCell<Self>>>
	where
		Self: Default,
		for<'de> Self: Deserialize<'de>,
	{
		let state = raw_state.borrow_mut();
		if state.as_null().is_some() {
			Ok(Rc::new(RefCell::new(Self::new(name, urn))))
		} else {
			let mut state = state.clone();
			let merge_fields = json!({
				"__name": {
					"value": name,
					"sensitive": true,
				},
				"__urn": {
					"value": urn,
					"sensitive": true,
				},
			});

			merge_json(&mut state, &merge_fields);

			Ok(Rc::new(RefCell::new(::serde_json::from_value::<Self>(state)?)))
		}
	}
	/// Returns the name of the resource.
	fn name(&self) -> &str;
	/// Returns the URN of the resource.
	fn urn(&self) -> &str;
}

/// A trait representing a resource in the Mashin SDK.
///
/// A resource is a manageable entity within a provider, such as a virtual
/// machine, database, or storage container. This trait defines the methods
/// necessary for managing the resource's lifecycle.
///
/// The Resource state is generated from the `self` value.
#[async_trait]
pub trait Resource: ResourceEq + ResourceSerialize + ResourceDefault {
	/// Retrieves the current state of the resource.
	///
	/// ### Arguments
	///
	/// * `provider_state` - An `Arc<Mutex<ProviderState>>` that represents the current state of the provider.
	///
	/// ### Returns
	///
	/// A `Result` that indicates whether the operation was successful or not.
	async fn get(&mut self, provider_state: Arc<Mutex<ProviderState>>) -> Result<()>;
	/// Creates the resource.
	///
	/// ### Arguments
	///
	/// * `provider_state` - An `Arc<Mutex<ProviderState>>` that represents the current state of the provider.
	///
	/// ### Returns
	///
	/// A `Result` that indicates whether the operation was successful or not.
	async fn create(&mut self, provider_state: Arc<Mutex<ProviderState>>) -> Result<()>;
	/// Deletes the resource.
	///
	/// ### Arguments
	///
	/// * `provider_state` - An `Arc<Mutex<ProviderState>>` that represents the current state of the provider.
	///
	/// ### Returns
	///
	/// A `Result` that indicates whether the operation was successful or not.
	async fn delete(&mut self, provider_state: Arc<Mutex<ProviderState>>) -> Result<()>;
	/// Updates the resource with new data.
	///
	/// # Arguments
	///
	/// * `provider_state` - An `Arc<Mutex<ProviderState>>` that represents the current state of the provider.
	/// * `diff` - A `ResourceDiff` that represents the changes to be applied to the resource.
	///
	/// # Returns
	///
	/// A `Result` that indicates whether the operation was successful or not.
	async fn update(
		&mut self,
		provider_state: Arc<Mutex<ProviderState>>,
		diff: &ResourceDiff,
	) -> Result<()>;
}

impl<R: 'static + PartialEq> ResourceEq for R {
	fn as_any(&self) -> &dyn Any {
		self
	}

	fn is_eq(&self, other: &dyn Resource) -> bool {
		// Do a type-safe casting. If the types are different,
		// return false, otherwise test the values for equality.
		other.as_any().downcast_ref::<R>().map_or(false, |a| self == a)
	}
}

impl<R: Serialize> ResourceSerialize for R {
	fn to_raw_state(&self) -> Result<serde_json::Value> {
		serde_json::to_value(self).map_err(Into::into)
	}
}

/// A trait representing a builder for a provider, which is responsible for
/// initializing the provider and setting up the initial state.
#[async_trait]
pub trait ProviderBuilder {
	/// Asynchronously builds the provider.
	///
	/// This method is called when the provider is being initialized and should
	/// contain any logic necessary for setting up the provider.
	async fn build(&mut self) -> Result<()>;
}

/// A trait representing default behavior for a provider.
/// This is implemented automatically by the macros.
pub trait ProviderDefault {
	/// Returns the current state of the provider as an `Arc<Mutex<ProviderState>>`.
	///
	/// The state can be used to pass data between the provider and its resources.
	fn state(&mut self) -> Arc<Mutex<ProviderState>>;
	/// Builds a dynamic resource from the given URN and raw state.
	///
	/// This method is responsible for matching the URN and applying the current
	/// JSON value from the state to the correct resource.
	fn build_resource(
		&self,
		urn: &Rc<Urn>,
		state: &Rc<RefCell<Value>>,
	) -> Result<Rc<RefCell<dyn Resource>>>;
}

/// A trait representing a provider in the Mashin SDK.
///
/// A provider is responsible for managing resources and their lifecycle.
#[async_trait]
pub trait Provider: ProviderBuilder + ProviderDefault {}

/// Merges two JSON values, deeply combining them into a single JSON value.
///
/// If both input values are JSON objects, their key-value pairs are merged
/// recursively. In case of a key collision, the value from `b` is used.
/// For all other JSON value types, the value from `b` simply replaces the value in `a`.
///
/// ### Arguments
///
/// * `a` - The mutable reference to the first JSON value to be merged
/// * `b` - The reference to the second JSON value to be merged
///
/// ### Examples
///
/// ```
/// use serde_json::json;
/// use mashin_sdk::merge_json;
///
/// let mut a = json!({
///     "name": "Alice",
///     "age": 30,
///     "nested": {
///         "a": 1,
///         "b": 2
///     }
/// });
///
/// let b = json!({
///     "age": 31,
///     "city": "New York",
///     "nested": {
///         "b": 3,
///         "c": 4
///     }
/// });
///
/// merge_json(&mut a, &b);
///
/// assert_eq!(a, json!({
///     "name": "Alice",
///     "age": 31,
///     "city": "New York",
///     "nested": {
///         "a": 1,
///         "b": 3,
///         "c": 4
///     }
/// }));
/// ```
pub fn merge_json(a: &mut serde_json::Value, b: &serde_json::Value) {
	match (a, b) {
		(&mut serde_json::Value::Object(ref mut a), serde_json::Value::Object(b)) => {
			for (k, v) in b {
				merge_json(a.entry(k.clone()).or_insert(serde_json::Value::Null), v);
			}
		},
		(a, b) => {
			*a = b.clone();
		},
	}
}
