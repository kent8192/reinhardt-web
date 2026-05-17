//! Verifies that `#[dto]` rejects enums with a clear error.

use reinhardt_macros::dto;

#[dto]
pub enum Status {
	Active,
	Inactive,
}

fn main() {}
