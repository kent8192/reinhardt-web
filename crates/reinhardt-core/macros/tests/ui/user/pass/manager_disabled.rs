//! `#[user(..., manager = false)]` suppresses generation of the
//! `<Name>Manager` struct. The test passes by *not* referencing any auto-
//! generated manager — only the existing trait impls (`BaseUser`,
//! `AuthIdentity`) are required to compile.

use chrono::{DateTime, Utc};
use reinhardt_auth::Argon2Hasher;
use reinhardt_macros::user;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[user(hasher = Argon2Hasher, username_field = "email", manager = false)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmanagedUser {
	pub id: Uuid,
	pub email: String,
	pub password_hash: Option<String>,
	pub last_login: Option<DateTime<Utc>>,
	pub is_active: bool,
	pub is_superuser: bool,
}

fn main() {
	use reinhardt_auth::{AuthIdentity, BaseUser};

	let mut user = UnmanagedUser {
		id: Uuid::now_v7(),
		email: "test@example.com".to_string(),
		password_hash: None,
		last_login: None,
		is_active: true,
		is_superuser: false,
	};

	assert_eq!(user.get_username(), "test@example.com");
	user.set_password("test123").unwrap();
	assert!(user.is_authenticated());

	// Note: `UnmanagedUserManager` is NOT generated because `manager = false`.
	// A companion compile-fail test (`fail/manager_disabled_no_manager.rs`)
	// proves that referencing it triggers a compile error.
}
