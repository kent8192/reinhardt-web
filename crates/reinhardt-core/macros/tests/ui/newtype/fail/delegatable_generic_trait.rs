//! `#[delegatable]` rejects generic traits because the generated `impl Trait
//! for NewType` would be missing the required type arguments.

use reinhardt_macros::delegatable;

#[delegatable]
pub trait Container<T> {
	fn push(&mut self, value: T);
}

fn main() {}
