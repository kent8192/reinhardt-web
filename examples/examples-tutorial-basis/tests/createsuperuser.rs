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
#![cfg(native)]
use examples_tutorial_basis::apps::users::models::User;
use reinhardt::BaseUser;
use reinhardt::reinhardt_auth::{SuperuserCreatorRegistration, SuperuserInit};
use rstest::rstest;
#[rstest]
fn tutorial_user_auto_generates_superuser_init() {
	let username = "alice";
	let ignored_email = "";
	let user = User::init_superuser(username, ignored_email);
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
	let mut user = User::init_superuser("bob", "");
	user.set_password("hunter2-tutorial")
		.expect("set_password should succeed for Argon2Hasher-backed user");
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
	let registrations: Vec<&SuperuserCreatorRegistration> =
		inventory::iter::<SuperuserCreatorRegistration>().collect();
	let type_names: Vec<&str> = registrations.iter().map(|r| r.type_name).collect();
	let target_type_name = std::any::type_name::<User>();
	let found = registrations
		.iter()
		.any(|r| r.type_name == target_type_name);
	assert!(
		found,
		"tutorial's `User` must appear in SuperuserCreatorRegistration inventory; \
		 found entries: {:?}",
		type_names
	);
}
