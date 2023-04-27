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
 *   see LICENSE-* for license details.                     *
 *                                                          *
\* ---------------------------------------------------------*/

use crate::{
	sdk::{
		ext::{anyhow::bail, async_trait::async_trait, serde_json},
		Result, Urn,
	},
	EncryptedState, StateHandler,
};
use rkv::{
	backend::{SafeMode, SafeModeEnvironment},
	Manager, Rkv, StoreOptions,
};
use std::{
	collections::BTreeSet,
	fs,
	path::PathBuf,
	sync::{Arc, RwLock},
};

pub struct FileState {
	path: PathBuf,
	db: Arc<RwLock<Rkv<SafeModeEnvironment>>>,
}

impl std::fmt::Debug for FileState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("FileState")
			.field("path", &self.path)
			.field("db", &self.db)
			.finish()
	}
}

impl FileState {
	pub fn new(db_path: PathBuf) -> Result<Self> {
		fs::create_dir_all(&db_path)?;

		let mut manager = Manager::<SafeModeEnvironment>::singleton().write().unwrap();
		let db = manager.get_or_create(db_path.as_path(), Rkv::new::<SafeMode>).unwrap();
		Ok(Self { db, path: db_path })
	}
}

#[async_trait]
impl StateHandler for FileState {
	fn get(&self, urn: &Urn) -> Result<Option<EncryptedState>> {
		let env = self.db.read().or_else(|_| bail!("unable to get env"))?;
		let store = env.open_single("state", StoreOptions::create())?;
		let reader = env.read()?;

		Ok(match store.get(&reader, urn)? {
			Some(current_value) => match current_value {
				rkv::Value::Str(current_value) | rkv::Value::Json(current_value) =>
					Some(serde_json::from_str::<EncryptedState>(current_value)?),
				rkv::Value::Blob(current_value) =>
					Some(serde_json::from_slice::<EncryptedState>(current_value)?),
				_ => None,
			},
			None => None,
		})
	}

	fn save(&self, urn: &Urn, state: &EncryptedState) -> Result<()> {
		let env = self.db.read().or_else(|_| bail!("unable to get env"))?;
		let store = env.open_single("state", StoreOptions::create())?;
		let mut writer = env.write()?;

		let raw_json = serde_json::to_string(state)?;
		let json_value = rkv::Value::Str(&raw_json);

		store.put(&mut writer, urn, &json_value)?;
		writer.commit().map_err(Into::into)
	}

	fn resources(&self) -> Result<BTreeSet<Urn>> {
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

	fn delete(&self, urn: &Urn) -> Result<()> {
		let env = self.db.read().or_else(|_| bail!("unable to get env"))?;
		let store = env.open_single("state", StoreOptions::create())?;
		let mut writer = env.write()?;

		store.delete(&mut writer, urn)?;

		writer.commit().map_err(Into::into)
	}
}
