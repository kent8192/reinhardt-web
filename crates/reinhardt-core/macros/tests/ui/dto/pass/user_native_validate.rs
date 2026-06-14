//! Verifies the idempotent path for users who already wrote
//! `#[cfg_attr(native, derive(Validate))]`. `#[dto]` must not emit a duplicate
//! native `Validate` derive, and the trybuild environment exercises the
//! non-native expansion where the cfg_attr is ignored.

#![allow(unexpected_cfgs)]

use reinhardt_macros::dto;

#[dto]
#[cfg_attr(native, derive(Validate))]
pub struct Mixed {
	#[validate(length(min = 1))]
	pub label: String,
}

fn main() {
	let _ = Mixed {
		label: String::from("hello"),
	};
}
