//! Default `manager = true` combined with a non-Uuid primary key MUST be
//! rejected at macro-expansion time. The auto-generated in-memory manager
//! keys its `HashMap` by the PK, and only `Uuid` / `Option<Uuid>` get the
//! `Uuid::now_v7()` re-seed; any other PK type (e.g. `i64`) would silently
//! overwrite previous users in the map. See issue #4455.

use chrono::{DateTime, Utc};
use reinhardt_auth::Argon2Hasher;
use reinhardt_macros::user;
use serde::{Deserialize, Serialize};

#[user(hasher = Argon2Hasher, username_field = "username")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct I64PkDefaultManagerUser {
	pub id: i64,
	pub username: String,
	pub password_hash: Option<String>,
	pub last_login: Option<DateTime<Utc>>,
	pub is_active: bool,
	pub is_superuser: bool,
}

fn main() {}
