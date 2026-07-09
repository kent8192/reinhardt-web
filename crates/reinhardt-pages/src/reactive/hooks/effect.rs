//! Effect hooks: use_effect and use_layout_effect
//!
//! React-aligned side effect hooks. Both take an explicit dependency tuple
//! as the second argument; the closure runs with no active reactive Observer
//! so only the listed deps subscribe (Option A semantics, Refs #4195).
//! Effect closures can return either `()` for no cleanup or `Option<C>` when
//! they need to register teardown.

use reinhardt_core::reactive::deps::IntoDeps;

use crate::reactive::{Effect, runtime::EffectTiming};

/// Return value accepted from effect closures.
///
/// `()` means the effect has no cleanup. `Option<C>` preserves the existing
/// cleanup-capable form, where `Some(cleanup)` runs before the next re-run and
/// on dispose.
pub trait EffectReturn<C>
where
	C: FnOnce() + 'static,
{
	/// Converts the closure return value into an optional cleanup function.
	fn into_cleanup(self) -> Option<C>;
}

impl EffectReturn<fn()> for () {
	fn into_cleanup(self) -> Option<fn()> {
		None
	}
}

impl<C> EffectReturn<C> for Option<C>
where
	C: FnOnce() + 'static,
{
	fn into_cleanup(self) -> Option<C> {
		self
	}
}

/// Internal adapter from effect closures to accepted effect return values.
///
/// This keeps the public cleanup type as the second generic parameter while
/// still allowing closures to return either `()` or `Option<C>`.
#[doc(hidden)]
pub trait EffectCallback<C>
where
	C: FnOnce() + 'static,
{
	type Return: EffectReturn<C>;

	fn call_effect(&mut self) -> Self::Return;
}

impl<F, R, C> EffectCallback<C> for F
where
	F: FnMut() -> R,
	R: EffectReturn<C>,
	C: FnOnce() + 'static,
{
	type Return = R;

	fn call_effect(&mut self) -> Self::Return {
		self()
	}
}

/// Runs a side effect when one of the listed `deps` changes.
///
/// React-aligned equivalent of `useEffect(f, deps)`. The effect function
/// runs immediately, and re-runs whenever any of the dependencies listed
/// in `deps` changes. Signal reads inside `f` do **not** auto-subscribe.
///
/// # Reactivity Semantics
///
/// - The closure runs with no active reactive Observer
///   (`run_without_observer`); auto-tracking is disabled inside `f`.
/// - Subscriptions are derived exclusively from `deps`.
/// - Use `()` to opt out of re-runs (mount-only effect).
///
/// # Type Parameters
///
/// * `F` - The effect function type.
/// * `C` - The cleanup function type.
/// * `D` - Any tuple of [`Trackable`]s (or `()`) that implements
///   [`IntoDeps`].
///
/// # Arguments
///
/// * `f` - A function that performs the side effect and optionally
///   returns a cleanup function. Cleanups run before the next re-run and
///   on dispose, matching React `useEffect`.
/// * `deps` - The explicit dependency tuple. Pass `()` for no deps.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::{use_effect, use_state};
///
/// let (count, _set_count) = use_state(0);
///
/// // Effect without cleanup; re-runs only when `count` changes.
/// use_effect(
///     {
///         let count = count.clone();
///         move || {
///             log!("Count is now: {}", count.get());
///         }
///     },
///     (count.clone(),),
/// );
///
/// // Effect with cleanup, mount-only deps `()`.
/// use_effect(
///     move || {
///         let interval_id = set_interval(|| log!("tick"), 1000);
///         Some(move || clear_interval(interval_id))
///     },
///     (),
/// );
/// ```
///
/// [`Trackable`]: reinhardt_core::reactive::deps::Trackable
/// [`IntoDeps`]: reinhardt_core::reactive::deps::IntoDeps
pub fn use_effect<F, C, D>(f: F, deps: D) -> Effect
where
	F: EffectCallback<C> + 'static,
	C: FnOnce() + 'static,
	D: IntoDeps,
{
	let mut f = f;
	Effect::new_with_deps(move || f.call_effect().into_cleanup(), deps.into_deps())
}

