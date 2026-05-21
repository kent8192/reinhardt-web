//! Default `#[newtype]` invocation: every supported trait is generated.

use reinhardt_macros::newtype;
use std::str::FromStr;

#[newtype]
pub struct UserId(u64);

fn main() {
	// std derives
	let a = UserId(7);
	let b = a;
	assert_eq!(a, b);
	let mut s = std::collections::HashSet::new();
	s.insert(UserId(1));

	// Display
	assert_eq!(format!("{}", UserId(42)), "42");

	// From / Into
	let id: UserId = 99u64.into();
	let raw: u64 = id.into();
	assert_eq!(raw, 99);

	// AsRef / AsMut
	let mut x = UserId(1);
	let _: &u64 = x.as_ref();
	let _: &mut u64 = x.as_mut();

	// Deref / DerefMut
	let y = UserId(10);
	assert_eq!(*y, 10);

	// FromStr
	let parsed = UserId::from_str("123").unwrap();
	assert_eq!(parsed, UserId(123));

	// Serde round-trip
	let s = serde_json::to_string(&UserId(5)).unwrap();
	assert_eq!(s, "5");
	let back: UserId = serde_json::from_str("5").unwrap();
	assert_eq!(back, UserId(5));
}
