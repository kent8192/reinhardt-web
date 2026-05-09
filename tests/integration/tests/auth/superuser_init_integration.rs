//! Regression tests for issue #4237: `createsuperuser` writes nil UUID into the user PK.
//!
//! The `#[user(full = true)]` macro generates `SuperuserInit::init_superuser`,
//! which previously called `Self::default()` and returned the resulting struct
//! without re-seeding the primary key. For `Uuid` PKs, `Uuid::default()` is
//! `Uuid::nil()` (`00000000-0000-0000-0000-000000000000`), which meant every
//! `createsuperuser` invocation produced the same row id and the second call
//! collided on the unique PK. The fix adds a Uuid-aware PK setter to the macro
//! output so that the generated `init_superuser` reseeds the field with
//! `Uuid::now_v7()` before the username/email setters run.
//!
//! These tests cover the construction-time behavior only — they do not exercise
//! the ORM `objects().create()` path (that is the `TypedSuperuserCreator`
//! contract and lives elsewhere) — but they pin down the macro-generated code
//! so a future regression to `Self::default()` would fail before reaching the
//! database.

use chrono::{DateTime, Utc};
use reinhardt_auth::{Argon2Hasher, SuperuserInit};
use reinhardt_macros::{model, user};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[user(hasher = Argon2Hasher, username_field = "username", full = true)]
#[model(table_name = "issue_4237_superuser_init_users")]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Issue4237User {
	#[field(primary_key = true)]
	pub id: Uuid,
	pub username: String,
	pub email: String,
	pub first_name: String,
	pub last_name: String,
	pub password_hash: Option<String>,
	pub last_login: Option<DateTime<Utc>>,
	pub is_active: bool,
	pub is_staff: bool,
	pub is_superuser: bool,
	pub date_joined: DateTime<Utc>,
	pub user_permissions: Vec<String>,
	pub groups: Vec<String>,
}

#[test]
fn init_superuser_assigns_non_nil_uuid_to_primary_key() {
	// Arrange / Act
	let user = Issue4237User::init_superuser("admin", "admin@example.com");

	// Assert — the generated impl must reseed the Uuid PK after Self::default(),
	// otherwise the row would land in the database with the nil sentinel and
	// the second `createsuperuser` call would collide on the PK.
	assert_ne!(
		user.id,
		Uuid::nil(),
		"init_superuser left the Uuid PK at Uuid::default() (=nil); \
		 the macro must reseed it with Uuid::now_v7() — issue #4237"
	);
	// Sanity: the username/email setters still run after the PK setter.
	assert_eq!(user.username, "admin");
	assert_eq!(user.email, "admin@example.com");
	assert!(user.is_superuser);
	assert!(user.is_staff);
	assert!(user.is_active);
}

#[test]
fn init_superuser_produces_distinct_uuids_across_calls() {
	// Arrange / Act — back-to-back calls model the user creating two
	// superusers in succession (the original Issue #4237 reproducer).
	let first = Issue4237User::init_superuser("admin1", "admin1@example.com");
	let second = Issue4237User::init_superuser("admin2", "admin2@example.com");

	// Assert — UUID v7 carries random tail bits even within the same
	// millisecond, so any two calls must produce distinct ids. A
	// regression that reverts to `Self::default()` would fail here too,
	// because both ids would be `Uuid::nil()` and therefore equal.
	assert_ne!(
		first.id, second.id,
		"two consecutive init_superuser calls produced the same id, \
		 indicating the Uuid PK is not being freshly assigned — issue #4237"
	);
	assert_ne!(first.id, Uuid::nil());
	assert_ne!(second.id, Uuid::nil());
}
