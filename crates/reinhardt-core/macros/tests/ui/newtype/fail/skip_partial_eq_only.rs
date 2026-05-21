//! Skipping `PartialEq` without also skipping `Eq` is a contradiction.

use reinhardt_macros::newtype;

#[newtype(skip(PartialEq))]
pub struct Bad(u64);

fn main() {}
