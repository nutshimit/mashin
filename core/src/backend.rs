use crate::{
    mashin_dir::{self, MashinDir},
    EncryptedState, FileState, Result, StateHandler, StateInner,
};
use mashin_sdk::Urn;
use std::collections::BTreeSet;

pub enum BackendState {
    Local(FileState),
    Plugin(StateInner),
}

impl BackendState {
    pub fn new(mashin_dir: &MashinDir) -> Result<Self> {
        Ok(Self::Local(FileState::new(mashin_dir.state_folder_path())?))
    }
    pub fn save(&self, urn: &Urn, state: &EncryptedState) -> Result<()> {
        match self {
            BackendState::Local(local) => local.save(&urn, state),
            BackendState::Plugin(_) => todo!(),
        }
    }

    pub fn get(&self, urn: &Urn) -> Result<Option<EncryptedState>> {
        match self {
            BackendState::Local(local) => local.get(&urn),
            BackendState::Plugin(_) => todo!(),
        }
    }

    pub fn resources(&self) -> Result<BTreeSet<Urn>> {
        match self {
            BackendState::Local(local) => local.resources(),
            BackendState::Plugin(_) => todo!(),
        }
    }

    pub fn delete(&self, urn: &Urn) -> Result<()> {
        match self {
            BackendState::Local(local) => local.delete(urn),
            BackendState::Plugin(_) => todo!(),
        }
    }
}
