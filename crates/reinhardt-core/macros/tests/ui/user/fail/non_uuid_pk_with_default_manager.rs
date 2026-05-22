//! Default `manager = true` combined with a non-Uuid primary key MUST be
//! rejected at macro-expansion time. The auto-generated in-memory manager
//! keys its `HashMap` by the PK, and only `Uuid` / `Option<Uuid>` get the
//! `Uuid::now_v7()` re-seed; any other PK type (e.g. `i64`) would silently
//! overwrite previous users in the map. See issue #4455.

// Defensive allow attributes to guarantee the `.stderr` golden stays a
// single-error block across future rustc / trybuild updates. The fixture
// is rejected by the `#[user(...)]` macro before any item-level usage is
// considered, so every import and field would otherwise produce a
// trailing warning block (issue #4552).
#![allow(unused_imports, dead_code)]

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
