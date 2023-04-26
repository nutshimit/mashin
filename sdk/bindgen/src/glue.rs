use crate::Result;
use mashin_primitives::Glue;
use std::{
	fs::OpenOptions,
	io::{Read, Write},
	path::Path,
};

pub fn get_glue<P>(path: P) -> Result<Glue>
where
	P: AsRef<Path>,
{
	let mut file = OpenOptions::new().read(true).open(path)?;
	let mut meta = String::new();
	file.read_to_string(&mut meta)?;
	serde_json::from_str(&meta).map_err(Into::into)
}

pub fn write_mod<P>(path: P, content: String) -> Result<()>
where
	P: AsRef<Path>,
{
	let path = path.as_ref();

	let mut file = OpenOptions::new()
		.truncate(true)
		.write(true)
		.create(true)
		.open(path.join("mod.ts"))?;
	file.write_all(&content.as_bytes())?;
	Ok(())
}
