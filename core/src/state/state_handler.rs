use super::{diff::compare_raw_state_recursive, trim_sensitive};
use crate::{
    sdk::{
        ext::{
            anyhow::anyhow,
            async_trait::async_trait,
            serde::{
                de::{self, Visitor},
                Deserialize, Deserializer, Serialize, Serializer,
            },
            serde_json::Value,
        },
        Urn,
    },
    Result,
};
use base64::{engine::general_purpose, Engine as _};
use mashin_sdk::ResourceDiff;
use sodiumoxide::crypto::{pwhash, secretbox};
use std::{collections::BTreeSet, fmt};

#[derive(Serialize, Deserialize)]
pub enum ProjectState {
    EncryptedState(EncryptedState),
    RawState(RawState),
}
#[derive(Debug)]
pub struct EncryptedState {
    /// I think this is a salt but idk tbh
    nonce: secretbox::Nonce,
    /// The cipher text of the encrypted JSON value.
    ciphertext: Vec<u8>,
}

impl Serialize for EncryptedState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!(
            "{}_{}",
            general_purpose::STANDARD.encode(self.nonce.as_ref()),
            general_purpose::STANDARD.encode(&self.ciphertext)
        ))
    }
}

struct EncryptedStateVisitor;

impl<'de> Visitor<'de> for EncryptedStateVisitor {
    type Value = EncryptedState;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string with two base64 encoded values separated with an underscore")
    }

    fn visit_str<E>(self, s: &str) -> core::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        let parts: Vec<&str> = s.split('_').collect();
        if parts.len() != 2 {
            return Err(E::custom(format!(
                "expected two base64 encoded values separated with an underscore, got {}",
                parts.len()
            )));
        }
        let (nonce, ciphertext) = match (
            general_purpose::STANDARD.decode(parts[0]),
            general_purpose::STANDARD.decode(parts[1]),
        ) {
            (Ok(nonce), Ok(ciphertext)) => {
                let nonce = secretbox::Nonce::from_slice(&nonce)
                    .ok_or_else(|| E::custom(String::from("nonce part was not 24 bits long")))?;
                (nonce, ciphertext)
            }
            _ => {
                return Err(E::custom(String::from(
                    "couldn't decode one of the bas64 parts",
                )))
            }
        };
        Ok(EncryptedState { nonce, ciphertext })
    }
}

impl<'de> Deserialize<'de> for EncryptedState {
    fn deserialize<D>(deserializer: D) -> Result<EncryptedState, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(EncryptedStateVisitor)
    }
}

impl EncryptedState {
    /// Serialize the passed value into a Vec<u8>, encrypt it and then wrap the
    /// ciphertext and nonce into a new instance of Self
    pub fn encrypt(value: &RawState, key: &secretbox::Key) -> Result<Self> {
        let plaintext = serde_json::to_vec(value.into())?;
        let nonce = secretbox::gen_nonce();
        Ok(Self {
            nonce,
            ciphertext: secretbox::seal(&plaintext, &nonce, key),
        })
    }

    /// Decrypt the ciphertext with libsodium's secretbox and then deserialize
    /// the plaintext.
    pub fn decrypt(&self, key: &secretbox::Key) -> Result<RawState> {
        let plaintext = secretbox::open(&self.ciphertext, &self.nonce, key)
            .map_err(|_| anyhow!("unable to decrypt value"))?;
        Ok(RawState(serde_json::from_slice(&plaintext)?))
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RawState(serde_json::Value);

impl RawState {
    pub fn encrypt(&self, key: &secretbox::Key) -> Result<EncryptedState> {
        EncryptedState::encrypt(self.into(), key)
    }

    pub fn generate_ts_output(&self) -> Value {
        trim_sensitive::trim_sensitive_fields(&self.0)
    }

    pub fn compare_with(&self, b: &RawState, path: Option<&str>, in_value: bool) -> ResourceDiff {
        ResourceDiff::new(compare_raw_state_recursive(&self.0, &b.0, path, in_value))
    }
}

impl Into<ProjectState> for EncryptedState {
    fn into(self) -> ProjectState {
        ProjectState::EncryptedState(self)
    }
}

impl Into<ProjectState> for RawState {
    fn into(self) -> ProjectState {
        ProjectState::RawState(self)
    }
}

impl Into<serde_json::Value> for RawState {
    fn into(self) -> serde_json::Value {
        self.0
    }
}

impl Into<serde_json::Value> for &RawState {
    fn into(self) -> serde_json::Value {
        self.0.clone()
    }
}

impl From<serde_json::Value> for RawState {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

#[async_trait]
pub trait StateHandler {
    async fn save(&self, urn: &Urn, state: &EncryptedState) -> Result<()>;
    async fn get(&self, urn: &Urn) -> Result<Option<EncryptedState>>;
    async fn resources(&self) -> Result<BTreeSet<Urn>>;
}

pub(crate) fn derive_key(passphrase: &[u8], salt: pwhash::Salt) -> Result<secretbox::Key> {
    let mut key = secretbox::Key([0; secretbox::KEYBYTES]);
    let secretbox::Key(ref mut key_bytes) = key;
    pwhash::derive_key_interactive(key_bytes, passphrase, &salt)
        .map_err(|_| anyhow!("unable to derive key"))?;
    Ok(key)
}
