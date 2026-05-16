//! `#[user(..., manager_name = MyMgr)]` emits a manager type named `MyMgr`
//! instead of the default `<Name>Manager`.

use chrono::{DateTime, Utc};
use reinhardt_auth::Argon2Hasher;
use reinhardt_macros::user;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[user(hasher = Argon2Hasher, username_field = "email", manager_name = AccountManager)]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RenamedUser {
	pub id: Uuid,
	pub email: String,
	pub password_hash: Option<String>,
	pub last_login: Option<DateTime<Utc>>,
	pub is_active: bool,
	pub is_superuser: bool,
}

fn _assert_renamed_manager()
where
	AccountManager: reinhardt_auth::BaseUserManager<RenamedUser>,
{
}

fn main() {
	use reinhardt_auth::BaseUserManager;

	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();

	let mut mgr = AccountManager::new();
	let user = rt
		.block_on(mgr.create_user("bob@example.com", None, HashMap::new()))
		.expect("create_user succeeds");
	assert_eq!(user.email, "bob@example.com");
}
