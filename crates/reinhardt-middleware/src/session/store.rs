//! In-memory `SessionStore` with lazy eviction of expired sessions.

use std::collections::HashMap;
use std::sync::RwLock;

use super::data::SessionData;

/// Session store with automatic lazy eviction of expired sessions
///
/// Performs threshold-based lazy cleanup of expired sessions to prevent
/// unbounded memory growth. Cleanup is triggered inside `save` whenever the
/// total number of stored sessions exceeds the configured threshold; it is
/// not time-based. The threshold defaults to
/// `SessionStore::DEFAULT_CLEANUP_THRESHOLD` and can be overridden at
/// construction via `SessionStore::with_cleanup_threshold` or adjusted at
/// runtime via `SessionStore::set_cleanup_threshold`.
#[derive(Debug, Default)]
pub struct SessionStore {
	/// Sessions
	pub(super) sessions: RwLock<HashMap<String, SessionData>>,
	/// Maximum number of sessions before triggering automatic cleanup
	max_sessions_before_cleanup: std::sync::atomic::AtomicUsize,
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
	/// number of stored sessions exceeds `threshold`.
	pub fn with_cleanup_threshold(threshold: usize) -> Self {
		Self {
			sessions: RwLock::new(HashMap::new()),
			max_sessions_before_cleanup: std::sync::atomic::AtomicUsize::new(threshold),
		}
	}

	/// Update the cleanup threshold at runtime.
	pub fn set_cleanup_threshold(&self, threshold: usize) {
		self.max_sessions_before_cleanup
			.store(threshold, std::sync::atomic::Ordering::Relaxed);
	}

	/// Get a session
	pub fn get(&self, id: &str) -> Option<SessionData> {
		let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
		sessions.get(id).cloned()
	}

	/// Save a session, with automatic cleanup when threshold is exceeded.
	///
	/// Cleanup is amortized: a full `retain` scan only runs when crossing
	/// the threshold from below, not on every subsequent insert.
	pub fn save(&self, session: SessionData) {
		let mut sessions = self.sessions.write().unwrap_or_else(|e| e.into_inner());
		let was_at_or_below = sessions.len()
			<= self
				.max_sessions_before_cleanup
				.load(std::sync::atomic::Ordering::Relaxed);
		sessions.insert(session.id.clone(), session);

		// Lazy eviction: clean up expired sessions only on the transition
		// from "<= threshold" to "> threshold" to avoid repeated full-map
		// scans under the write lock on every subsequent save.
		let threshold = self
			.max_sessions_before_cleanup
			.load(std::sync::atomic::Ordering::Relaxed);
		if was_at_or_below && sessions.len() > threshold {
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
