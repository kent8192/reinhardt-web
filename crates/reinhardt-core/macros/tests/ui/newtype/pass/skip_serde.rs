//! `skip(Serialize, Deserialize)` removes the serde dependency on the
//! generated impls. Compiles without any serde derive in scope.

use reinhardt_macros::newtype;

#[newtype(skip(Serialize, Deserialize))]
pub struct Counter(u32);

fn main() {
	let c = Counter(0);
	assert_eq!(format!("{}", c), "0");
}
