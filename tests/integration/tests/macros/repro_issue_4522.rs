//! Regression test for issue #4522: relax `#[user]` SuperuserCreator
//! auto-registration guard from `full && has_model` to just `has_model`.
//!
//! Before #4522, [`SuperuserInit`] and the `inventory::submit!` entry for
//! [`SuperuserCreatorRegistration`] were emitted only when `full = true` was
//! set on `#[user]`. Minimal user structs (no `full = true`, no
//! `email/is_staff/date_joined/first_name/last_name`) were silently skipped,
//! so `cargo run --bin manage createsuperuser` failed with
//! "No SuperuserCreator registered" even though the user struct itself was
//! perfectly capable of representing a superuser.
//!
//! This test verifies that a minimal `#[user] + #[model]` struct receives
//! both the generated `SuperuserInit` impl and a `SuperuserCreatorRegistration`
//! inventory entry, regardless of whether `full = true` is set. It also
//! verifies that absent optional fields (e.g., `email`, `is_staff`,
//! `date_joined`) are gracefully no-op'd by the generated setters.
//!
//! Note: this test deliberately does NOT call
//! [`auto_register_superuser_creator`], because the macros test binary
//! also links the `#[user] + #[model]` struct from `repro_issue_3651.rs`,
//! which would trigger the multi-registration panic. The inventory is
//! inspected read-only instead.

use chrono::{DateTime, Utc};
use reinhardt::macros::user;
use reinhardt::prelude::*;
use reinhardt::{Argon2Hasher, BaseUser};
use reinhardt_auth::{SuperuserCreatorRegistration, SuperuserInit};
use serde::{Deserialize, Serialize};

/// Minimal user struct: NO `full = true`, NO `email`, NO `is_staff`,
/// NO `date_joined`, NO `first_name/last_name`. Mirrors the shape used by
/// `examples/examples-tutorial-basis` (Issue #4521).
#[user(hasher = Argon2Hasher, username_field = "username", manager = false)]
#[model(app_label = "auth_4522", table_name = "auth_minimal_user_4522")]
#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct MinimalUser4522 {
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

#[test]
fn minimal_user_superuser_init_compiles_and_sets_required_fields() {
	// Arrange
	let username = "alice";
	let email_ignored = "alice@example.com";

	// Act -- the proc macro generates `impl SuperuserInit for MinimalUser4522`
	// only when the relaxed `has_model` guard fires.
	let user = MinimalUser4522::init_superuser(username, email_ignored);

	// Assert -- required fields are set; absent optional fields stay at Default.
	assert_eq!(user.username, username, "username_field must be populated");
	assert!(
		user.is_superuser,
		"is_superuser must be true on init_superuser"
	);
	// is_active is wired via the macro-generated setter when the field exists.
	assert!(
		user.is_active,
		"is_active setter must run when the field is present"
	);
	// email is not on the struct, so the macro emits no setter — the field
	// simply does not exist. The `email_ignored` argument is silently dropped.
	// (Compile-time evidence that the absent-field path is exercised.)
}

#[test]
fn minimal_user_password_hashing_works_via_baseuser() {
	// Arrange
	let mut user = MinimalUser4522::init_superuser("bob", "");

	// Act
	user.set_password("hunter2").expect("set_password");

	// Assert
	assert!(user.password_hash().is_some(), "password_hash populated");
	assert!(
		user.check_password("hunter2").expect("check_password"),
		"password verification succeeds"
	);
}

#[test]
fn minimal_user_is_registered_in_superuser_creator_inventory() {
	// Arrange -- after #4522, every #[user] + #[model] struct submits an
	// inventory entry, regardless of `full = true`.
	let target_type_name = std::any::type_name::<MinimalUser4522>();

	// Act
	let registrations: Vec<&SuperuserCreatorRegistration> =
		inventory::iter::<SuperuserCreatorRegistration>().collect();

	// Assert -- our minimal user must appear among the registered creators.
	// We intentionally do NOT assert an exact count: this binary also links
	// `repro_issue_3651.rs`'s `User`, and future tests may add more.
	let found = registrations
		.iter()
		.any(|r| target_type_name.ends_with(r.type_name) || r.type_name == "MinimalUser4522");
	assert!(
		found,
		"MinimalUser4522 must be present in SuperuserCreatorRegistration inventory; \
		 found entries: {:?}",
		registrations
			.iter()
			.map(|r| r.type_name)
			.collect::<Vec<_>>()
	);
}
