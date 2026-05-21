//! `#[delegatable]` rejects traits with required associated consts because the
//! generated `impl Trait for NewType` would be missing the const definition.

use reinhardt_macros::delegatable;

#[delegatable]
pub trait Bounded {
	const MAX: usize;
	fn current(&self) -> usize;
}

fn main() {}
