//! Regression test for tutorial issue #4521 / framework issue #4522.
//!
//! Verifies that the tutorial's minimal `User` struct (no `full = true`,
//! `manager = false`) participates in the framework's auto-registration of
//! `SuperuserCreator` once #4522 relaxed the `#[user]` macro guard from
//! `parsed_args.full && has_model` to just `has_model`.
//!
//! Before #4522, `cargo run --bin manage createsuperuser` against the
//! tutorial failed with
//!
//! ```text
//! Error: No SuperuserCreator registered. Ensure your user model has
//!        #[user(hasher = ..., username_field = "...", full = true)] and
//!        #[model(...)]. Auto-registration happens automatically.
//! ```
//!
//! and the tutorial had to manually impl `SuperuserInit` and call
//! `register_superuser_creator(superuser_creator_for::<User>())` in
//! `bin/manage.rs`. Both manual workarounds were removed as part of this
//! verification PR; the auto-generated `SuperuserInit` impl and the
//! `inventory::submit!(SuperuserCreatorRegistration)` block emitted by the
//! `#[user] + #[model]` macro pair carry the same wiring.
//!
//! This test does NOT call `auto_register_superuser_creator()` directly,
//! because doing so would mutate the process-wide `SUPERUSER_CREATOR`
//! `OnceLock` and could collide with other test binaries that run in the
//! same `cargo test` invocation. The inventory is inspected read-only
//! instead, plus `SuperuserInit::init_superuser` is exercised directly.

// Native-only: the tutorial's `User` model and the `reinhardt-auth` types
// it pulls in (sqlx, etc.) do not build for `wasm32-unknown-unknown`.
#![cfg(server)]

use examples_tutorial_basis::apps::users::server::models::User;
use reinhardt::BaseUser;
use reinhardt::reinhardt_auth::{SuperuserCreatorRegistration, SuperuserInit};
use rstest::rstest;

#[rstest]
fn tutorial_user_auto_generates_superuser_init() {
	// Arrange
	let username = "alice";
	let ignored_email = "";

	// Act -- the `#[user] + #[model]` macro pair emits
	// `impl SuperuserInit for User` since #4522.
	let user = User::init_superuser(username, ignored_email);

	// Assert -- required superuser fields are set; the absent `email` field
	// is silently no-op'd by the generator (the tutorial's minimal User has
	// no email column).
	assert_eq!(
		user.username, username,
		"username_field must be populated by init_superuser"
	);
	assert!(
		user.is_superuser,
		"is_superuser must be true on init_superuser"
	);
	assert!(
		user.is_active,
		"is_active must be true on init_superuser (default-on field with setter)"
	);
}

#[rstest]
fn tutorial_user_init_superuser_password_hashing_works() {
	// Arrange
	let mut user = User::init_superuser("bob", "");

	// Act
	user.set_password("hunter2-tutorial")
		.expect("set_password should succeed for Argon2Hasher-backed user");

	// Assert
	assert!(
		user.password_hash().is_some(),
		"password_hash must be populated after set_password"
	);
	assert!(
		user.check_password("hunter2-tutorial")
			.expect("check_password should succeed"),
		"the hashed password must verify against the original plaintext"
	);
}

#[rstest]
fn tutorial_user_is_registered_in_superuser_creator_inventory() {
	// Arrange -- since #4522 every `#[user] + #[model]` struct submits an
	// inventory entry, regardless of `full = true`. The tutorial's `User`
	// must therefore appear here.
	let registrations: Vec<&SuperuserCreatorRegistration> =
		inventory::iter::<SuperuserCreatorRegistration>().collect();
	let type_names: Vec<&str> = registrations.iter().map(|r| r.type_name).collect();

	// Act -- look for the tutorial's User in the registered entries.
	let target_type_name = std::any::type_name::<User>();
	let found = registrations
		.iter()
		.any(|r| r.type_name == target_type_name);

	// Assert -- if this fails, the macro auto-registration regressed for
	// minimal-user models; re-check the guard in
	// `crates/reinhardt-core/macros/src/user_attribute.rs` (around the
	// `if has_model { ... }` block introduced by #4522).
	assert!(
		found,
		"tutorial's `User` must appear in SuperuserCreatorRegistration inventory; \
		 found entries: {:?}",
		type_names
	);
}
