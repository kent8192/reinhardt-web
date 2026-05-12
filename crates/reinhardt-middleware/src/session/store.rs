//! In-memory `SessionStore` with lazy eviction of expired sessions.

use std::collections::HashMap;
use std::sync::RwLock;

use super::data::SessionData;

/// Session store with automatic lazy eviction of expired sessions
///
/// Performs periodic cleanup of expired sessions to prevent unbounded
/// memory growth. Cleanup runs automatically when the session count
/// exceeds a configurable threshold.
#[derive(Debug, Default)]
pub struct SessionStore {
	/// Sessions
	pub(super) sessions: RwLock<HashMap<String, SessionData>>,
	/// Maximum number of sessions before triggering automatic cleanup
	max_sessions_before_cleanup: std::sync::atomic::AtomicUsize,
}

impl SessionStore {
	/// Default cleanup threshold: trigger cleanup when session count exceeds 10,000
	const DEFAULT_CLEANUP_THRESHOLD: usize = 10_000;

	/// Create a new store
	pub fn new() -> Self {
		Self {
			sessions: RwLock::new(HashMap::new()),
			max_sessions_before_cleanup: std::sync::atomic::AtomicUsize::new(
				Self::DEFAULT_CLEANUP_THRESHOLD,
			),
		}
	}

	/// Get a session
	pub fn get(&self, id: &str) -> Option<SessionData> {
		let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
		sessions.get(id).cloned()
	}

	/// Save a session, with automatic cleanup when threshold is exceeded
	pub fn save(&self, session: SessionData) {
		let mut sessions = self.sessions.write().unwrap_or_else(|e| e.into_inner());
		sessions.insert(session.id.clone(), session);

		// Lazy eviction: clean up expired sessions when threshold is exceeded
		let threshold = self
			.max_sessions_before_cleanup
			.load(std::sync::atomic::Ordering::Relaxed);
		if sessions.len() > threshold {
			sessions.retain(|_, s| s.is_valid());
		}
	}

	/// Delete a session
	pub fn delete(&self, id: &str) {
		let mut sessions = self.sessions.write().unwrap_or_else(|e| e.into_inner());
		sessions.remove(id);
	}

	/// Clean up expired sessions
	pub fn cleanup(&self) {
		let mut sessions = self.sessions.write().unwrap_or_else(|e| e.into_inner());
		sessions.retain(|_, session| session.is_valid());
	}

	/// Clear the store
	pub fn clear(&self) {
		let mut sessions = self.sessions.write().unwrap_or_else(|e| e.into_inner());
		sessions.clear();
	}

	/// Get the number of sessions
	pub fn len(&self) -> usize {
		let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
		sessions.len()
	}

	/// Check if the store is empty
	pub fn is_empty(&self) -> bool {
		let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
		sessions.is_empty()
	}
}
