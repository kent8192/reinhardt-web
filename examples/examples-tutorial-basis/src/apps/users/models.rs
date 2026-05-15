//! User model for the tutorial-basis example.
//!
//! Implements `BaseUser` manually (mirroring examples-twitter) to avoid the
//! `#[user]` macro path until the upstream macro fully supports plain
//! username-based users without `ManyToManyField` workarounds.

use chrono::{DateTime, Utc};
use reinhardt::prelude::*;
use reinhardt::{Argon2Hasher, BaseUser};
use serde::{Deserialize, Serialize};

#[model(app_label = "users", table_name = "users")]
#[derive(Serialize, Deserialize)]
pub struct User {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(max_length = 150, unique = true)]
	pub username: String,

	#[field(max_length = 255)]
	pub password_hash: Option<String>,

	#[field(default = true)]
	pub is_active: bool,

	#[field(include_in_new = false)]
	pub last_login: Option<DateTime<Utc>>,

	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
}

impl BaseUser for User {
	type PrimaryKey = i64;
	type Hasher = Argon2Hasher;

	fn get_username_field() -> &'static str {
		"username"
	}

	fn get_username(&self) -> &str {
		&self.username
	}

	fn password_hash(&self) -> Option<&str> {
		self.password_hash.as_deref()
	}

	fn set_password_hash(&mut self, hash: String) {
		self.password_hash = Some(hash);
	}

	fn last_login(&self) -> Option<DateTime<Utc>> {
		self.last_login
	}

	fn set_last_login(&mut self, time: DateTime<Utc>) {
		self.last_login = Some(time);
	}

	fn is_active(&self) -> bool {
		self.is_active
	}
}
