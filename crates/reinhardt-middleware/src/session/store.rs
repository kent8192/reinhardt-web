//! In-memory `SessionStore` with lazy eviction of expired sessions and a hard
//! capacity limit.

use reinhardt_core::security::csrf::{generate_token_hmac, get_secret_bytes, verify_token_hmac};
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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
///
/// The store also holds at most `SessionStore::max_sessions` sessions
/// (`SessionStore::DEFAULT_MAX_SESSIONS` unless changed with
/// `SessionStore::set_max_sessions`). Saving a new session into a full store
/// purges expired sessions and then evicts the sessions closest to expiry, so
/// memory use stays bounded even when clients can cause sessions to be saved.
/// Use a persistent session backend if evicting live sessions under load is
/// not acceptable.
#[derive(Debug)]
pub struct SessionStore {
	/// Sessions
	pub(super) sessions: RwLock<HashMap<String, SessionData>>,
	/// Maximum number of sessions before triggering automatic cleanup
	max_sessions_before_cleanup: AtomicUsize,
	/// Next session count at which `save` should perform cleanup.
	next_cleanup_session_count: AtomicUsize,
	/// Hard limit on the number of stored sessions. Never zero.
	max_sessions: AtomicUsize,
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

	/// Default hard limit on the number of stored sessions: 100,000
	pub const DEFAULT_MAX_SESSIONS: usize = 100_000;

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
			max_sessions: AtomicUsize::new(Self::DEFAULT_MAX_SESSIONS),
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

	/// Set the hard limit on the number of stored sessions.
	///
	/// The limit is enforced the next time a new session is saved; see
	/// `SessionStore::save` for how a full store makes room.
	///
	/// # Examples
	///
	/// ```
	/// use std::num::NonZeroUsize;
	/// use reinhardt_middleware::session::SessionStore;
	///
	/// let store = SessionStore::new();
	/// assert_eq!(store.max_sessions(), SessionStore::DEFAULT_MAX_SESSIONS);
	///
	/// store.set_max_sessions(NonZeroUsize::new(50_000).unwrap());
	/// assert_eq!(store.max_sessions(), 50_000);
	/// ```
	pub fn set_max_sessions(&self, max_sessions: NonZeroUsize) {
		self.max_sessions
			.store(max_sessions.get(), Ordering::Relaxed);
	}

