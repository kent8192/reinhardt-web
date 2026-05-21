//! `#[delegatable]` rejects traits with required associated types because the
//! generated `impl Trait for NewType` would be missing the type definition.

use reinhardt_macros::delegatable;

#[delegatable]
pub trait Storage {
	type Key;
	fn put(&self, key: Self::Key);
}

fn main() {}
