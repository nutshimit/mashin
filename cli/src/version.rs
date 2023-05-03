pub const GIT_COMMIT_HASH: &str = env!("GIT_COMMIT_HASH");

pub fn mashin() -> &'static str {
	if is_canary() {
		concat!(env!("CARGO_PKG_VERSION"), "+", env!("GIT_COMMIT_HASH_SHORT"))
	} else {
		env!("CARGO_PKG_VERSION")
	}
}

pub fn get_user_agent() -> &'static str {
	if is_canary() {
		concat!("Mashin/", env!("CARGO_PKG_VERSION"), "+", env!("GIT_COMMIT_HASH_SHORT"))
	} else {
		concat!("Mashin/", env!("CARGO_PKG_VERSION"))
	}
}

pub fn is_canary() -> bool {
	option_env!("MASHIN_CANARY").is_some()
}

pub fn release_version_or_canary_commit_hash() -> &'static str {
	if is_canary() {
		GIT_COMMIT_HASH
	} else {
		env!("CARGO_PKG_VERSION")
	}
}
