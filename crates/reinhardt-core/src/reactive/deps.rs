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

/// Trait implemented by reactive values that can participate in a hook dependency list.
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

/// An explicit collection of reactive dependencies.
#[derive(Debug)]
pub struct ExplicitDeps(Deps);

impl ExplicitDeps {
	/// Builds an explicit dependency collection from reactive node IDs.
	#[doc(hidden)]
	pub fn from_node_ids(ids: impl IntoIterator<Item = NodeId>) -> Self {
		let mut nodes = SmallVec::new();
		nodes.extend(ids);
		Self(Deps(nodes))
	}

	#[doc(hidden)]
	pub fn into_deps(self) -> Deps {
		self.0
	}

	#[doc(hidden)]
	pub fn as_slice(&self) -> &[NodeId] {
		self.0.as_slice()
	}
}

/// Selects explicit or automatically tracked reactive dependencies.
#[derive(Debug)]
pub enum ReactiveDeps {
	/// Uses the provided explicit dependency collection.
	Explicit(ExplicitDeps),
	/// Uses dependencies discovered while the reactive closure runs.
	Auto,
}

impl From<ExplicitDeps> for ReactiveDeps {
	fn from(deps: ExplicitDeps) -> Self {
		Self::Explicit(deps)
	}
}

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

	/// Construct a `Deps` directly from a slice of `NodeId`s.
	///
	/// Crate-internal convenience used by tests and by hook helpers that
	/// already hold raw `NodeId`s rather than `Trackable` values.
	#[allow(dead_code)]
	#[doc(hidden)]
	pub fn from_signals(ids: &[NodeId]) -> Self {
		let mut sv = SmallVec::new();
		sv.extend_from_slice(ids);
		Deps(sv)
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

/// Creates an explicit dependency collection from trackable expressions.
#[macro_export]
macro_rules! deps {
	($($dependency:expr),* $(,)?) => {{
		$crate::reactive::ExplicitDeps::from_node_ids([
			$($crate::reactive::Trackable::node_id(&$dependency),)*
		])
	}};
}

/// Selects automatic reactive dependency tracking.
#[macro_export]
macro_rules! deps_auto {
	() => {
		$crate::reactive::ReactiveDeps::Auto
	};
}

#[cfg(test)]
mod tests {
	use rstest::rstest;
	use serial_test::serial;

	use crate::reactive::memo::Memo;
	use crate::reactive::signal::Signal;

	#[rstest]
	#[serial(reactive_runtime)]
	fn explicit_deps_macro_collects_signal_node_ids() {
		crate::reactive::ReactiveScope::run(|| {
			// Arrange
			let signal = Signal::new(42_i32);

			// Act
			let deps = crate::deps![signal];

			// Assert
			assert_eq!(deps.as_slice(), &[signal.id()]);
		});
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn explicit_deps_with_memo_collects_memo_node_id() {
		crate::reactive::ReactiveScope::run(|| {
			// Arrange
			let signal = Signal::new(2_i32);
			let memo = Memo::new(move || signal.get() * 10);
			let memo_id = memo.id();

			// Act
			let deps = crate::deps![memo];

			// Assert
			let slice = deps.as_slice();
			assert_eq!(
				slice.len(),
				1,
				"single-element tuple of Memo must yield one NodeId"
			);
			assert_eq!(slice[0], memo_id, "deps element must be Memo::id()");
		});
	}
}
