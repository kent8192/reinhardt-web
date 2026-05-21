//! Multi-field structs cannot be newtypes.

use reinhardt_macros::newtype;

#[newtype]
pub struct Bad(u64, u64);

fn main() {}
