//! User model for authentication
//!
//! Implements BaseUser trait with Argon2id password hashing via the `#[user]`
//! attribute macro. Supports self-referential `ManyToManyField<User, User>`
//! relationships for following/blocking (kent8192/reinhardt-web#3651).
//!
//! Also provides a manual `impl AdminUser for User` so that `AdminSite::set_user_type::<User>()`
//! works without requiring the full `FullUser` field set (kent8192/reinhardt-web#3652).
use chrono::{DateTime, Utc};
use reinhardt::Argon2Hasher;
#[cfg(native)]
use reinhardt::admin::AdminUser;
use reinhardt::db::associations::ManyToManyField;
use reinhardt::macros::user;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(test, native))]
use sqlx::FromRow;
use uuid::Uuid;
#[user(hasher = Argon2Hasher, username_field = "email", manager = false)]
#[model(app_label = "auth", table_name = "auth_user")]
#[derive(Default, Serialize, Deserialize)]
#[cfg_attr(all(test, native), derive(FromRow))]
pub struct User {
	#[field(primary_key = true)]
	pub id: Uuid,
	#[field(max_length = 150, unique = true)]
	pub username: String,
	#[field(max_length = 255, unique = true)]
	pub email: String,
	#[field(max_length = 255)]
	pub password_hash: Option<String>,
	#[field(default = true)]
	pub is_active: bool,
	#[field(default = false)]
	pub is_superuser: bool,
	#[field(include_in_new = false)]
	pub last_login: Option<DateTime<Utc>>,
	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
	#[field(max_length = 500, null = true)]
	pub bio: Option<String>,
	#[serde(skip, default)]
	#[cfg_attr(all(test, native), sqlx(skip))]
	#[rel(many_to_many, related_name = "followers")]
	pub following: ManyToManyField<User, User>,
	#[serde(skip, default)]
	#[cfg_attr(all(test, native), sqlx(skip))]
	#[rel(many_to_many, related_name = "blocked_by")]
	pub blocked_users: ManyToManyField<User, User>,
}
#[cfg(native)]
impl AdminUser for User {
	fn is_active(&self) -> bool {
		self.is_active
	}
	fn is_staff(&self) -> bool {
		self.is_superuser
	}
	fn is_superuser(&self) -> bool {
		self.is_superuser
	}
	fn get_username(&self) -> &str {
		&self.email
	}
}
