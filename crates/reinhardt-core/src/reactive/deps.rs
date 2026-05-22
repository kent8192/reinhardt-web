//! Explicit dependency declarations for hook closures.
//!
//! This module supplies the type vocabulary used by the React-aligned hooks
//! (`use_effect`, `use_memo`, `use_callback`, ...) to declare what reactive
//! inputs drive their re-execution. See the design spec at
//! `docs/superpowers/specs/2026-05-22-issue-4195-hooks-deps-array-design.md`.

use smallvec::SmallVec;

use super::runtime::NodeId;

/// Marker trait for reactive values that can participate in a hook deps tuple.
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
pub struct Deps(SmallVec<[NodeId; 8]>);

impl Deps {
	// See struct-level note: consumed by Task 5 / Task 6.
	#[allow(dead_code)]
	pub(crate) fn as_slice(&self) -> &[NodeId] {
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
