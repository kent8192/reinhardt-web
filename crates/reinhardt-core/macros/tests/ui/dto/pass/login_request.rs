//! Verifies that `#[dto]` accepts a struct with `#[validate(...)]`
//! field attributes and emits shared validation that compiles under the
//! trybuild environment.

#![allow(unexpected_cfgs)]

extern crate self as reinhardt_core;

#[path = "../support.rs"]
mod support;

pub use reinhardt_macros::Validate;
pub use support::validators;

use reinhardt_macros::dto;

#[dto]
pub struct LoginRequest {
	#[validate(length(min = 1))]
	pub email: String,
	#[validate(length(min = 8, message = "Password too short"))]
	pub password: String,
}

fn main() {
	let value = LoginRequest {
		email: String::from("user@example.com"),
		password: String::from("hunter2hunter2"),
	};
	assert!(reinhardt_core::validators::Validate::validate(&value).is_ok());
}
