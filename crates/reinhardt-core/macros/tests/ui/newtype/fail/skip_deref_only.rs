//! Skipping `Deref` without `DerefMut` is rejected by the macro.

use reinhardt_macros::newtype;

#[newtype(skip(Deref))]
pub struct Bad(u64);

fn main() {}
