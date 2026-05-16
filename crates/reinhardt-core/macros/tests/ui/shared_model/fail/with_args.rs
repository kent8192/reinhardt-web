//! Verifies that `#[shared_model(...)]` rejects arguments in v1.

use reinhardt_macros::shared_model;

#[shared_model(no_schema)]
pub struct Foo {
	pub x: i32,
}

fn main() {}