	/// Get the hard limit on the number of stored sessions.
	pub fn max_sessions(&self) -> usize {
		self.max_sessions.load(Ordering::Relaxed)
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
	///
	/// Saving a session whose ID is not stored yet into a store that already
	/// holds `SessionStore::max_sessions` sessions first makes room: expired
	/// sessions are purged, then the sessions closest to expiry are evicted
	/// until a tenth of the capacity (at least one slot) is free. Updating a
	/// session that is already stored never evicts anything.
	pub fn save(&self, session: SessionData) {
		let mut sessions = self.sessions.write().unwrap_or_else(|e| e.into_inner());
		let max_sessions = self.max_sessions.load(Ordering::Relaxed);
		if sessions.len() >= max_sessions && !sessions.contains_key(&session.id) {
			make_room(&mut sessions, max_sessions);
		}
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

/// Frees room in a full store before a new session is inserted.
///
/// Expired sessions go first. If fewer than a tenth of `max_sessions` slots
/// (at least one) are free afterwards, the sessions closest to expiry are
/// evicted until that many are. Freeing a batch rather than a single slot
/// keeps a store that stays at capacity from rescanning every entry on each
/// insert.
fn make_room(sessions: &mut HashMap<String, SessionData>, max_sessions: usize) {
	sessions.retain(|_, session| session.is_valid());

	let target_len = max_sessions - (max_sessions / 10).max(1);
	let excess = sessions.len().saturating_sub(target_len);
	if excess == 0 {
		return;
	}

	let mut by_expiry: Vec<(SystemTime, &str)> = sessions
		.iter()
		.map(|(id, session)| (session.expires_at, id.as_str()))
		.collect();
	by_expiry.select_nth_unstable(excess - 1);
	let evicted: Vec<String> = by_expiry[..excess]
		.iter()
		.map(|(_, id)| (*id).to_owned())
		.collect();
	for id in evicted {
		sessions.remove(&id);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::time::Duration;

	fn store_with_max_sessions(max_sessions: usize) -> SessionStore {
		let store = SessionStore::new();
		store.set_max_sessions(NonZeroUsize::new(max_sessions).expect("non-zero test limit"));
		store
	}

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
	fn new_store_uses_the_default_max_sessions() {
		// Arrange + Act
		let store = SessionStore::new();

		// Assert
		assert_eq!(store.max_sessions(), SessionStore::DEFAULT_MAX_SESSIONS);
	}

	#[rstest]
	#[case(1)]
	#[case(2)]
	#[case(10)]
	#[case(25)]
	fn save_never_exceeds_max_sessions(#[case] max_sessions: usize) {
		// Arrange
		let store = store_with_max_sessions(max_sessions);

		for _ in 0..max_sessions * 3 {
			let session = SessionData::new(Duration::from_secs(3600));
			let id = session.id.clone();

			// Act
			store.save(session);

			// Assert
			assert!(store.len() <= max_sessions);
			assert!(store.contains_valid(&id));
		}
	}

	#[rstest]
	fn full_store_evicts_the_session_closest_to_expiry() {
		// Arrange
		let store = store_with_max_sessions(3);
		let ids = save_sessions_expiring_in(
			&store,
			&[
				Duration::from_secs(120),
				Duration::from_secs(60),
				Duration::from_secs(180),
			],
		);
		let incoming = SessionData::new(Duration::from_secs(3600));
		let incoming_id = incoming.id.clone();

		// Act
		store.save(incoming);

		// Assert
		assert_eq!(store.len(), 3);
		assert!(store.get(&ids[1]).is_none());
		assert!(store.contains_valid(&ids[0]));
		assert!(store.contains_valid(&ids[2]));
		assert!(store.contains_valid(&incoming_id));
	}

	#[rstest]
	fn full_store_frees_a_tenth_of_its_capacity_at_once() {
		// Arrange: 20 sessions expiring one minute apart, soonest first
		let store = store_with_max_sessions(20);
		let ttls: Vec<Duration> = (1..=20)
			.map(|minutes| Duration::from_secs(minutes * 60))
			.collect();
		let ids = save_sessions_expiring_in(&store, &ttls);

		// Act
		let incoming = SessionData::new(Duration::from_secs(3600));
		let incoming_id = incoming.id.clone();
		store.save(incoming);

		// Assert: the two soonest-expiring sessions made room for the new one
		// and one more, so the next new session needs no further eviction.
		assert_eq!(store.len(), 19);
		assert!(store.get(&ids[0]).is_none());
		assert!(store.get(&ids[1]).is_none());
		assert!(ids[2..].iter().all(|id| store.contains_valid(id)));
		assert!(store.contains_valid(&incoming_id));
	}

	#[rstest]
	fn full_store_drops_expired_sessions_before_live_ones() {
		// Arrange
		let store = store_with_max_sessions(2);
		let mut expired = SessionData::new(Duration::from_secs(3600));
		expired.expires_at = SystemTime::now() - Duration::from_secs(1);
		let expired_id = expired.id.clone();
		store.save(expired);
		let live_ids = save_sessions_expiring_in(&store, &[Duration::from_secs(60)]);
		let incoming = SessionData::new(Duration::from_secs(3600));
		let incoming_id = incoming.id.clone();

		// Act
		store.save(incoming);

		// Assert
		assert_eq!(store.len(), 2);
		assert!(store.get(&expired_id).is_none());
		assert!(store.contains_valid(&live_ids[0]));
		assert!(store.contains_valid(&incoming_id));
	}

	#[rstest]
	fn updating_a_stored_session_in_a_full_store_evicts_nothing() {
		// Arrange
		let store = store_with_max_sessions(2);
		let ids =
			save_sessions_expiring_in(&store, &[Duration::from_secs(60), Duration::from_secs(120)]);
		let mut updated = store.get(&ids[0]).unwrap();
		updated.set("cart".to_string(), 3).unwrap();

		// Act
		store.save(updated);

		// Assert
		assert_eq!(store.len(), 2);
		assert_eq!(store.get(&ids[0]).unwrap().get::<i32>("cart"), Some(3));
		assert!(store.contains_valid(&ids[1]));
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
