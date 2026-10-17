//! In-memory `SessionStore` with lazy eviction of expired sessions.

use reinhardt_core::security::csrf::{generate_token_hmac, get_secret_bytes, verify_token_hmac};
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::data::SessionData;

/// DI key for resolving the middleware-owned session store through
/// `KeyedDepends<SessionStoreKey, Arc<SessionStore>>`.
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
	/// Authenticates the IDs of sessions issued but not stored yet.
	pending_id_key: PendingIdKey,
}

/// Random per-store HMAC key for unsaved session IDs, redacted from `Debug`.
struct PendingIdKey(Vec<u8>);

impl fmt::Debug for PendingIdKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("PendingIdKey(..)")
	}
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
			pending_id_key: PendingIdKey(get_secret_bytes()),
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

	/// Returns whether a session with `id` is stored and not expired.
	pub(super) fn contains_valid(&self, id: &str) -> bool {
		let sessions = self.sessions.read().unwrap_or_else(|e| e.into_inner());
		sessions.get(id).is_some_and(SessionData::is_valid)
	}

	/// Issue an ID for a session that is not stored yet, usable until
	/// `expires_at`.
	///
	/// The ID is `<nonce>.<expiry in Unix milliseconds>.<HMAC of both>`
	/// under this store's key, so `SessionStore::is_pending_id` recognizes it
	/// later without the store keeping any state for it.
	pub(super) fn new_pending_id(&self, expires_at: SystemTime) -> String {
		let payload = format!("{}.{}", Uuid::new_v4().simple(), unix_millis(expires_at));
		let mac = generate_token_hmac(&self.pending_id_key.0, &payload);
		format!("{payload}.{mac}")
	}

	/// Returns whether `id` was issued by `SessionStore::new_pending_id` on
	/// this store and has not expired.
	pub(super) fn is_pending_id(&self, id: &str) -> bool {
		let Some((payload, mac)) = id.rsplit_once('.') else {
			return false;
		};
		let Some(expires_at) = payload
			.split_once('.')
			.and_then(|(_, expires_at)| expires_at.parse::<u128>().ok())
		else {
			return false;
		};
		unix_millis(SystemTime::now()) < expires_at
			&& verify_token_hmac(mac, &self.pending_id_key.0, payload)
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

fn unix_millis(time: SystemTime) -> u128 {
	time.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_millis()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::time::Duration;

	/// Stores one session per TTL and returns their IDs in the same order.
	fn save_sessions_expiring_in(store: &SessionStore, ttls: &[Duration]) -> Vec<String> {
		ttls.iter()
			.map(|ttl| {
				let session = SessionData::new(*ttl);
				let id = session.id.clone();
				store.save(session);
				id
			})
			.collect()
	}

	#[rstest]
	fn pending_id_is_recognized_until_it_expires() {
		// Arrange
		let store = SessionStore::new();
		let live = store.new_pending_id(SystemTime::now() + Duration::from_secs(60));
		let expired = store.new_pending_id(SystemTime::now() - Duration::from_secs(1));

		// Act + Assert
		assert!(store.is_pending_id(&live));
		assert!(!store.is_pending_id(&expired));
		assert_ne!(
			live,
			store.new_pending_id(SystemTime::now() + Duration::from_secs(60))
		);
	}

	#[rstest]
	#[case::other_store_key("other-store")]
	#[case::extended_expiry("extended-expiry")]
	#[case::truncated_mac("truncated-mac")]
	#[case::plain_uuid("plain-uuid")]
	#[case::empty("empty")]
	fn forged_pending_ids_are_rejected(#[case] forgery: &str) {
		// Arrange
		let store = SessionStore::new();
		let issued = store.new_pending_id(SystemTime::now() + Duration::from_secs(60));
		let (payload, mac) = issued.rsplit_once('.').unwrap();
		let (nonce, _) = payload.split_once('.').unwrap();
		let forged = match forgery {
			"other-store" => {
				SessionStore::new().new_pending_id(SystemTime::now() + Duration::from_secs(60))
			}
			"extended-expiry" => format!("{nonce}.{}.{mac}", u128::MAX),
			"truncated-mac" => format!("{payload}.{}", &mac[..mac.len() - 2]),
			"plain-uuid" => SessionData::new(Duration::from_secs(60)).id,
			"empty" => String::new(),
			_ => unreachable!("unknown forgery case"),
		};

		// Act + Assert
		assert!(!store.is_pending_id(&forged));
	}

	#[rstest]
	fn debug_output_redacts_the_pending_id_key() {
		// Arrange
		let store = SessionStore::new();
		let key = format!("{:?}", store.pending_id_key.0);

		// Act
		let rendered = format!("{store:?}");

		// Assert
		assert!(rendered.contains("PendingIdKey(..)"));
		assert!(!rendered.contains(&key));
	}

	#[rstest]
	fn contains_valid_rejects_missing_and_expired_sessions() {
		// Arrange
		let store = SessionStore::new();
		let live_ids = save_sessions_expiring_in(&store, &[Duration::from_secs(60)]);
		let mut expired = SessionData::new(Duration::from_secs(3600));
		expired.expires_at = SystemTime::now() - Duration::from_secs(1);
		let expired_id = expired.id.clone();
		store.save(expired);

		// Act + Assert
		assert!(store.contains_valid(&live_ids[0]));
		assert!(!store.contains_valid(&expired_id));
		assert!(!store.contains_valid("missing"));
	}
}
