use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Glue {
	pub name: String,
	pub version: String,
	pub repository: String,
	pub type_defs: HashMap<String, TsType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsType {
	pub doc: String,
	pub name: String,
	pub typescript: String,
	pub mashin_ty: InternalMashinType,
	pub is_enum: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InternalMashinType {
	ProviderConfig,
	ResourceConfig,
	Resource(String),
	Extra,
}
