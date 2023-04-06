pub use crate::urn::Urn;
use async_trait::async_trait;
pub use deserialize::deserialize_state_field;
pub use diff::compare_json_objects_recursive;
pub use mashin_macro::resource;
pub use provider_state::ProviderState;

mod deserialize;
mod diff;
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

pub use anyhow::Result;
pub type ResourceId = u32;

pub enum ResourceAction {
    Update,
    Create,
    Delete,
    Get,
}

#[async_trait]
pub trait Resource {
    async fn get(&mut self, provider_state: &ProviderState) -> Result<bool>;
    async fn create(&mut self, provider_state: &ProviderState) -> Result<()>;
    async fn delete(&mut self, provider_state: &ProviderState) -> Result<()>;
    async fn update(&mut self, provider_state: &ProviderState) -> Result<()>;
}

#[async_trait]
pub trait Provider: Send + Sync {
    //fn new(runtime: &tokio::runtime::Handle) -> Self;
    async fn init(&mut self) -> Result<()>;
    fn state(&self) -> &ProviderState;
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
