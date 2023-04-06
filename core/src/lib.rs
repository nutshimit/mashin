mod client;
mod config;
mod state;
use std::{collections::HashMap, ffi::c_void, rc::Rc};

pub use crate::{
    client::Client,
    state::{EncryptedState, ProjectState, RawState, StateHandler},
};
pub use mashin_sdk as sdk;
pub(crate) use sdk::Result;

pub use mashin_ffi::Symbol;

#[derive(Clone)]
pub struct ProviderInner {
    pub name: String,
    pub provider: *mut c_void,
    pub drop_fn: Symbol,
}

#[derive(Clone)]
pub struct StateInner {
    pub get_symbol: Symbol,
    pub save_symbol: Symbol,
}

pub type ProviderList = HashMap<sdk::ResourceId, ProviderInner>;
