//! Verifies the Copilot-reported edge case for the `ToSchema` import: when the
//! user already wrote `#[cfg_attr(native, derive(Schema))]`, `#[dto]` must
//! still emit `use ::reinhardt::rest::openapi::ToSchema as _;` so the
//! `inventory::submit!(..., Self::schema)` block produced by `Schema`'s derive
//! can resolve the trait-method sugar on native. The trybuild environment runs
//! the wasm-side expansion (no `native` cfg), so all we can guarantee here is
//! that the macro accepts the input and the resulting source compiles.

#![allow(unexpected_cfgs)]

use reinhardt_macros::dto;

#[dto]
#[cfg_attr(native, derive(Schema))]
pub struct Mixed {
	#[validate(length(min = 1))]
	pub label: String,
}

fn main() {
	let _ = Mixed {
		label: String::from("hello"),
	};
}
