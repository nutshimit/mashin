use crate::{state::derive_key, Result};
use sodiumoxide::crypto::{pwhash::Salt, secretbox};
use std::ffi::c_void;

/// Instance of a single client for an Mashin consumer.
#[derive(Clone)]
pub struct Client {
    pub state_handler: *mut c_void,
    pub key: secretbox::Key,
}

impl Client {
    pub async fn new(state_handler: *mut c_void, passphrase: &[u8]) -> Result<Self> {
        // FIXME: use dynamic salt
        let salt = Salt([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]);

        let key = derive_key(passphrase, salt)?;

        Ok(Self { state_handler, key })
    }
}
