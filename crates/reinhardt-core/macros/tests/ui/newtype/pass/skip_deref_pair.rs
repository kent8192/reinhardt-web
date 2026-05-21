//! `skip(Deref, DerefMut)` opts out of the smart-pointer-style transparency,
//! a common preference per DESIGN_PHILOSOPHY §3 (CoC is a right, not an
//! obligation).

use reinhardt_macros::newtype;

#[newtype(skip(Copy, Deref, DerefMut))]
pub struct OpaqueToken(String);

fn main() {
	let t = OpaqueToken(String::from("secret"));
	// Display + AsRef still work.
	assert_eq!(format!("{}", t), "secret");
	let _: &String = t.as_ref();
}
