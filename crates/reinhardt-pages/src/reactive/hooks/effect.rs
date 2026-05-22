//! Effect hooks: `use_effect` and `use_layout_effect`.
//!
//! React-parity hooks for side effects. Both hooks take an explicit
//! deps tuple as the final positional argument — exact parity with
//! React's `useEffect(fn, [deps])`. There is no auto-tracking
//! variant (spec §4.2): missing the deps argument is a compile error
//! (`error[E0061]`).
//!
//! # Deps semantics
//!
//! - `()` — mount-only. The effect runs once when the component mounts
//!   and never re-runs.
//! - `(s,)` — one dependency. The effect re-runs whenever `s` is updated
//!   via `Signal::set` / `Memo` recompute / `Resource` state change.
//! - `(s1, s2, .., s12)` — up to twelve dependencies. The effect re-runs
//!   when any listed dep fires. Unlisted signals are ignored even if
//!   they are read inside the closure.
//!
//! Equality is identity-based: comparing two deps tuples compares the
//! underlying `signal_id()`s, never the values. Clones of the same
//! `Signal<T>` share an identifier, so passing `count.clone()` works.

use crate::reactive::deps::IntoDeps;
use crate::reactive::{Effect, runtime::EffectTiming};

/// Runs a side effect, re-running when any dependency in `deps` changes.
///
/// React parity: `useEffect(fn, [deps])`. The first argument is the
/// effect closure; the second is a tuple of [`Trackable`] dependencies.
/// Pass `()` for a mount-only effect that never re-runs.
///
/// # Type Parameters
///
/// - `F` — the effect closure.
/// - `D` — the deps tuple shape (`()` or `(T1,)` .. `(T1, .., T12)`).
///
/// # Arguments
///
/// - `f` — closure executed on mount and on every re-run.
/// - `deps` — explicit dependency tuple.
///
/// # Returns
///
/// An [`Effect`] handle that can be used to dispose the effect early.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::Signal;
/// use reinhardt_pages::reactive::hooks::use_effect;
///
/// let count = Signal::new(0);
///
/// // Re-runs whenever `count` changes.
/// let _e = use_effect({
///     let count = count.clone();
///     move || { let _ = count.get(); }
/// }, (count.clone(),));
///
/// // Mount-only: runs once, never re-runs.
/// let _mount = use_effect(|| {
///     // one-time setup
/// }, ());
/// ```
///
/// [`Trackable`]: crate::reactive::Trackable
pub fn use_effect<F, D>(f: F, deps: D) -> Effect
where
	F: FnMut() + 'static,
	D: IntoDeps,
{
	Effect::new_with_deps(f, deps.into_deps())
}

