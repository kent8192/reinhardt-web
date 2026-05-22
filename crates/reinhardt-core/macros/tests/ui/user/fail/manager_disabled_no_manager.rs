//! When `manager = false`, the `<Name>Manager` struct must NOT be emitted.
//! Referencing it should fail to compile.

use chrono::{DateTime, Utc};
use reinhardt_auth::Argon2Hasher;
use reinhardt_macros::user;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[user(hasher = Argon2Hasher, username_field = "email", manager = false)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OptOutUser {
	pub id: Uuid,
	pub email: String,
	pub password_hash: Option<String>,
	pub last_login: Option<DateTime<Utc>>,
	pub is_active: bool,
	pub is_superuser: bool,
}

fn main() {
	let _ = OptOutUserManager::new();
}
