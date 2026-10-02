//! Verifies that `#[dto]` rejects tuple structs before emitting derives.

use reinhardt_macros::dto;

#[dto]
pub struct TupleDto(pub i32);

fn main() {}
