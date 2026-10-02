//! Verifies that `#[dto]` accepts an unconditional `#[derive(Validate)]`
//! annotation and preserves shared validation expansion.

#![allow(unexpected_cfgs)]

extern crate self as reinhardt_core;

#[path = "../support.rs"]
mod support;

pub use reinhardt_macros::Validate;
pub use support::validators;

use reinhardt_macros::dto;

#[dto]
#[derive(Validate)]
pub struct ExplicitValidate {
	#[validate(length(min = 1))]
	pub label: String,
}

fn main() {
	let value = ExplicitValidate {
		label: String::from("hello"),
	};
	assert!(reinhardt_core::validators::Validate::validate(&value).is_ok());
}
