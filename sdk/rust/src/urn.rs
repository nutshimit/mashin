use crate::Result;
use anyhow::anyhow;
use std::{cmp::Ordering, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Urn(urn::Urn);

impl PartialOrd for Urn {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Urn {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.cmp(&other.0)
	}
}

impl AsRef<[u8]> for Urn {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl Urn {
	pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
		let bytes_str = std::str::from_utf8(bytes)?;
		let urn = urn::Urn::from_str(bytes_str)?;
		Ok(Self(urn))
	}

	pub fn as_provider(&self) -> Result<String> {
		let nss: Vec<&str> = self.nss().split(':').collect();
		Ok(nss.first().ok_or(anyhow!("invalid provider")).cloned()?.to_string())
	}

	pub fn as_display(&self) -> String {
		self.to_string().replace("urn:provider:", "")
	}

	pub fn as_str(&self) -> &str {
		self.0.as_str()
	}

	/// NID (Namespace identifier), the first part of the URN.
	///
	/// For example, in `urn:ietf:rfc:2648`, `ietf` is the namespace.
	pub fn nid(&self) -> &str {
		self.0.nid()
	}

	/// NSS (Namespace-specific string) identifying the resource.
	///
	/// For example, in `urn:ietf:rfc:2648`, `rfs:2648` is the NSS.
	pub fn nss(&self) -> &str {
		self.0.nss()
	}

	/// q-component, following the `?=` character sequence. Has a similar function to the URL query
	/// string.
	///
	/// In `urn:example:weather?=op=map&lat=39.56&lon=-104.85`,
	/// the q-component is `op=map&lat=39.56&lon=-104.85`.
	///
	/// Should not be used for equivalence checks.
	pub fn q_component(&self) -> Option<&str> {
		self.0.q_component()
	}
}

impl FromStr for Urn {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Urn(urn::Urn::from_str(s)?))
	}
}

impl std::fmt::Display for Urn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
