//! Generic structs are out of scope for the MVP.

use reinhardt_macros::newtype;

#[newtype]
pub struct Wrapper<T>(T);

fn main() {}
