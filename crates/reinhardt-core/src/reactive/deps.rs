//! Explicit dependency declarations for hook closures.
//!
//! This module supplies the type vocabulary used by the React-aligned hooks
//! (`use_effect`, `use_memo`, `use_callback`, ...) to declare what reactive
//! inputs drive their re-execution. See the design spec at
//! `docs/superpowers/specs/2026-05-22-issue-4195-hooks-deps-array-design.md`.

use smallvec::SmallVec;

use super::memo::Memo;
use super::runtime::NodeId;
use super::signal::Signal;

/// Trait implemented by reactive values that can participate in a hook deps tuple.
///
/// Implemented for `Signal<T>`, `Memo<T>` (in this crate), and `Resource<T, E>`
/// (in `reinhardt-pages`). The trait is intentionally open so 3rd-party
/// reactive primitives may participate.
pub trait Trackable {
	/// Returns the reactive runtime `NodeId` that backs this value.
	///
	/// Hook `*::new_with_deps` constructors call this once per dependency to
	/// register an explicit subscription with the runtime.
	fn node_id(&self) -> NodeId;
}

/// Opaque container of `NodeId`s used by `*::new_with_deps` constructors to
/// route subscriptions. Uses an inline `SmallVec` capacity of 8 to avoid heap
/// allocation in the common case (React deps are empirically 0–3 entries).
//
// The inner `SmallVec` and the `as_slice` / `into_inner` accessors are unused
// in Task 1 of the Layer ② plan and will become live once Task 5 (`Effect::
// new_with_deps`) and Task 6 (`Memo::new_with_deps`) land. Suppress dead-code
// noise during the foundational commit.
#[allow(dead_code)]
#[derive(Debug)]
pub struct Deps(SmallVec<[NodeId; 8]>);

impl Deps {
	/// Returns the internal `NodeId` slice for subscription routing.
	pub fn as_slice(&self) -> &[NodeId] {
		&self.0
	}

	// See struct-level note: consumed by Task 5 / Task 6.
	#[allow(dead_code)]
	pub(crate) fn into_inner(self) -> SmallVec<[NodeId; 8]> {
		self.0
	}

	pub(crate) fn empty() -> Self {
		Deps(SmallVec::new())
	}

	/// Construct a `Deps` directly from a slice of `NodeId`s.
	///
	/// Crate-internal convenience used by tests and by hook helpers that
	/// already hold raw `NodeId`s rather than `Trackable` values.
	#[allow(dead_code)]
	pub(crate) fn from_signals(ids: &[NodeId]) -> Self {
		let mut sv = SmallVec::new();
		sv.extend_from_slice(ids);
		Deps(sv)
	}
}

/// Conversion from a tuple of `Trackable`s (or `()`) into `Deps`. Implemented
/// for `()` (mount-only) and tuples of arity 1..=12 via the macro below.
pub trait IntoDeps {
	/// Consumes `self` and produces a `Deps` value carrying the reactive
	/// `NodeId`s extracted from each tuple element.
	fn into_deps(self) -> Deps;
}

impl IntoDeps for () {
	fn into_deps(self) -> Deps {
		Deps::empty()
	}
}

impl<T: 'static> Trackable for Signal<T> {
	fn node_id(&self) -> NodeId {
		self.id()
	}
}

impl<T: Clone + 'static> Trackable for Memo<T> {
	fn node_id(&self) -> NodeId {
		self.id()
	}
}

macro_rules! impl_into_deps_for_tuple {
	($($name:ident),+) => {
		impl<$($name: Trackable),+> IntoDeps for ($($name,)+) {
			#[allow(non_snake_case)]
			fn into_deps(self) -> Deps {
				let ($($name,)+) = self;
				let mut sv: SmallVec<[NodeId; 8]> = SmallVec::new();
				$( sv.push($name.node_id()); )+
				Deps(sv)
			}
		}
	};
}

impl_into_deps_for_tuple!(T1);
impl_into_deps_for_tuple!(T1, T2);
impl_into_deps_for_tuple!(T1, T2, T3);
impl_into_deps_for_tuple!(T1, T2, T3, T4);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_into_deps_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

#[cfg(test)]
mod tests {
	use rstest::rstest;
	use serial_test::serial;

	use super::*;
	use crate::reactive::memo::Memo;
	use crate::reactive::signal::Signal;

	#[rstest]
	#[serial(reactive_runtime)]
	fn into_deps_unit_is_empty() {
		// Arrange
		let deps_input: () = ();

		// Act
		let deps = deps_input.into_deps();

		// Assert
		assert!(deps.as_slice().is_empty());
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn into_deps_single_signal() {
		// Arrange
		let s = Signal::new(42_i32);

		// Act
		let deps = (s.clone(),).into_deps();

		// Assert
		assert_eq!(deps.as_slice(), &[s.id()]);
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn into_deps_three_signals_preserves_order() {
		// Arrange
		let a = Signal::new(1_i32);
		let b = Signal::new("two");
		let c = Signal::new(3.0_f64);

		// Act
		let deps = (a.clone(), b.clone(), c.clone()).into_deps();

		// Assert
		assert_eq!(deps.as_slice(), &[a.id(), b.id(), c.id()]);
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn into_deps_arity_12_compiles_and_collects() {
		// Arrange
		let s = [
			Signal::new(0_i32),
			Signal::new(1_i32),
			Signal::new(2_i32),
			Signal::new(3_i32),
			Signal::new(4_i32),
			Signal::new(5_i32),
			Signal::new(6_i32),
			Signal::new(7_i32),
			Signal::new(8_i32),
			Signal::new(9_i32),
			Signal::new(10_i32),
			Signal::new(11_i32),
		];

		// Act
		let deps = (
			s[0].clone(),
			s[1].clone(),
			s[2].clone(),
			s[3].clone(),
			s[4].clone(),
			s[5].clone(),
			s[6].clone(),
			s[7].clone(),
			s[8].clone(),
			s[9].clone(),
			s[10].clone(),
			s[11].clone(),
		)
			.into_deps();

		// Assert
		assert_eq!(deps.as_slice().len(), 12);
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn into_deps_with_memo_collects_memo_node_id() {
		// Arrange
		let signal = Signal::new(2_i32);
		let signal_clone = signal.clone();
		let memo = Memo::new(move || signal_clone.get() * 10);
		let memo_id = memo.id();

		// Act
		let deps = (memo,).into_deps();

		// Assert
		let slice = deps.as_slice();
		assert_eq!(
			slice.len(),
			1,
			"single-element tuple of Memo must yield one NodeId"
		);
		assert_eq!(slice[0], memo_id, "deps element must be Memo::id()");
	}
}
