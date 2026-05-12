//! Session-ID newtypes and the request-scoped active ID holder.
//!
//! These types are stored in request extensions by `SessionMiddleware`
//! so downstream handlers and `Injectable` implementations can read the
//! current session ID and configured cookie name without parsing cookies
//! manually.

use std::sync::{Arc, RwLock};

/// Newtype wrapper for session ID stored in request extensions.
///
/// Handlers can retrieve the current session ID from the request
/// extensions without parsing cookies manually.
///
/// # Example
///
/// ```rust,ignore
/// fn handle(&self, request: Request) -> Result<Response> {
///     if let Some(session_id) = request.extensions.get::<SessionId>() {
///         println!("Session: {}", session_id.as_str());
///     }
///     // ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
	/// Create a new `SessionId` from the given string.
	pub fn new(id: String) -> Self {
		Self(id)
	}

	/// Returns the session ID as a string slice.
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl AsRef<str> for SessionId {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

impl std::fmt::Display for SessionId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

/// Shared, mutable handle to the session ID that the middleware will write
/// to the response `Set-Cookie` header.
///
/// Stored in request extensions by `SessionMiddleware`. Handlers that rotate
/// the session ID (e.g., for session-fixation prevention on login) MUST
/// either call `SessionData::regenerate_id` (which updates this holder
/// transparently) or write to it directly via `set`. Otherwise the cookie
/// returned to the client points at a session ID that no longer exists in
/// the store. See #3827.
#[derive(Debug, Clone)]
pub struct ActiveSessionId(Arc<RwLock<String>>);

impl ActiveSessionId {
	/// Create an `ActiveSessionId` initialised to `id`.
	pub fn new(id: String) -> Self {
		Self(Arc::new(RwLock::new(id)))
	}

	/// Read the current session ID.
	pub fn get(&self) -> String {
		self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
	}

	/// Replace the session ID. Call after rotating the underlying
	/// `SessionData::id` so the middleware's `Set-Cookie` matches the
	/// store entry.
	pub fn set(&self, id: String) {
		*self.0.write().unwrap_or_else(|e| e.into_inner()) = id;
	}
}

/// Newtype wrapper for the configured session cookie name.
///
/// Stored in request extensions by `SessionMiddleware` so that
/// `Injectable` implementations can retrieve the configured cookie name
/// instead of hardcoding it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCookieName(String);

impl SessionCookieName {
	/// Create a new `SessionCookieName`.
	pub fn new(name: String) -> Self {
		Self(name)
	}

	/// Returns the cookie name as a string slice.
	pub fn as_str(&self) -> &str {
		&self.0
	}
}
