use mashin_core::{
    sdk::{
        ext::{anyhow::bail, async_trait::async_trait, serde_json},
        Result, Urn,
    },
    EncryptedState, StateHandler, StateInner,
};
use std::{
    collections::BTreeSet,
    env::current_dir,
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use rkv::backend::{SafeMode, SafeModeEnvironment};
use rkv::{Manager, Rkv, StoreOptions};

pub enum BackendState {
    Local(FileState),
    Plugin(StateInner),
}

impl BackendState {
    pub async fn save(&self, urn: &Urn, state: &EncryptedState) -> Result<()> {
        match self {
            BackendState::Local(local) => local.save(&urn, state).await,
            BackendState::Plugin(_) => todo!(),
        }
    }

    pub async fn get(&self, urn: &Urn) -> Result<Option<EncryptedState>> {
        match self {
            BackendState::Local(local) => local.get(&urn).await,
            BackendState::Plugin(_) => todo!(),
        }
    }

    pub async fn resources(&self) -> Result<BTreeSet<Urn>> {
        match self {
            BackendState::Local(local) => local.resources().await,
            BackendState::Plugin(_) => todo!(),
        }
    }
}

pub struct FileState {
    path: PathBuf,
    db: Arc<RwLock<Rkv<SafeModeEnvironment>>>,
}

impl FileState {
    pub fn new(path: PathBuf) -> Result<Self> {
        let db_path = path.join("state");
        fs::create_dir_all(&db_path)?;

        let mut manager = Manager::<SafeModeEnvironment>::singleton().write().unwrap();
        let db = manager
            .get_or_create(db_path.as_path(), Rkv::new::<SafeMode>)
            .unwrap();
        Ok(Self { db, path })
    }
}

impl std::fmt::Debug for FileState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileState")
            .field("path", &self.path)
            .field("db", &self.db)
            .finish()
    }
}

#[async_trait]
impl StateHandler for FileState {
    async fn get(&self, urn: &Urn) -> Result<Option<EncryptedState>> {
        let env = self.db.read().or_else(|_| bail!("unable to get env"))?;
        let store = env.open_single("state", StoreOptions::create())?;
        let reader = env.read()?;

        Ok(match store.get(&reader, urn)? {
            Some(current_value) => match current_value {
                rkv::Value::Str(current_value) | rkv::Value::Json(current_value) => {
                    Some(serde_json::from_str::<EncryptedState>(current_value)?)
                }
                rkv::Value::Blob(current_value) => {
                    Some(serde_json::from_slice::<EncryptedState>(current_value)?)
                }
                _ => None,
            },
            None => None,
        })
    }

    async fn save(&self, urn: &Urn, state: &EncryptedState) -> Result<()> {
        let env = self.db.read().or_else(|_| bail!("unable to get env"))?;
        let store = env.open_single("state", StoreOptions::create())?;
        let mut writer = env.write()?;

        let raw_json = serde_json::to_string(state)?;
        let json_value = rkv::Value::Str(&raw_json);

        store.put(&mut writer, urn, &json_value).unwrap();
        writer.commit().map_err(Into::into)
    }

    async fn resources(&self) -> Result<BTreeSet<Urn>> {
        let env = self.db.read().or_else(|_| bail!("unable to get env"))?;
        let store = env.open_single("state", StoreOptions::create())?;
        let reader = env.read()?;

        let mut all_resources = BTreeSet::new();
        for val in store.iter_start(&reader)? {
            let (key, _) = val?;
            all_resources.insert(Urn::try_from_bytes(key)?);
        }

        Ok(all_resources)
    }
}
