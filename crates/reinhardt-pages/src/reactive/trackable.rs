//! Trackable reactive values used by dependency-list macros.

/// Re-export the canonical core dependency trait.
pub use reinhardt_core::reactive::deps::Trackable;

#[cfg(test)]
mod tests {
	use super::Trackable;
	use crate::reactive::{Memo, NodeId, Signal};
	use rstest::rstest;
	use serial_test::serial;

	fn assert_trackable<T: Trackable>(_: &T) {}

	#[derive(Clone, Copy)]
	struct CustomTrackable(NodeId);

	impl Trackable for CustomTrackable {
		fn node_id(&self) -> NodeId {
			self.0
		}
	}

	#[rstest]
	#[serial]
	fn signal_implements_trackable() {
		// Arrange
		let signal = Signal::new(0_i32);

		// Act + Assert (compile-time check via fn bound)
		assert_trackable(&signal);
	}

	#[rstest]
	#[serial]
	fn memo_implements_trackable() {
		// Arrange
		let memo = Memo::new(|| 0_i32);

		// Act + Assert
		assert_trackable(&memo);
	}

	#[rstest]
	#[serial]
	fn custom_core_trackable_is_accepted_by_deps_macro() {
		// Arrange
		let signal = Signal::new(0_i32);
		let custom = CustomTrackable(signal.id());

		// Act
		let deps = crate::deps![custom];

		// Assert
		assert_eq!(deps.as_slice(), &[signal.id()]);
	}
}
