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

use super::symbol::{NativeType, Symbol};
use crate::{log, NativeValue, Result};
use anyhow::anyhow;
use deno_core::Resource;
use dlopen::raw::Library;
use mashin_sdk::{ResourceArgs, ResourceResult};
use serde::Deserialize;
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap, ffi::c_void, mem, ptr, rc::Rc, slice};

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
	pub fn call_new(&self, props: &Value) -> Result<*mut c_void> {
		let symbol = self.symbols.get("new").ok_or(anyhow!("valid `drop` symbol"))?;

		let logger_ptr = Box::into_raw(Box::new(log::logger())) as *mut c_void;
		let (props_ptr, props_length) = {
			let json = serde_json::to_string(&props)?;
			let encoded = json.into_bytes();
			let length = encoded.len();
			let ret = encoded.as_ptr();
			::std::mem::forget(encoded);
			(ret, length)
		};

		let provider_pointer = unsafe {
			symbol.cif.call::<*mut c_void>(
				symbol.ptr,
				&[
					NativeValue { pointer: logger_ptr }.as_arg(&NativeType::Pointer),
					NativeValue { pointer: props_ptr as *mut c_void }.as_arg(&NativeType::Pointer),
					NativeValue { usize_value: props_length }.as_arg(&NativeType::USize),
				],
			)
		};

		unsafe {
			ptr::drop_in_place(props_ptr as *mut u8);
		};

		Ok(provider_pointer)
	}

	pub fn call_drop(&self, provider_ptr: *mut c_void) -> Result<()> {
		let symbol = self.symbols.get("drop").ok_or(anyhow!("valid `drop` symbol"))?;

		unsafe {
			symbol.cif.call::<()>(
				symbol.ptr,
				&[NativeValue { pointer: provider_ptr }.as_arg(&NativeType::Pointer)],
			);
		};

		Ok(())
	}

	pub fn call_resource(
		&self,
		provider_ptr: *mut c_void,
		args: &ResourceArgs,
	) -> Result<ResourceResult> {
		let symbol = self.symbols.get("run").ok_or(anyhow!("valid `run` symbol"))?;

		let (args_ptr, args_length) = {
			let json = serde_json::to_string(&args)?;
			let encoded = json.into_bytes();
			let length = encoded.len();
			let ret = encoded.as_ptr();
			mem::forget(encoded);
			(ret, length)
		};

		let res_ptr = unsafe {
			symbol.cif.call::<*const u8>(
				symbol.ptr,
				&[
					NativeValue { pointer: provider_ptr }.as_arg(&NativeType::Pointer),
					NativeValue { pointer: args_ptr as *mut c_void }.as_arg(&NativeType::Pointer),
					NativeValue { usize_value: args_length }.as_arg(&NativeType::USize),
				],
			)
		};

		// we extract the 4 first bytes (u32) that represent the lenght of our result slice
		let size_of_key = std::mem::size_of::<u32>();
		let out: &mut [u8; 4] = &mut [0, 0, 0, 0];
		unsafe { ptr::copy::<u8>(res_ptr, out.as_mut_ptr(), size_of_key) };
		let args_length = u32::from_be_bytes(*out) as usize;

		// rebuild the slice from the offset and the args length
		let buf = unsafe { slice::from_raw_parts(res_ptr.add(size_of_key), args_length) };

		unsafe {
			ptr::drop_in_place(res_ptr as *mut c_void);
			ptr::drop_in_place(args_ptr as *mut c_void)
		};

		serde_json::from_slice(buf).map_err(Into::into)
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
