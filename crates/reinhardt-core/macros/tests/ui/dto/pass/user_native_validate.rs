//! Verifies that legacy `#[cfg_attr(native, derive(Validate))]` annotations are
//! normalized by `#[dto]` while the trybuild environment exercises the shared
//! non-native validation expansion.

#![allow(unexpected_cfgs)]

extern crate self as reinhardt_core;

#[path = "../support.rs"]
mod support;

pub use reinhardt_macros::Validate;
pub use support::validators;

use reinhardt_macros::dto;

#[dto]
#[cfg_attr(native, derive(Validate))]
pub struct Mixed {
	#[validate(length(min = 1))]
	pub label: String,
}

fn main() {
	let value = Mixed {
		label: String::from("hello"),
	};
	assert!(reinhardt_core::validators::Validate::validate(&value).is_ok());
}
