use crate::{
    backend::BackendState,
    state::{derive_key, StateDiff},
    DynamicLibraryResource, NativeValue, RawState, Result,
};
use deno_core::Resource;
use mashin_sdk::{ResourceAction, ResourceArgs, ResourceDiff, Urn};
use serde_json::Value;
use sodiumoxide::crypto::{pwhash::Salt, secretbox};
use std::{
    cell::{RefCell, RefMut},
    collections::{BTreeMap, HashMap},
    ffi::c_void,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Clone)]
pub struct ResourceJob {
    diff: StateDiff,
}

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
        self.resources
            .iter()
            .filter_map(|(_, s)| s.required_change.clone())
            .collect()
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
        // doing some checkup here, so we dont have to borrow the both state, so they can be dropped from here
        // as we only need the diff state and the next action needed
        let (required_change, diff) = if current_state.is_null() {
            (Some(ResourceAction::Create), None)
        } else if current_state.inner() == new_state.inner() {
            (None, None)
        } else {
            let diff = new_state.compare_with(&current_state);
            (
                Some(ResourceAction::Update {
                    diff: Rc::new(diff.provider_resource_diff()),
                }),
                Some(diff),
            )
        };

        ExecutedResource {
            provider: provider_name,
            diff,
            required_change,
        }
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
