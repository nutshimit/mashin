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
 *   see LICENSE for license details.                       *
 *                                                          *
\* ---------------------------------------------------------*/

pub use crate::{
	backend::BackendState,
	client::{
		ExecutedResource, ExecutedResources, MashinEngine, RegisteredProvider, RegisteredProviders,
	},
	ffi::{DynamicLibraryResource, ForeignFunction, NativeType, NativeValue, Symbol},
	state::{EncryptedState, FileState, ProjectState, RawState, StateHandler},
};
pub use mashin_sdk as sdk;
pub(crate) use sdk::Result;

mod backend;
mod client;
pub mod colors;
mod ffi;
pub mod mashin_dir;
mod state;

#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: "mashin::core",
            $patter $(, $values)*
		)
	};
}

#[derive(Clone)]
pub struct StateInner {
	pub get_symbol: Symbol,
	pub save_symbol: Symbol,
}
