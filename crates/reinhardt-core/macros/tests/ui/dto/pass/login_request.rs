//! Verifies that `#[dto]` accepts a struct with `#[validate(...)]`
//! field attributes and that the emitted `cfg_attr(native, derive(...))` does
//! not break compilation under the trybuild environment (where `native` is
//! unset, so the wasm-side expansion is exercised).

#![allow(unexpected_cfgs)]

use reinhardt_macros::dto;

#[dto]
pub struct LoginRequest {
	#[validate(length(min = 1))]
	pub email: String,
	#[validate(length(min = 8, message = "Password too short"))]
	pub password: String,
}

fn main() {
	let _ = LoginRequest {
		email: String::from("user@example.com"),
		password: String::from("hunter2hunter2"),
	};
}
