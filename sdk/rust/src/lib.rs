pub use crate::urn::Urn;
pub use anyhow::Result;
use async_trait::async_trait;
pub use build::build;
pub use deserialize::deserialize_state_field;
pub use logger::CliLogger;
pub use mashin_macro::provider;
pub use provider_state::ProviderState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{any::Any, cell::RefCell, rc::Rc};

mod build;
mod deserialize;
mod logger;
mod provider_state;
mod urn;

pub const KEY_CONFIG: &str = "__config";
pub const KEY_URN: &str = "__urn";
pub const KEY_NAME: &str = "__name";
pub const KEY_SENSITIVE: &str = "__sensitive";
// keys to skip
pub const KEYS_CORE: [&str; 1] = [KEY_SENSITIVE];
pub const KEY_VALUE: &str = "__value";

// re-export some helpers
pub mod ext {
	pub use anyhow;
	pub use async_trait;
	pub use serde;
	pub use serde_json;
	pub use tokio;
}

pub type ResourceId = u32;

#[derive(Debug)]
pub struct ResourceArgs {
	pub action: Rc<ResourceAction>,
	pub urn: Rc<Urn>,
	pub raw_config: Rc<Value>,
	pub raw_state: Rc<RefCell<Value>>,
}

#[derive(Default, Clone, Debug, PartialEq)]
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceDiff(Vec<String>);

impl ResourceDiff {
	pub fn new(diff: Vec<String>) -> Self {
		Self(diff)
	}

	pub fn has_change(&self, key: impl ToString) -> bool {
		self.0.contains(&key.to_string())
	}
}

#[derive(Debug, Clone)]
pub struct ResourceResult(serde_json::Value);

impl ResourceResult {
	pub fn new(raw_state_as_json: serde_json::Value) -> Self {
		ResourceResult(raw_state_as_json)
	}

	pub fn inner(&self) -> serde_json::Value {
		self.0.clone()
	}
}

pub trait ResourceEq {
	fn as_any(&self) -> &dyn Any;
	fn is_eq(&self, other: &dyn Resource) -> bool;
}

pub trait ResourceSerialize {
	fn to_raw_state(&self) -> Result<serde_json::Value>;
}

pub trait ResourceDefault {
	fn __default_with_params(name: &str, urn: &str) -> Self
	where
		Self: Sized;

	fn __set_config_from_value(&mut self, config: &Rc<Value>);

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
			Ok(Rc::new(RefCell::new(Self::__default_with_params(name, urn))))
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

	fn name(&self) -> &str;
	fn urn(&self) -> &str;
}

#[async_trait]
pub trait Resource: ResourceEq + ResourceSerialize + ResourceDefault {
	async fn get(&mut self, provider_state: &ProviderState) -> Result<()>;
	async fn create(&mut self, provider_state: &ProviderState) -> Result<()>;
	async fn delete(&mut self, provider_state: &ProviderState) -> Result<()>;
	async fn update(&mut self, provider_state: &ProviderState, diff: &ResourceDiff) -> Result<()>;
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

pub trait Config {}

#[async_trait]
pub trait ProviderBuilder {
	async fn build(&mut self) -> Result<()>;
}

pub trait ProviderDefault {
	fn state(&mut self) -> &mut Box<ProviderState>;
	fn state_as_ref(&self) -> &ProviderState;
	fn __from_current_state(
		&self,
		urn: &Rc<Urn>,
		state: &Rc<RefCell<Value>>,
	) -> Result<Rc<RefCell<dyn Resource>>>;
}

#[async_trait]
pub trait Provider: ProviderBuilder + ProviderDefault + Send + Sync {}

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
