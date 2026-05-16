//! Companion to `fail/non_uuid_pk_with_default_manager.rs`: a non-Uuid PK
//! is fine as long as the caller opts out of the auto-generated manager via
//! `manager = false` (and provides — or simply omits — their own
//! `BaseUserManager` implementation). The PK type itself is unrestricted in
//! that mode. See issue #4455.

use chrono::{DateTime, Utc};
use reinhardt_auth::Argon2Hasher;
use reinhardt_macros::user;
use serde::{Deserialize, Serialize};

#[user(hasher = Argon2Hasher, username_field = "username", manager = false)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct I64PkManagerFalseUser {
	pub id: i64,
	pub username: String,
	pub password_hash: Option<String>,
	pub last_login: Option<DateTime<Utc>>,
	pub is_active: bool,
	pub is_superuser: bool,
}

fn main() {
	use reinhardt_auth::{AuthIdentity, BaseUser};

	let user = I64PkManagerFalseUser {
		id: 42,
		username: "alice".to_string(),
		password_hash: None,
		last_login: None,
		is_active: true,
		is_superuser: false,
	};

	assert_eq!(user.id(), "42");
	assert_eq!(user.get_username(), "alice");
}
