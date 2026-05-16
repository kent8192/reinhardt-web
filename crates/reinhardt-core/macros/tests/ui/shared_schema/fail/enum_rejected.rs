//! Verifies that `#[shared_schema]` rejects enums with a clear error.

use reinhardt_macros::shared_schema;

#[shared_schema]
pub enum Status {
	Active,
	Inactive,
}

fn main() {}
