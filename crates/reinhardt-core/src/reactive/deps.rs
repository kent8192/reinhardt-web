//! Deps tuple for React-style hook re-run gating (spec §4.2).
//!
//! `Deps` stores `Vec<u64>` — one element per dependency, where each
//! `u64` is `Trackable::signal_id()`. Equality compares only IDs, never
//! the underlying value, so `Deps` is cheap (no `T: Clone` requirement
//! on the user's signal value).
//!
//! # Placement rationale
//!
//! These types live in `reinhardt-core` (rather than `reinhardt-pages`)
//! because the runtime constructors `Effect::new_with_deps`,
//! `Memo::new_with_deps`, etc. must accept `Deps` directly. Putting
//! `Deps` in `reinhardt-pages` would create a circular crate dependency
//! (`reinhardt-core` cannot depend on `reinhardt-pages`). The pages
//! crate re-exports `Deps` / `IntoDeps` / `Trackable` so end users
//! continue to import them from `reinhardt_pages::reactive`.

extern crate alloc;
use alloc::vec::Vec;

use super::memo::Memo;
use super::signal::Signal;

/// Implemented by every reactive value that can be tracked as a hook
/// dependency.
///
/// Returns a stable per-instance identifier (the underlying `NodeId`
/// converted to `u64`). Cloning a `Signal<T>` or `Memo<T>` preserves
/// the identifier — equality is identity-based, never value-based.
pub trait Trackable {
	/// Returns this reactive value's stable identifier.
	fn signal_id(&self) -> u64;
}

impl<T: 'static> Trackable for Signal<T> {
	fn signal_id(&self) -> u64 {
		// `NodeId` wraps `usize`; lossless widening to `u64` keeps
		// `Deps` portable across 32-bit and 64-bit targets.
		self.id().as_u64()
	}
}

impl<T: Clone + 'static> Trackable for Memo<T> {
	fn signal_id(&self) -> u64 {
		self.id().as_u64()
	}
}

// Blanket impl so users can pass `&S` where `S: Trackable` (the typical
// shape: `(count.clone(),)` borrows from owned clones, but
// `(&count,)` is also expected to work for ergonomics).
impl<T: Trackable + ?Sized> Trackable for &T {
	fn signal_id(&self) -> u64 {
		(*self).signal_id()
	}
}

/// Opaque snapshot of a hook's dependency set.
///
/// Equality compares only signal IDs, so two `Deps` instances are
/// equal iff they reference exactly the same set of reactive values
/// (in the same positions).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Deps(pub(crate) Vec<u64>);

impl Deps {
	/// Returns the captured signal IDs in declaration order.
	pub fn ids(&self) -> &[u64] {
		&self.0
	}

	/// Returns `true` for the mount-only `()` shape (no dependencies).
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

/// Implemented by every tuple shape accepted as a hook's deps argument.
///
/// `impl IntoDeps for ()` is "mount-only" (the hook runs once and never
/// re-runs). Tuples `(T1,)` .. `(T1, .., T12)` accept any combination
/// of `Trackable` values.
pub trait IntoDeps {
	/// Captures the deps snapshot.
	fn into_deps(self) -> Deps;
}

impl IntoDeps for () {
	fn into_deps(self) -> Deps {
		Deps(Vec::new())
	}
}

macro_rules! impl_into_deps_tuple {
	( $( ($($T:ident),+) ),* $(,)? ) => {
		$(
			impl<$($T: Trackable),+> IntoDeps for ($($T,)+) {
				fn into_deps(self) -> Deps {
					#[allow(non_snake_case)] // tuple destructure reuses type-parameter names as bindings
					let ($($T,)+) = self;
					Deps(alloc::vec![ $($T.signal_id()),+ ])
				}
			}
		)*
	};
}

impl_into_deps_tuple!(
	(T1),
	(T1, T2),
	(T1, T2, T3),
	(T1, T2, T3, T4),
	(T1, T2, T3, T4, T5),
	(T1, T2, T3, T4, T5, T6),
	(T1, T2, T3, T4, T5, T6, T7),
	(T1, T2, T3, T4, T5, T6, T7, T8),
	(T1, T2, T3, T4, T5, T6, T7, T8, T9),
	(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10),
	(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11),
	(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12),
);

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serial_test::serial;

	#[rstest]
	#[serial]
	fn unit_is_mount_only() {
		// Arrange + Act
		let d: Deps = ().into_deps();

		// Assert
		assert!(d.is_empty());
		assert_eq!(d.ids().len(), 0);
	}

	#[rstest]
	#[serial]
	fn single_signal_tuple_captures_id() {
		// Arrange
		let s = Signal::new(7_i32);

		// Act
		let d = (s.clone(),).into_deps();

		// Assert
		assert_eq!(d.ids().len(), 1);
		assert_eq!(d.ids()[0], s.signal_id());
	}

	#[rstest]
	#[serial]
	fn equal_signals_produce_equal_deps_after_set() {
		// Arrange
		let s = Signal::new(0_i32);

		let before = (s.clone(),).into_deps();

		// Act — change the value but not the identity
		s.set(42);
		let after = (s.clone(),).into_deps();

		// Assert — value changes do not affect identity-based equality
		assert_eq!(before, after);
	}

	#[rstest]
	#[serial]
	fn different_signals_produce_different_deps() {
		// Arrange + Act
		let a = Signal::new(0_i32);
		let b = Signal::new(0_i32);
		let da = (a,).into_deps();
		let db = (b,).into_deps();

		// Assert
		assert_ne!(da, db);
	}

	#[rstest]
	#[serial]
	fn tuple_of_twelve_compiles() {
		// Arrange + Act (compile-only check that the macro expanded to 12)
		let s = Signal::new(0_i32);
		let d = (
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
			s.clone(),
		)
			.into_deps();

		// Assert
		assert_eq!(d.ids().len(), 12);
	}

	#[rstest]
	#[serial]
	fn memo_is_trackable() {
		// Arrange
		let s = Signal::new(3_i32);
		let s_for_memo = s.clone();
		let m = Memo::new(move || s_for_memo.get() * 2);

		// Act
		let d = (m.clone(),).into_deps();

		// Assert
		assert_eq!(d.ids().len(), 1);
		assert_eq!(d.ids()[0], m.signal_id());
		// Memo has a distinct identity from its source signal.
		assert_ne!(m.signal_id(), s.signal_id());
	}

	#[rstest]
	#[serial]
	fn mixed_signal_and_memo_tuple() {
		// Arrange
		let s = Signal::new(1_i32);
		let s_for_memo = s.clone();
		let m = Memo::new(move || s_for_memo.get());

		// Act
		let d = (s.clone(), m.clone()).into_deps();

		// Assert
		assert_eq!(d.ids().len(), 2);
		assert_eq!(d.ids()[0], s.signal_id());
		assert_eq!(d.ids()[1], m.signal_id());
	}

	#[rstest]
	#[serial]
	fn borrowed_signal_works_via_blanket_impl() {
		// Arrange
		let s = Signal::new(5_i32);

		// Act
		let d = (&s,).into_deps();

		// Assert
		assert_eq!(d.ids()[0], s.signal_id());
	}
}
