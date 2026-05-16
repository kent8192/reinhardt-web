//! Confirms that `#[user(...)]` auto-generates `<Name>Manager` by default
//! and that the generated manager implements `BaseUserManager<Name>`.

use chrono::{DateTime, Utc};
use reinhardt_auth::Argon2Hasher;
use reinhardt_macros::user;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[user(hasher = Argon2Hasher, username_field = "email")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MinimalManagedUser {
	pub id: Uuid,
	pub email: String,
	pub password_hash: Option<String>,
	pub last_login: Option<DateTime<Utc>>,
	pub is_active: bool,
	pub is_superuser: bool,
}

// Compile-time proof that `<Name>Manager` exists and impls `BaseUserManager`.
fn _assert_manager_satisfies_trait()
where
	MinimalManagedUserManager: reinhardt_auth::BaseUserManager<MinimalManagedUser>,
{
}

fn main() {
	use reinhardt_auth::BaseUserManager;

	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();

	let mut mgr = MinimalManagedUserManager::new();
	let user = rt
		.block_on(mgr.create_user(
			"alice@example.com",
			Some("hunter2"),
			HashMap::new(),
		))
		.expect("create_user succeeds");

	assert_eq!(user.email, "alice@example.com");
	assert!(user.is_active);
	assert!(!user.is_superuser);
	assert!(user.password_hash.is_some());

	let su = rt
		.block_on(mgr.create_superuser(
			"root@example.com",
			Some("rootpw"),
			HashMap::new(),
		))
		.expect("create_superuser succeeds");
	assert!(su.is_superuser);
}
