//! `SessionData`: per-session payload + helpers for read/write/rotate.

use reinhardt_http::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use super::id::ActiveSessionId;

/// Canonical session-store key used by Reinhardt examples to persist the
/// authenticated user's primary key after a successful login.
///
/// This is the key consumed by the [`crate::session::SessionValue`] and
/// [`crate::session::OptionalSessionValue`] extractors and written by the
/// [`crate::session::SessionAuthExt`] helper trait. Application code should
/// reference this constant instead of hardcoding `"user_id"` so that any
/// future migration to a different key (for example, the Django-compatible
/// `_auth_user_id` used by `reinhardt-auth::session`) is mechanical.
///
/// See issue #4446.
pub const USER_ID_SESSION_KEY: &str = "user_id";

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SessionData {
	/// Session ID
	pub id: String,
	/// Data
	pub data: HashMap<String, serde_json::Value>,
	/// Creation timestamp
	pub created_at: SystemTime,
	/// Last access timestamp
	pub last_accessed: SystemTime,
	/// Expiration timestamp
	pub expires_at: SystemTime,
	/// Back-reference to the request-scoped active session ID holder.
	///
	/// Populated by `SessionData::inject` from the request extensions; used by
	/// `regenerate_id` to keep the middleware's `Set-Cookie` value in sync
	/// with the rotated session ID. Never serialized — sessions persisted to a
	/// store carry only the data they own. See #3827.
	///
	/// Defaults to `None`; callers constructing `SessionData` literally outside
	/// the middleware (tests, fixtures) can leave it `None` because rotation
	/// only matters when the session is actively wired into a live request.
	#[serde(skip)]
	pub id_holder: Option<ActiveSessionId>,
}

impl SessionData {
	/// Create a new session
	pub fn new(ttl: Duration) -> Self {
		let now = SystemTime::now();
		Self {
			id: Uuid::new_v4().to_string(),
			data: HashMap::new(),
			created_at: now,
			last_accessed: now,
			expires_at: now + ttl,
			id_holder: None,
		}
	}

	/// Rotate the session ID (e.g., after authentication, to prevent session
	/// fixation). Updates both `self.id` and the request-scoped
	/// [`ActiveSessionId`] so that `SessionMiddleware` writes the new ID to
	/// the response cookie.
	///
	/// Returns the previous ID so callers can delete the stale entry from
	/// the store.
	///
	/// See #3827.
	pub fn regenerate_id(&mut self) -> String {
		let old_id = std::mem::replace(&mut self.id, Uuid::now_v7().to_string());
		if let Some(holder) = &self.id_holder {
			holder.set(self.id.clone());
		}
		old_id
	}

	/// Check if session is valid
	pub(super) fn is_valid(&self) -> bool {
		SystemTime::now() < self.expires_at
	}

	/// Update last access timestamp
	pub fn touch(&mut self, ttl: Duration) {
		let now = SystemTime::now();
		self.last_accessed = now;
		self.expires_at = now + ttl;
	}

	/// Get a value
	pub fn get<T>(&self, key: &str) -> Option<T>
	where
		T: for<'de> Deserialize<'de>,
	{
		self.data
			.get(key)
			.and_then(|v| serde_json::from_value(v.clone()).ok())
	}

	/// Set a value
	pub fn set<T>(&mut self, key: String, value: T) -> Result<()>
	where
		T: Serialize,
	{
		self.data.insert(
			key,
			serde_json::to_value(value)
				.map_err(|e| reinhardt_core::exception::Error::Serialization(e.to_string()))?,
		);
		Ok(())
	}

	/// Delete a value
	pub fn delete(&mut self, key: &str) {
		self.data.remove(key);
	}

	/// Check if a key exists
	pub fn contains_key(&self, key: &str) -> bool {
		self.data.contains_key(key)
	}

	/// Clear the session
	pub fn clear(&mut self) {
		self.data.clear();
	}
}
