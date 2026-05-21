//! `skip(Copy)` lets us wrap non-`Copy` inner types like `String`.

use reinhardt_macros::newtype;

#[newtype(skip(Copy))]
pub struct UserName(String);

fn main() {
	let a = UserName(String::from("alice"));
	let b = a.clone();
	assert_eq!(a, b);
	assert_eq!(format!("{}", b), "alice");
}
