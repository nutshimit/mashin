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

use deno_core::resolve_path;
use std::{env::current_dir, path::PathBuf};

#[derive(Debug, Clone, Default)]
pub struct MashinDir {
	root: PathBuf,
}

impl MashinDir {
	pub fn new(maybe_custom_root: Option<PathBuf>) -> std::io::Result<Self> {
		let root: PathBuf = if let Some(root) = maybe_custom_root {
			root
		} else {
			resolve_path(".mashin", current_dir().expect("valid current dir").as_path())
				.expect("valid path")
				.to_file_path()
				.expect("valid local path")
		};
		assert!(root.is_absolute());

		let mashin_dir = Self { root };
		Ok(mashin_dir)
	}
	pub fn deps_folder_path(&self) -> PathBuf {
		self.root.join("deps")
	}
	pub fn state_folder_path(&self) -> PathBuf {
		self.root.join("state")
	}
}
