mod diff;
mod file_state;
mod state_handler;
mod trim_sensitive;

pub use diff::{diff, StateDiff};
pub use file_state::FileState;
pub(crate) use state_handler::derive_key;
pub use state_handler::{EncryptedState, ProjectState, RawState, StateHandler};
