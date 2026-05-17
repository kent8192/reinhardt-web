//! Verifies that `#[dto(...)]` rejects arguments in v1.

use reinhardt_macros::dto;

#[dto(no_schema)]
pub struct Foo {
	pub x: i32,
}

fn main() {}
