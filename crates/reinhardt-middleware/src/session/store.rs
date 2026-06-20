//! In-memory `SessionStore` with lazy eviction of expired sessions.

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::data::SessionData;

/// DI key for resolving the middleware-owned session store through
/// `Depends<SessionStoreKey, Arc<SessionStore>>`.
#[derive(Debug, Clone, Copy)]
pub struct SessionStoreKey;

impl reinhardt_di::InjectableKey for SessionStoreKey {}

/// Session store with automatic lazy eviction of expired sessions
///
/// Performs threshold-based lazy cleanup of expired sessions to prevent
/// unbounded memory growth. Cleanup is triggered inside `save` when the
/// total number of stored sessions crosses an amortized cleanup boundary; it
/// is not time-based. The threshold defaults to
/// `SessionStore::DEFAULT_CLEANUP_THRESHOLD` and can be overridden at
/// construction via `SessionStore::with_cleanup_threshold` or adjusted at
/// runtime via `SessionStore::set_cleanup_threshold`.
#[derive(Debug)]
pub struct SessionStore {
	/// Sessions
	pub(super) sessions: RwLock<HashMap<String, SessionData>>,
	/// Maximum number of sessions before triggering automatic cleanup
	max_sessions_before_cleanup: AtomicUsize,
	/// Next session count at which `save` should perform cleanup.
	next_cleanup_session_count: AtomicUsize,
}

impl Default for SessionStore {
	fn default() -> Self {
		Self::new()
	}
}

impl SessionStore {
	/// Default cleanup threshold: trigger cleanup when session count exceeds 10,000
	pub const DEFAULT_CLEANUP_THRESHOLD: usize = 10_000;

	/// Create a new store with the default cleanup threshold
	/// (`SessionStore::DEFAULT_CLEANUP_THRESHOLD`).
	pub fn new() -> Self {
		Self::with_cleanup_threshold(Self::DEFAULT_CLEANUP_THRESHOLD)
	}

	/// Create a new store with a custom cleanup threshold.
	///
	/// The store triggers a single `retain` pass inside `save` whenever the
	/// number of stored sessions crosses an amortized cleanup boundary.
	pub fn with_cleanup_threshold(threshold: usize) -> Self {
		Self {
			sessions: RwLock::new(HashMap::new()),
			max_sessions_before_cleanup: AtomicUsize::new(threshold),
			next_cleanup_session_count: AtomicUsize::new(threshold),
		}
	}

	/// Update the cleanup threshold at runtime.
	pub fn set_cleanup_threshold(&self, threshold: usize) {
		self.max_sessions_before_cleanup
			.store(threshold, Ordering::Relaxed);
		self.next_cleanup_session_count
			.store(threshold, Ordering::Relaxed);
	}

	/// Get a session
	pub fn get(&self, id: &str) -> Option<SessionData> {
		let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
		sessions.get(id).cloned()
	}

	/// Save a session, with automatic cleanup when threshold is exceeded.
	///
	/// Cleanup runs when the post-save session count exceeds the configured
	/// threshold and reaches the next amortized cleanup boundary. The boundary
	/// advances after each pass so a store that remains above the threshold does
	/// not scan all sessions on every save.
	pub fn save(&self, session: SessionData) {
		let mut sessions = self.sessions.write().unwrap_or_else(|e| e.into_inner());
		sessions.insert(session.id.clone(), session);

		let threshold = self.max_sessions_before_cleanup.load(Ordering::Relaxed);
		let next_cleanup_session_count = self.next_cleanup_session_count.load(Ordering::Relaxed);
		if sessions.len() > threshold && sessions.len() >= next_cleanup_session_count {
			sessions.retain(|_, s| s.is_valid());

			let cleanup_interval = threshold.max(1);
			self.next_cleanup_session_count.store(
				sessions.len().saturating_add(cleanup_interval),
				Ordering::Relaxed,
			);
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
