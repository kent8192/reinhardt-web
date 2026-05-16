//! Verifies that `#[shared_schema(...)]` rejects arguments in v1.

use reinhardt_macros::shared_schema;

#[shared_schema(no_schema)]
pub struct Foo {
	pub x: i32,
}

fn main() {}
