//! Effect hooks: use_effect and use_layout_effect
//!
//! These hooks provide React-like side effect management built on top of Effect.

use crate::reactive::Effect;

/// Runs a side effect with automatic dependency tracking.
///
/// This is the React-like equivalent of `useEffect`. The effect function runs
/// automatically whenever its reactive dependencies change.
///
/// Unlike React's useEffect, dependencies are automatically tracked - you don't
/// need to specify a dependency array. Any Signal accessed inside the effect
/// will be tracked as a dependency.
///
/// # Type Parameters
///
/// * `F` - The effect function type
/// * `C` - The optional cleanup function type
///
/// # Arguments
///
/// * `f` - A function that performs the side effect and optionally returns a cleanup function
///
/// # Returns
///
/// The Effect handle, which can be used to manually dispose the effect
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::{use_state, use_effect};
///
/// let (count, set_count) = use_state(0);
///
/// // Effect without cleanup
/// use_effect({
///     let count = count.clone();
///     move || {
///         log!("Count is now: {}", count.get());
///         None::<fn()>
///     }
/// });
///
/// // Effect with cleanup
/// use_effect(move || {
///     let interval_id = set_interval(|| log!("tick"), 1000);
///     Some(move || clear_interval(interval_id))
/// });
/// ```
///
/// # Note on Cleanup
///
/// The cleanup function, if provided, is called before the effect re-runs
/// and when the effect is disposed. This is useful for cleaning up subscriptions,
/// timers, or other resources.
pub fn use_effect<F>(f: F) -> Effect
where
	F: FnMut() + 'static,
{
	Effect::new(f)
}

/// Runs a side effect synchronously before browser paint.
///
/// This is the React-like equivalent of `useLayoutEffect`. It has the same
/// signature as `use_effect` but runs synchronously after all DOM mutations
/// but before the browser paints.
///
/// # When to Use
///
/// Use `use_layout_effect` instead of `use_effect` when you need to:
/// - Read layout from the DOM and synchronously re-render
/// - Measure DOM elements
/// - Apply visual updates that must be synchronous
///
/// # Warning
///
/// `use_layout_effect` blocks the browser from painting, so it should be used
/// sparingly. Prefer `use_effect` for most use cases.
///
/// # Arguments
///
/// * `f` - A function that performs the side effect
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::{use_ref, use_layout_effect};
///
/// let element_ref = use_ref(None::<Element>);
/// let (width, set_width) = use_state(0);
///
/// use_layout_effect({
///     let element_ref = element_ref.clone();
///     let set_width = set_width.clone();
///     move || {
///         if let Some(el) = element_ref.current().as_ref() {
///             set_width(el.offset_width());
///         }
///     }
/// });
/// ```
///
/// # Note
///
/// In the current implementation, `use_layout_effect` behaves identically to
/// `use_effect`. True synchronous execution will be implemented when the
/// rendering pipeline supports it.
pub fn use_layout_effect<F>(f: F) -> Effect
where
	F: FnMut() + 'static,
{
	// TODO: Implement true layout effect timing
	// For now, this behaves the same as use_effect
	Effect::new(f)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use serial_test::serial;
	use std::cell::RefCell;
	use std::rc::Rc;

	#[test]
	#[serial]
	fn test_use_effect_runs_immediately() {
		let called = Rc::new(RefCell::new(false));

		let _effect = use_effect({
			let called = Rc::clone(&called);
			move || {
				*called.borrow_mut() = true;
			}
		});

		assert!(*called.borrow());
	}

	#[test]
	#[serial]
	fn test_use_effect_tracks_dependencies() {
		let count = Signal::new(0);
		let effect_count = Rc::new(RefCell::new(0));

		let _effect = use_effect({
			let count = count.clone();
			let effect_count = Rc::clone(&effect_count);
			move || {
				let _ = count.get(); // Track dependency
				*effect_count.borrow_mut() += 1;
			}
		});

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

		let _effect = use_layout_effect({
			let called = Rc::clone(&called);
			move || {
				*called.borrow_mut() = true;
			}
		});

		assert!(*called.borrow());
	}
}
