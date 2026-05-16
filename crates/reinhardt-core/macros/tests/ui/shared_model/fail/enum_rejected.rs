//! Verifies that `#[shared_model]` rejects enums with a clear error.

use reinhardt_macros::shared_model;

#[shared_model]
pub enum Status {
	Active,
	Inactive,
}

fn main() {}
