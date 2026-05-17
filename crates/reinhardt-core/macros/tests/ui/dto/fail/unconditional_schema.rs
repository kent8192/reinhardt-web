//! Verifies that combining `#[dto]` with an unconditional `#[derive(Schema)]`
//! is rejected upfront. `Schema` lives behind the `native` cfg, so an
//! unconditional derive cannot resolve on wasm and would duplicate the macro's
//! emission on native.

#![allow(unexpected_cfgs)]

use reinhardt_macros::dto;

// Synthetic name to avoid requiring `reinhardt-openapi-macros` in scope; the
// `#[dto]` check matches by the last path segment.
#[derive()]
pub struct Schema;

#[dto]
#[derive(Schema)]
pub struct Bar {
	pub y: i32,
}

fn main() {}
