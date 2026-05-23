//! Trackable: trait for values that can act as reactive dependencies.

use crate::reactive::{Memo, Signal};

/// A value whose identity can be tracked across reactivity cycles.
///
/// Implemented by `Signal<T>`, `Memo<T>`, and `Resource<T>`. Used by
/// `page!` codegen (auto-wrap visitor) and by hook deps tuples (#4195).
pub trait Trackable {
	/// Returns an opaque identifier stable across clones of the same source.
	fn signal_id(&self) -> u64;
}

impl<T: 'static> Trackable for Signal<T> {
	fn signal_id(&self) -> u64 {
		Signal::id(self).as_u64()
	}
}

impl<T: Clone + 'static> Trackable for Memo<T> {
	fn signal_id(&self) -> u64 {
		Memo::id(self).as_u64()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serial_test::serial;

	fn assert_trackable<T: Trackable>(_: &T) {}

	#[rstest]
	#[serial]
	fn signal_implements_trackable() {
		// Arrange
		let s = Signal::new(0_i32);

		// Act + Assert (compile-time check via fn bound)
		assert_trackable(&s);
	}

	#[rstest]
	#[serial]
	fn memo_implements_trackable() {
		// Arrange
		let m = Memo::new(|| 0_i32);

		// Act + Assert
		assert_trackable(&m);
	}
}
