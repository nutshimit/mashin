// `path` is only used on Windows.
#[allow(unused_variables)]
pub(crate) fn format_error(e: dlopen::Error, path: String) -> String {
	match e {
		#[cfg(target_os = "windows")]
		// This calls FormatMessageW with library path
		// as replacement for the insert sequences.
		// Unlike libstd which passes the FORMAT_MESSAGE_IGNORE_INSERTS
		// flag without any arguments.
		//
		// https://github.com/denoland/deno/issues/11632
		dlopen::Error::OpeningLibraryError(e) => {
			use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
			use winapi::{
				shared::{minwindef::DWORD, winerror::ERROR_INSUFFICIENT_BUFFER},
				um::{
					errhandlingapi::GetLastError,
					winbase::{
						FormatMessageW, FORMAT_MESSAGE_ARGUMENT_ARRAY, FORMAT_MESSAGE_FROM_SYSTEM,
					},
					winnt::{LANG_SYSTEM_DEFAULT, MAKELANGID, SUBLANG_SYS_DEFAULT},
				},
			};

			let err_num = match e.raw_os_error() {
				Some(err_num) => err_num,
				// This should never hit unless dlopen changes its error type.
				None => return e.to_string(),
			};

			// Language ID (0x0800)
			let lang_id = MAKELANGID(LANG_SYSTEM_DEFAULT, SUBLANG_SYS_DEFAULT) as DWORD;

			let mut buf = vec![0; 500];

			let path =
				OsStr::new(&path).encode_wide().chain(Some(0).into_iter()).collect::<Vec<_>>();

			let arguments = [path.as_ptr()];

			loop {
				// SAFETY:
				// winapi call to format the error message
				let length = unsafe {
					FormatMessageW(
						FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_ARGUMENT_ARRAY,
						std::ptr::null_mut(),
						err_num as DWORD,
						lang_id as DWORD,
						buf.as_mut_ptr(),
						buf.len() as DWORD,
						arguments.as_ptr() as _,
					)
				};

				if length == 0 {
					// SAFETY:
					// winapi call to get the last error message
					let err_num = unsafe { GetLastError() };
					if err_num == ERROR_INSUFFICIENT_BUFFER {
						buf.resize(buf.len() * 2, 0);
						continue
					}

					// Something went wrong, just return the original error.
					return e.to_string()
				}

				let msg = String::from_utf16_lossy(&buf[..length as usize]);
				return msg
			}
		},
		_ => e.to_string(),
	}
}
