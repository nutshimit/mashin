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
 *  This file is licensed as MIT. See LICENSE for details.  *
 *                                                          *
\* ---------------------------------------------------------*/

use std::{
	any::{type_name, Any, TypeId},
	collections::BTreeMap,
};

/// `ProviderState` is a storage container for provider-specific data that can be shared between
/// provider and resource instances. It allows providers to store and access data such as API
/// clients or other shared information.
///
/// The implementation is inspired by and based on the `GothamState` from the Deno project.
/// See <https://github.com/denoland/deno/blob/main/core/gotham_state.rs> for the original code.
///
/// `ProviderState` stores data in a type-safe manner using the `TypeId` of the stored value.
/// It supports storing one value for each type, and provides methods for inserting, borrowing,
/// borrowing mutably, and taking ownership of values.
///
/// # Examples
///
/// ```no_run
/// use mashin_sdk::ProviderState;
///
/// struct ApiClient {
///     // ...
/// }
///
/// let mut state = ProviderState::default();
/// state.put(ApiClient { /* ... */ });
///
/// // Accessing the stored ApiClient instance
/// let api_client: &ApiClient = state.borrow();
/// ```
///
/// In this example, an `ApiClient` instance is stored in a `ProviderState` and later borrowed for use.
///
#[derive(Debug, Default)]
pub struct ProviderState {
	data: BTreeMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ProviderState {
	/// Puts a value into the `State` storage. One value of each type is retained.
	/// Successive calls to `put` will overwrite the existing value of the same
	/// type.
	pub fn put<T: 'static + Send + Sync>(&mut self, t: T) {
		let type_id = TypeId::of::<T>();
		//trace!(" inserting record to state for type_id `{:?}`", type_id);
		self.data.insert(type_id, Box::new(t));
	}

	/// Determines if the current value exists in `State` storage.
	pub fn has<T: 'static>(&self) -> bool {
		let type_id = TypeId::of::<T>();
		self.data.get(&type_id).is_some()
	}

	/// Tries to borrow a value from the `State` storage.
	#[allow(clippy::should_implement_trait)]
	pub fn try_borrow<T: 'static>(&self) -> Option<&T> {
		let type_id = TypeId::of::<T>();
		//trace!(" borrowing state data for type_id `{:?}`", type_id);
		self.data.get(&type_id).and_then(|b| b.downcast_ref())
	}

	/// Borrows a value from the `State` storage.
	#[allow(clippy::should_implement_trait)]
	pub fn borrow<T: 'static>(&self) -> &T {
		self.try_borrow().unwrap_or_else(|| missing::<T>())
	}

	/// Tries to mutably borrow a value from the `State` storage.
	#[allow(clippy::should_implement_trait)]
	pub fn try_borrow_mut<T: 'static>(&mut self) -> Option<&mut T> {
		let type_id = TypeId::of::<T>();
		//trace!(" mutably borrowing state data for type_id `{:?}`", type_id);
		self.data.get_mut(&type_id).and_then(|b| b.downcast_mut())
	}

	/// Mutably borrows a value from the `State` storage.
	#[allow(clippy::should_implement_trait)]
	pub fn borrow_mut<T: 'static>(&mut self) -> &mut T {
		self.try_borrow_mut().unwrap_or_else(|| missing::<T>())
	}

	/// Tries to move a value out of the `State` storage and return ownership.
	pub fn try_take<T: 'static>(&mut self) -> Option<T> {
		let type_id = TypeId::of::<T>();
		//trace!(
		//    " taking ownership from state data for type_id `{:?}`",
		//    type_id
		//);
		self.data.remove(&type_id).and_then(|b| b.downcast().ok()).map(|b| *b)
	}

	/// Moves a value out of the `State` storage and returns ownership.
	///
	/// # Panics
	///
	/// If a value of type `T` is not present in `State`.
	pub fn take<T: 'static>(&mut self) -> T {
		self.try_take().unwrap_or_else(|| missing::<T>())
	}
}

fn missing<T: 'static>() -> ! {
	panic!("required type {} is not present in State container", type_name::<T>());
}
