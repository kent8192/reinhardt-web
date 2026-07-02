//! Verifies that `#[dto]` rejects unit structs before emitting derives.

use reinhardt_macros::dto;

#[dto]
pub struct UnitDto;

fn main() {}