/// Runs a side effect synchronously before browser paint when any listed
/// `dep` changes.
///
/// React-aligned equivalent of `useLayoutEffect(f, deps)`. Same Option A
/// semantics as [`use_effect`] but with [`EffectTiming::Layout`] so
/// re-runs propagate synchronously rather than via the passive scheduler.
///
/// # When to Use
///
/// Use `use_layout_effect` instead of [`use_effect`] when you need to:
/// - Read layout from the DOM and synchronously re-render
/// - Measure DOM elements
/// - Apply visual updates that must be synchronous
///
/// # Warning
///
/// `use_layout_effect` blocks the browser from painting, so it should be
/// used sparingly. Prefer [`use_effect`] for most use cases.
///
/// # Reactivity Semantics
///
/// See [`use_effect`] — identical, plus Layout timing.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::{use_layout_effect, use_ref, use_state};
///
/// let element_ref = use_ref(None::<Element>);
/// let (_width, set_width) = use_state(0);
///
/// use_layout_effect(
///     {
///         let element_ref = element_ref.clone();
///         let set_width = set_width.clone();
///         move || {
///             if let Some(el) = element_ref.current().as_ref() {
///                 set_width(el.offset_width());
///             }
///         }
///     },
///     (element_ref,),
/// );
/// ```
pub fn use_layout_effect<F, C, D>(f: F, deps: D) -> Effect
where
	F: EffectCallback<C> + 'static,
	C: FnOnce() + 'static,
	D: IntoDeps,
{
	let mut f = f;
	Effect::new_with_deps_and_timing(
		move || f.call_effect().into_cleanup(),
		deps.into_deps(),
		EffectTiming::Layout,
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use rstest::rstest;
	use serial_test::serial;
	use std::cell::RefCell;
	use std::rc::Rc;

	#[test]
	#[serial]
	fn test_use_effect_runs_immediately() {
		let called = Rc::new(RefCell::new(false));

		let _effect = use_effect(
			{
				let called = Rc::clone(&called);
				move || {
					*called.borrow_mut() = true;
					None::<fn()>
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
					let _ = count.get();
					*effect_count.borrow_mut() += 1;
					None::<fn()>
				}
			},
			(count.clone(),),
		);

		// Initial run
		assert_eq!(*effect_count.borrow(), 1);
	}

	#[rstest]
	#[serial(hooks_effect)]
	fn test_use_effect_accepts_unit_return() {
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
	fn test_use_layout_effect() {
		let called = Rc::new(RefCell::new(false));

		let _effect = use_layout_effect(
			{
				let called = Rc::clone(&called);
				move || {
					*called.borrow_mut() = true;
					None::<fn()>
				}
			},
			(),
		);

		assert!(*called.borrow());
	}

	#[test]
	#[serial]
	fn test_layout_effect_synchronous_execution() {
		let signal = Signal::new(0);
		let execution_order = Rc::new(RefCell::new(Vec::new()));

		let _effect = use_layout_effect(
			{
				let signal = signal.clone();
				let execution_order = Rc::clone(&execution_order);
				move || {
					let value = signal.get();
					execution_order.borrow_mut().push(value);
					None::<fn()>
				}
			},
			(signal.clone(),),
		);

		// Initial execution
		assert_eq!(*execution_order.borrow(), vec![0]);

		// Change signal - layout effect should execute synchronously
		signal.set(1);
		execution_order.borrow_mut().push(100);

		// Layout effect ran synchronously before the push(100)
		assert_eq!(*execution_order.borrow(), vec![0, 1, 100]);
	}

	#[rstest]
	#[serial(hooks_effect)]
	fn test_use_layout_effect_accepts_unit_return_and_tracks_synchronously() {
		let signal = Signal::new(0);
		let execution_order = Rc::new(RefCell::new(Vec::new()));

		let _effect = use_layout_effect(
			{
				let signal = signal.clone();
				let execution_order = Rc::clone(&execution_order);
				move || {
					execution_order.borrow_mut().push(signal.get());
				}
			},
			(signal.clone(),),
		);

		assert_eq!(*execution_order.borrow(), vec![0]);

		signal.set(1);
		execution_order.borrow_mut().push(100);

		assert_eq!(*execution_order.borrow(), vec![0, 1, 100]);
	}

	#[test]
	#[serial]
	fn test_layout_vs_passive_timing() {
		let signal = Signal::new(0);
		let layout_count = Rc::new(RefCell::new(0));
		let passive_count = Rc::new(RefCell::new(0));

		let _layout_effect = use_layout_effect(
			{
				let signal = signal.clone();
				let layout_count = Rc::clone(&layout_count);
				move || {
					let _ = signal.get();
					*layout_count.borrow_mut() += 1;
					None::<fn()>
				}
			},
			(signal.clone(),),
		);

		let _passive_effect = use_effect(
			{
				let signal = signal.clone();
				let passive_count = Rc::clone(&passive_count);
				move || {
					let _ = signal.get();
					*passive_count.borrow_mut() += 1;
					None::<fn()>
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
	}

	#[test]
	#[serial]
	fn test_mixed_layout_and_passive_effects() {
		let signal = Signal::new(0);
		let execution_order = Rc::new(RefCell::new(Vec::new()));

		let _layout_effect = use_layout_effect(
			{
				let signal = signal.clone();
				let execution_order = Rc::clone(&execution_order);
				move || {
					let value = signal.get();
					execution_order.borrow_mut().push(("layout", value));
					None::<fn()>
				}
			},
			(signal.clone(),),
		);

		let _passive_effect = use_effect(
			{
				let signal = signal.clone();
				let execution_order = Rc::clone(&execution_order);
				move || {
					let value = signal.get();
					execution_order.borrow_mut().push(("passive", value));
					None::<fn()>
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
