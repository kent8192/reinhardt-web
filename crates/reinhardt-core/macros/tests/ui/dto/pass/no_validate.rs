//! Verifies that `#[dto]` works on a struct with no `#[validate(...)]`
//! field attributes — the emitted `Validate` derive must compile to a trivial
//! `Ok(())` impl (handled by the underlying `validate_derive`).

#![allow(unexpected_cfgs)]

extern crate self as reinhardt_core;

#[path = "../support.rs"]
mod support;

pub use reinhardt_macros::Validate;
pub use support::validators;

use reinhardt_macros::dto;

#[dto]
pub struct UserInfo {
	pub id: u64,
	pub username: String,
	pub email: String,
	pub is_active: bool,
}

fn main() {
	let value = UserInfo {
		id: 1,
		username: String::from("alice"),
		email: String::from("alice@example.com"),
		is_active: true,
	};
	assert!(reinhardt_core::validators::Validate::validate(&value).is_ok());
}
