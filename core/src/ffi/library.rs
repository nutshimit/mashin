use super::symbol::{NativeType, Symbol};
use crate::{log, NativeValue, Result};
use anyhow::anyhow;
use deno_core::Resource;
use dlopen::raw::Library;
use mashin_sdk::ResourceResult;
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap, ffi::c_void, rc::Rc};

pub struct DynamicLibraryResource {
	pub lib: Library,
	pub symbols: HashMap<String, Box<Symbol>>,
}

impl Drop for DynamicLibraryResource {
	fn drop(&mut self) {
		log!(trace, "Drop `DynamicLibraryResource`");
	}
}

impl DynamicLibraryResource {
	pub fn call_new(&self, props_ptr: *mut c_void) -> Result<*mut c_void> {
		let symbol = self.symbols.get("new").ok_or(anyhow!("valid `drop` symbol"))?;

		let logger_ptr = Box::into_raw(Box::new(log::logger())) as *mut c_void;

		let provider_pointer = unsafe {
			let call_args = vec![
				NativeValue { pointer: logger_ptr }.as_arg(symbol.parameter_types.get(0).unwrap()),
				NativeValue { pointer: props_ptr }.as_arg(symbol.parameter_types.get(1).unwrap()),
			];
			symbol.cif.call::<*mut c_void>(symbol.ptr, &call_args)
		};

		Ok(provider_pointer)
	}

	pub fn call_drop(&self, provider_ptr: *mut c_void) -> Result<()> {
		let symbol = self.symbols.get("drop").ok_or(anyhow!("valid `drop` symbol"))?;

		unsafe {
			let call_args = vec![NativeValue { pointer: provider_ptr }
				.as_arg(symbol.parameter_types.get(0).unwrap())];
			symbol.cif.call::<()>(symbol.ptr, &call_args);
		};

		Ok(())
	}

	pub fn call_resource(
		&self,
		provider_ptr: *mut c_void,
		args_ptr: *mut c_void,
	) -> Result<*mut ResourceResult> {
		let symbol = self.symbols.get("run").ok_or(anyhow!("valid `run` symbol"))?;

		let res = unsafe {
			let call_args = vec![
				NativeValue { pointer: provider_ptr }.as_arg(&NativeType::Pointer),
				NativeValue { pointer: args_ptr }.as_arg(&NativeType::Pointer),
			];
			symbol.cif.call::<*mut ResourceResult>(symbol.ptr, &call_args)
		};

		Ok(res)
	}
}

impl Resource for DynamicLibraryResource {
	fn name(&self) -> Cow<str> {
		"dynamicLibrary".into()
	}

	fn close(self: Rc<Self>) {
		drop(self)
	}
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ForeignFunction {
	pub name: Option<String>,
	pub parameters: Vec<NativeType>,
	pub result: NativeType,
}
