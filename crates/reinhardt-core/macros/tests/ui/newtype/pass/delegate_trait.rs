//! `#[delegatable]` + `#[newtype(delegate(Trait))]` round-trip.
//!
//! Demonstrates the v3 delegation: a logging decorator forwards every
//! `Repository` method to the wrapped backing store.

use reinhardt_macros::{delegatable, newtype};

#[delegatable]
pub trait Repository {
	fn find(&self, id: u64) -> Option<String>;
	fn count(&self) -> usize;
}

pub struct InMemory;

impl Repository for InMemory {
	fn find(&self, id: u64) -> Option<String> {
		Some(format!("user-{id}"))
	}
	fn count(&self) -> usize {
		1
	}
}

#[newtype(skip(Copy, Display, From, Into, AsRef, AsMut, Deref, DerefMut, FromStr, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Clone), delegate(Repository))]
pub struct LoggingRepo(InMemory);

fn main() {
	let r = LoggingRepo(InMemory);
	assert_eq!(r.find(42), Some(String::from("user-42")));
	assert_eq!(r.count(), 1);
}
