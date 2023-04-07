use std::{any::Any, rc::Rc};

pub use crate::urn::Urn;
pub use anyhow::Result;
use async_trait::async_trait;
pub use deserialize::deserialize_state_field;
pub use mashin_macro::resource;
pub use provider_state::ProviderState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod deserialize;

mod provider_state;
mod urn;

pub const KEY_SENSITIVE: &'static str = "__sensitive";
pub const KEYS_CORE: [&'static str; 1] = [KEY_SENSITIVE];
pub const KEY_VALUE: &'static str = "__value";

// re-export some helpers
pub mod ext {
    pub use anyhow;
    pub use async_trait;
    pub use serde;
    pub use serde_json;
    pub use tokio;
}

pub type ResourceId = u32;

pub enum ResourceAction {
    Update,
    Create,
    Delete,
    Get,
}

#[derive(Debug, Clone, Default)]
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
    // An &Any can be cast to a reference to a concrete type.
    fn as_any(&self) -> &dyn Any;

    // Perform the test.
    fn is_eq(&self, other: &dyn Resource) -> bool;
}

pub trait ResourceSerialize {
    fn to_raw_state(&self) -> Result<serde_json::Value>;
}

#[async_trait]
pub trait Resource: ResourceEq + ResourceSerialize {
    fn __default_with_params(name: &str, urn: &str) -> Self
    where
        Self: Sized;

    fn __set_config_from_value(&mut self, config: &Value);

    fn from_current_state(name: &str, urn: &str, state: &Value) -> Result<Box<Self>>
    where
        Self: Default,
        for<'de> Self: Deserialize<'de>,
    {
        if state.as_null().is_some() {
            Ok(Box::new(Self::__default_with_params(name, urn)))
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

            Ok(Box::new(::serde_json::from_value::<Self>(state)?))
        }
    }

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
        other
            .as_any()
            .downcast_ref::<R>()
            .map_or(false, |a| self == a)
    }
}

impl<R: Serialize> ResourceSerialize for R {
    fn to_raw_state(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self).map_err(Into::into)
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn init(&mut self) -> Result<()>;
    fn state(&self) -> &ProviderState;
    fn __from_current_state(&self, urn: &Urn, state: &Value) -> Result<Box<dyn Resource>>;
}

pub fn merge_json(a: &mut serde_json::Value, b: &serde_json::Value) {
    match (a, b) {
        (&mut serde_json::Value::Object(ref mut a), &serde_json::Value::Object(ref b)) => {
            for (k, v) in b {
                merge_json(a.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}