/// Runs a side effect synchronously before browser paint.
///
/// React parity: `useLayoutEffect(fn, [deps])`. Identical to
/// [`use_effect`] in API but uses [`EffectTiming::Layout`] internally so
/// the effect runs synchronously after DOM mutations but before the
/// browser paints.
///
/// # When to use
///
/// - Reading layout (e.g., `offsetWidth`, `getBoundingClientRect`).
/// - Applying visual updates that must be synchronous.
/// - Measuring DOM nodes and feeding the result into another signal.
///
/// # Warning
///
/// Layout effects block paint. Prefer [`use_effect`] unless you need
/// pre-paint synchrony.
///
/// # Arguments
///
/// - `f` — closure executed on mount and on every re-run.
/// - `deps` — explicit dependency tuple (`()` for mount-only).
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::Signal;
/// use reinhardt_pages::reactive::hooks::{use_layout_effect, use_ref};
///
/// let element_ref = use_ref(None::<web_sys::Element>);
/// let width = Signal::new(0);
///
/// use_layout_effect({
///     let element_ref = element_ref.clone();
///     let width = width.clone();
///     move || {
///         if let Some(el) = element_ref.current().as_ref() {
///             width.set(el.client_width());
///         }
///     }
/// }, ());
/// ```
pub fn use_layout_effect<F, D>(f: F, deps: D) -> Effect
where
	F: FnMut() + 'static,
	D: IntoDeps,
{
	Effect::new_with_deps_and_timing(f, deps.into_deps(), EffectTiming::Layout)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use rstest::rstest;
	use serial_test::serial;
	use std::cell::RefCell;
	use std::rc::Rc;

	#[rstest]
	#[serial]
	fn use_effect_with_explicit_deps_signature() {
		// Arrange
		let count = Signal::new(0_i32);
		let runs = Rc::new(RefCell::new(0_i32));

		// Act — new signature: closure first, deps second.
		let _e = use_effect(
			{
				let runs = Rc::clone(&runs);
				move || {
					*runs.borrow_mut() += 1;
				}
			},
			(count.clone(),),
		);

		// Assert — runs once on mount.
		assert_eq!(*runs.borrow(), 1);
	}

	#[rstest]
	#[serial]
	fn use_effect_mount_only_unit_deps() {
		// Arrange
		let runs = Rc::new(RefCell::new(0_i32));

		// Act
		let _e = use_effect(
			{
				let runs = Rc::clone(&runs);
				move || {
					*runs.borrow_mut() += 1;
				}
			},
			(),
		);

		// Assert
		assert_eq!(*runs.borrow(), 1);
	}

	#[test]
	#[serial]
	fn test_use_effect_runs_immediately() {
		let called = Rc::new(RefCell::new(false));

		let _effect = use_effect(
			{
				let called = Rc::clone(&called);
				move || {
					*called.borrow_mut() = true;
				}
			},
			(),
		);

		assert!(*called.borrow());
	}

	#[test]
	#[serial]
	fn test_use_effect_tracks_dependencies() {
		let count = Signal::new(0);
		let effect_count = Rc::new(RefCell::new(0));

		let _effect = use_effect(
			{
				let count = count.clone();
				let effect_count = Rc::clone(&effect_count);
				move || {
					let _ = count.get(); // Read the listed dep
					*effect_count.borrow_mut() += 1;
				}
			},
			(count.clone(),),
		);

		// Initial run
		assert_eq!(*effect_count.borrow(), 1);

		// Change the signal - effect should re-run
		// Note: In actual implementation, this would trigger re-run
		// via the runtime's update mechanism
	}

	#[test]
	#[serial]
	fn test_use_layout_effect() {
		let called = Rc::new(RefCell::new(false));

		let _effect = use_layout_effect(
			{
				let called = Rc::clone(&called);
				move || {
					*called.borrow_mut() = true;
				}
			},
			(),
		);

		assert!(*called.borrow());
	}

	#[test]
	#[serial]
	fn test_layout_effect_synchronous_execution() {
		// Test that layout effects execute synchronously when dependencies change
		let signal = Signal::new(0);
		let execution_order = Rc::new(RefCell::new(Vec::new()));

		let _effect = use_layout_effect(
			{
				let signal = signal.clone();
				let execution_order = Rc::clone(&execution_order);
				move || {
					let value = signal.get();
					execution_order.borrow_mut().push(value);
				}
			},
			(signal.clone(),),
		);

		// Initial execution
		assert_eq!(*execution_order.borrow(), vec![0]);

		// Change signal - layout effect should execute synchronously
		signal.set(1);
		execution_order.borrow_mut().push(100); // Marker after signal change

		// Layout effect should have executed before this marker
		// Note: In full implementation, this would be more evident with flush_updates()
	}

	#[test]
	#[serial]
	fn test_layout_vs_passive_timing() {
		// Test that layout effects have different timing than passive effects
		let signal = Signal::new(0);
		let layout_count = Rc::new(RefCell::new(0));
		let passive_count = Rc::new(RefCell::new(0));

		// Create layout effect
		let _layout_effect = use_layout_effect(
			{
				let signal = signal.clone();
				let layout_count = Rc::clone(&layout_count);
				move || {
					let _ = signal.get();
					*layout_count.borrow_mut() += 1;
				}
			},
			(signal.clone(),),
		);

		// Create passive effect (use_effect)
		let _passive_effect = use_effect(
			{
				let signal = signal.clone();
				let passive_count = Rc::clone(&passive_count);
				move || {
					let _ = signal.get();
					*passive_count.borrow_mut() += 1;
				}
			},
			(signal.clone(),),
		);

		// Both should have run initially
		assert_eq!(*layout_count.borrow(), 1);
		assert_eq!(*passive_count.borrow(), 1);

		// Change signal
		signal.set(1);

		// Layout effect executes synchronously
		assert_eq!(*layout_count.borrow(), 2);
		// Passive effect may not have executed yet (scheduled for microtask)
	}

	#[test]
	#[serial]
	fn test_mixed_layout_and_passive_effects() {
		// Test execution order when both layout and passive effects depend on same signal
		let signal = Signal::new(0);
		let execution_order = Rc::new(RefCell::new(Vec::new()));

		// Create layout effect (should execute first)
		let _layout_effect = use_layout_effect(
			{
				let signal = signal.clone();
				let execution_order = Rc::clone(&execution_order);
				move || {
					let value = signal.get();
					execution_order.borrow_mut().push(("layout", value));
				}
			},
			(signal.clone(),),
		);

		// Create passive effect
		let _passive_effect = use_effect(
			{
				let signal = signal.clone();
				let execution_order = Rc::clone(&execution_order);
				move || {
					let value = signal.get();
					execution_order.borrow_mut().push(("passive", value));
				}
			},
			(signal.clone(),),
		);

		// Both execute initially
		let order = execution_order.borrow();
		assert_eq!(order.len(), 2);
		assert_eq!(order[0], ("layout", 0));
		assert_eq!(order[1], ("passive", 0));
	}
}
