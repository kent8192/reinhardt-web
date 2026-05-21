//! Unknown trait names in `skip(...)` are surfaced as compile errors.

use reinhardt_macros::newtype;

#[newtype(skip(MagicSauce))]
pub struct Bad(u64);

fn main() {}
