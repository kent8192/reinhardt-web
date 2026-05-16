//! Verifies that `#[shared_model]` works on a struct with no `#[validate(...)]`
//! field attributes — the emitted `Validate` derive must compile to a trivial
//! `Ok(())` impl (handled by the underlying `validate_derive`).

#![allow(unexpected_cfgs)]

use reinhardt_macros::shared_model;

#[shared_model]
pub struct UserInfo {
	pub id: u64,
	pub username: String,
	pub email: String,
	pub is_active: bool,
}

fn main() {
	let _ = UserInfo {
		id: 1,
		username: String::from("alice"),
		email: String::from("alice@example.com"),
		is_active: true,
	};
}
