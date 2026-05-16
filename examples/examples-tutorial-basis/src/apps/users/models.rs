//! User model for the tutorial-basis example.
//!
//! Uses the `#[user]` attribute macro to derive `BaseUser`,
//! `PermissionsMixin`, and `AuthIdentity` implementations from the
//! conventional field set (`username`, `password_hash`, `last_login`,
//! `is_active`, `is_superuser`). `full = true` is intentionally **not**
//! enabled — the tutorial keeps the model minimal (no `email` /
//! `first_name` / `last_name` / `date_joined` / `is_staff`), which keeps
//! both the schema and the SignupForm small.
//!
//! All registration / authentication-state changes go through
//! [`UserManager`] (a project-local implementation of
//! `BaseUserManager<User>`) rather than constructing `User` instances
//! by hand — see the `register` server function in
//! `crate::apps::users::server_fn`.

use chrono::{DateTime, Utc};
use reinhardt::Argon2Hasher;
use reinhardt::macros::user;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

// `manager = false` opts out of the auto-generated `UserManager` that
// `#[user(...)]` emits by default since reinhardt-web#4451 — the tutorial
// keeps its own DB-backed [`UserManager`] below (registered via
// `#[injectable_factory]`) which would otherwise be shadowed. The
// auto-manager is also gated to `Uuid` / `Option<Uuid>` primary keys
// (issue #4455), and this model uses `i64` to demonstrate auto-increment
// integer PKs in the tutorial.
#[user(hasher = Argon2Hasher, username_field = "username", manager = false)]
#[model(app_label = "users", table_name = "users")]
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct User {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(max_length = 150, unique = true)]
	pub username: String,

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
}

#[cfg(native)]
mod manager {
	use super::User;
	use reinhardt::BaseUser;
	use reinhardt::DatabaseConnection;
	use reinhardt::Model;
	use reinhardt::core::async_trait;
	use reinhardt::core::exception::Error;
	use reinhardt::db::orm::{FilterOperator, FilterValue};
	use reinhardt::di::{Depends, injectable_factory};
	// `BaseUserManager` lives in `reinhardt-auth` and is not yet re-exported
	// at the top level of `reinhardt`; reach it via the doc-hidden module
	// re-export until the facade exposes it directly (tracked in #4444).
	use reinhardt::reinhardt_auth::BaseUserManager;
	use serde_json::Value;
	use std::collections::HashMap;

	/// Project-local `BaseUserManager<User>` implementation.
	///
	/// Encapsulates the "create + hash + persist" pipeline for the tutorial
	/// `User`. Server functions receive an injected instance via
	/// `#[inject] um: Depends<UserManager>` and delegate to `create_user`
	/// / `create_superuser` so password hashing, uniqueness checks, and
	/// saves stay in a single place.
	///
	/// Until `#[user(...)]` learns to emit a manager itself (tracked in
	/// #4444), every `#[user]`-decorated model needs a manager like this
	/// one. `Clone` is derived so a server function can pull an owned
	/// `UserManager` out of `Depends<_>` and invoke the
	/// `BaseUserManager::create_user(&mut self, …)` trait method without
	/// fighting `Arc` mutability.
	///
	/// We register through `#[injectable_factory]` rather than `#[injectable]`
	/// on the struct itself because `#[injectable]` emits
	/// `#[async_trait::async_trait]` directly, requiring the consuming crate
	/// to add `async-trait` to its `Cargo.toml`. That breaks
	/// `examples/CLAUDE.md` DM-1 ("Reinhardt Dependencies Only"); the
	/// `#[injectable_factory]` path does not have this issue. See #4445.
	#[derive(Clone)]
	pub struct UserManager {
		db: DatabaseConnection,
	}

	#[injectable_factory(scope = "transient")]
	async fn user_manager_factory(#[inject] db: Depends<DatabaseConnection>) -> UserManager {
		UserManager { db: (*db).clone() }
	}

	impl UserManager {
		async fn build_user(
			&self,
			username: &str,
			password: Option<&str>,
			extra: &HashMap<String, Value>,
		) -> Result<User, Error> {
			let username = username.trim();
			if username.is_empty() {
				return Err(Error::Validation("Username cannot be empty".to_string()));
			}
			if username.chars().count() > 150 {
				return Err(Error::Validation(
					"Username must be 150 characters or fewer".to_string(),
				));
			}

			let manager = User::objects();
			let existing = manager
				.filter(
					User::field_username(),
					FilterOperator::Eq,
					FilterValue::String(username.to_string()),
				)
				.first()
				.await
				.map_err(|e| Error::Database(e.to_string()))?;
			if existing.is_some() {
				return Err(Error::Validation("Username is already taken".to_string()));
			}

			let is_active = extra
				.get("is_active")
				.and_then(|v| v.as_bool())
				.unwrap_or(true);

			// `User::build()` (typestate builder from `#[model]`) keeps this
			// call site stable as the schema grows — a new required field
			// surfaces as an additional setter rather than a positional
			// argument rewrite.
			let mut user = User::build()
				.username(username.to_string())
				.password_hash(None)
				.is_active(is_active)
				.is_superuser(false)
				.finish();
			if let Some(pw) = password {
				if pw.chars().count() < 8 {
					return Err(Error::Validation(
						"Password must be at least 8 characters".to_string(),
					));
				}
				user.set_password(pw)
					.map_err(|e| Error::Internal(format!("Password hashing failed: {}", e)))?;
			}
			Ok(user)
		}
	}

	#[async_trait]
	impl BaseUserManager<User> for UserManager {
		async fn create_user(
			&mut self,
			username: &str,
			password: Option<&str>,
			extra: HashMap<String, Value>,
		) -> Result<User, Error> {
			let new_user = self.build_user(username, password, &extra).await?;
			User::objects()
				.create_with_conn(&self.db, &new_user)
				.await
				.map_err(|e| Error::Database(e.to_string()))
		}

		async fn create_superuser(
			&mut self,
			username: &str,
			password: Option<&str>,
			extra: HashMap<String, Value>,
		) -> Result<User, Error> {
			let mut new_user = self.build_user(username, password, &extra).await?;
			new_user.is_superuser = true;
			User::objects()
				.create_with_conn(&self.db, &new_user)
				.await
				.map_err(|e| Error::Database(e.to_string()))
		}
	}
}

#[cfg(native)]
pub use manager::UserManager;
