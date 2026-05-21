//! Single-field named struct form.

use reinhardt_macros::newtype;

#[newtype]
pub struct Counter {
	value: u64,
}

fn main() {
	let c = Counter { value: 3 };
	assert_eq!(*c, 3);
	assert_eq!(format!("{}", c), "3");
}
