//! Verifies that combining `#[dto]` with an unconditional `#[derive(Validate)]`
//! is rejected upfront — otherwise the macro would emit a second
//! `cfg_attr(native, derive(Validate))` and produce a confusing duplicate-derive
//! compile error on native builds.

#![allow(unexpected_cfgs)]

use reinhardt_macros::dto;

#[dto]
#[derive(reinhardt_macros::Validate)]
pub struct Foo {
	pub x: i32,
}

fn main() {}
