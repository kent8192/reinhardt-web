//! Integration tests for `SessionAuthExt::login` / `logout`.
//!
//! Exercises the helper trait end-to-end against an in-memory `SessionStore`:
//! we install a session, drive `login` / `logout`, and assert the four
//! observable post-conditions every example handler used to perform by
//! hand (regenerated id, old store entry removed, new entry present,
//! `USER_ID_SESSION_KEY` value round-trips). See issue #4446.

use std::sync::Arc;
use std::time::Duration;

use reinhardt_middleware::session::{
	SessionAuthExt, SessionData, SessionStore, USER_ID_SESSION_KEY,
};

fn make_store_and_session() -> (Arc<SessionStore>, SessionData) {
	let store = Arc::new(SessionStore::new());
	let session = SessionData::new(Duration::from_secs(3600));
	store.save(session.clone());
	(store, session)
}

#[tokio::test]
async fn login_regenerates_id_writes_user_id_and_rotates_store_entry() {
	let (store, mut session) = make_store_and_session();
	let original_id = session.id.clone();
	assert!(store.get(&original_id).is_some());

	session.login(&store, 42i64).unwrap();

	// The id was regenerated.
	assert_ne!(
		session.id, original_id,
		"login must regenerate the session id (fixation prevention)"
	);

	// The old store entry is gone.
	assert!(
		store.get(&original_id).is_none(),
		"login must delete the pre-rotation store entry"
	);

	// The new id is present in the store and carries the user id.
	let stored = store
		.get(&session.id)
		.expect("login must persist the rotated session under the new id");
	assert_eq!(stored.id, session.id);
	assert_eq!(
		stored.get::<i64>(USER_ID_SESSION_KEY),
		Some(42),
		"login must write user_id under USER_ID_SESSION_KEY"
	);
}

#[tokio::test]
async fn logout_rotates_id_drops_user_id_and_keeps_other_keys() {
	let (store, mut session) = make_store_and_session();

	// Pre-populate the session with both a user id and an unrelated key
	// (`flash`) — logout must preserve the latter so callers can still
	// surface "you have been logged out" messages on the response.
	session.login(&store, 7i64).unwrap();
	let post_login_id = session.id.clone();
	session.set("flash".to_string(), "bye".to_string()).unwrap();
	store.save(session.clone());

	session.logout(&store);

	assert_ne!(
		session.id, post_login_id,
		"logout must regenerate the session id"
	);
	assert!(
		store.get(&post_login_id).is_none(),
		"logout must delete the pre-rotation store entry"
	);
	let stored = store
		.get(&session.id)
		.expect("logout must persist the rotated session under the new id");
	assert!(
		stored.get::<i64>(USER_ID_SESSION_KEY).is_none(),
		"logout must clear the user id from the rotated session"
	);
	assert_eq!(
		stored.get::<String>("flash").as_deref(),
		Some("bye"),
		"logout must preserve unrelated session entries (only user_id is cleared)"
	);
}

#[tokio::test]
async fn login_round_trip_uuid_primary_key() {
	// Mirror the examples-twitter case where `PrimaryKey = Uuid`. The
	// helper must be primary-key-shape agnostic — anything `Serialize` works.
	use uuid::Uuid;

	let (store, mut session) = make_store_and_session();
	let user_id = Uuid::now_v7();

	session.login(&store, user_id).unwrap();

	let stored = store.get(&session.id).expect("rotated session present");
	assert_eq!(
		stored.get::<Uuid>(USER_ID_SESSION_KEY),
		Some(user_id),
		"login must round-trip non-integer primary keys via serde"
	);
}
