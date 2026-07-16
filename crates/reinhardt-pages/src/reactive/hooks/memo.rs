//! Memoization hooks: use_memo, use_callback, use_callback_with
//!
//! React-aligned hooks built on top of [`Memo::new_with_deps`] and the
//! `callback_with_deps` Rc-swap helper. All three take an explicit
//! dependency tuple as the second argument (Refs #4195).

use crate::callback::{Callback, callback_with_deps};
use crate::reactive::{ExplicitDeps, Memo, ReactiveDeps};

/// Memoizes an expensive calculation.
///
/// This is the React-like equivalent of `useMemo`. The calculation is re-run
/// only when its reactive dependencies change.
///
/// Reinhardt Pages uses an explicit dependency tuple instead of a React
/// dependency array. Signal reads inside the calculation do not subscribe
/// implicitly; the tuple passed as `deps` determines when the memo re-runs.
///
/// # Type Parameters
///
/// * `T` - The return type of the calculation
/// * `F` - The calculation function type
///
/// # Arguments
///
/// * `f` - A function that performs the calculation
/// * `deps` - Explicit dependency tuple; pass `()` for mount-only memoization
///
/// # Returns
///
/// A `Memo<T>` that can be read with `.get()`
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::reactive::hooks::{use_state, use_memo};
///
/// let (items, _set_items) = use_state(vec![1, 2, 3, 4, 5]);
/// let (filter, _set_filter) = use_state(2);
///
/// // Expensive filtering operation - only re-runs when items or filter change
/// let filtered = use_memo(
///     move || {
///         items.get()
///             .into_iter()
///             .filter(|&x| x > filter.get())
///             .collect::<Vec<_>>()
///     },
///     (items, filter),
/// );
///
/// // Reading the memoized value
/// let result = filtered.get();
/// ```
pub fn use_memo<T, F>(f: F, deps: impl Into<ReactiveDeps>) -> Memo<T>
where
	T: Clone + 'static,
	F: FnMut() -> T + 'static,
{
	Memo::new_with_mode(f, deps.into())
}

/// Memoizes a callback function to maintain a stable reference.
///
/// This is the React-like equivalent of `useCallback`. It wraps a function
/// in a `Callback` that maintains a stable identity, preventing unnecessary
/// re-renders of child components.
///
/// # Type Parameters
///
/// * `F` - The callback function type
///
/// # Arguments
///
/// * `f` - The callback function to memoize
///
/// # Returns
///
/// A `Callback<Args, ()>` that can be passed to typed event handlers
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::{SetStateExt, use_callback, use_state};
/// use reinhardt_pages::page;
///
/// let (count, set_count) = use_state(0);
///
/// // Memoized callback - reference won't change between renders
/// let increment = use_callback(
///     {
///         let set_count = set_count.clone();
///         move |_event| {
///             set_count.update(|current| current + 1);
///         }
///     },
///     (),
/// );
///
/// page!(|| {
///     button {
///         @click: increment,
///         "Increment"
///     }
/// })
/// ```
///
/// # Note
///
/// Reinhardt uses an explicit dependency tuple rather than a JavaScript array.
/// To use the latest values, capture reactive handles directly because they are
/// `Copy`, rather than capturing value snapshots. Reference-counted setters may
/// still need cloning.
#[cfg(wasm)]
#[track_caller]
pub fn use_callback<Args, F>(f: F, deps: ExplicitDeps) -> Callback<Args, ()>
where
	F: Fn(Args) + 'static,
	Args: 'static,
{
	callback_with_deps::<Args, ()>(f, deps.into_deps())
}

/// Memoizes a callback function to maintain a stable reference (server-side version).
///
/// See the WASM version for full documentation.
/// Native callbacks share the thread-affine reactive scope contract.
#[cfg(native)]
#[track_caller]
pub fn use_callback<Args, F>(f: F, deps: ExplicitDeps) -> Callback<Args, ()>
where
	F: Fn(Args) + 'static,
	Args: 'static,
{
	callback_with_deps::<Args, ()>(f, deps.into_deps())
}

/// Creates a memoized callback with custom argument and return types.
///
/// This is a more flexible version of `use_callback` that allows specifying
/// custom argument and return types.
///
/// # Type Parameters
///
/// * `Args` - The argument type
/// * `Ret` - The return type
/// * `F` - The callback function type
///
/// # Arguments
///
/// * `f` - The callback function to memoize
///
/// # Returns
///
/// A `Callback<Args, Ret>`
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::reactive::hooks::use_callback_with;
///
/// let add = use_callback_with(|x: i32| x + 1, ());
/// assert_eq!(add.call(5), 6);
/// ```
#[cfg(wasm)]
#[track_caller]
pub fn use_callback_with<Args, Ret, F>(f: F, deps: ExplicitDeps) -> Callback<Args, Ret>
where
	F: Fn(Args) -> Ret + 'static,
	Args: 'static,
	Ret: 'static,
{
	callback_with_deps::<Args, Ret>(f, deps.into_deps())
}

/// Creates a memoized callback with custom argument and return types (server-side version).
///
/// See the WASM version for full documentation.
/// Native callbacks share the thread-affine reactive scope contract.
#[cfg(native)]
#[track_caller]
pub fn use_callback_with<Args, Ret, F>(f: F, deps: ExplicitDeps) -> Callback<Args, Ret>
where
	F: Fn(Args) -> Ret + 'static,
	Args: 'static,
	Ret: 'static,
{
	callback_with_deps::<Args, Ret>(f, deps.into_deps())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use serial_test::serial;

	#[test]
	#[serial]
	fn test_use_memo_basic() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let memo = use_memo(|| 42, ());
			assert_eq!(memo.get(), 42);
		});
	}

	#[test]
	#[serial]
	fn test_use_memo_with_signal() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let count = Signal::new(5);

			let doubled = use_memo(
				{
					let count = count.clone();
					move || count.get() * 2
				},
				(count.clone(),),
			);

			assert_eq!(doubled.get(), 10);
		});
	}

	#[test]
	#[serial]
	fn test_use_memo_complex() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let items = Signal::new(vec![1, 2, 3, 4, 5]);

			let sum = use_memo(
				{
					let items = items.clone();
					move || items.get().iter().sum::<i32>()
				},
				(items.clone(),),
			);

			assert_eq!(sum.get(), 15);
		});
	}

	#[cfg(native)]
	#[test]
	fn test_use_callback_accepts_typed_event_arguments() {
		use crate::component::NativeEvent;
		use crate::event::{ClickEvent, EventPayload};
		use reinhardt_core::types::page::{EventType, NativeEventPayload, PointerEventData};

		reinhardt_core::reactive::ReactiveScope::run(|| {
			let callback: Callback<ClickEvent, ()> = use_callback(
				|event: ClickEvent| {
					assert_eq!(event.event_type(), "click");
				},
				(),
			);
			let raw = NativeEvent::for_known(
				EventType::Click,
				NativeEventPayload::Pointer(PointerEventData::default()),
			);
			callback.call(ClickEvent::try_from_raw(raw).expect("click payload must convert"));
		});
	}

	#[cfg(native)]
	#[test]
	fn test_use_callback_with() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			let add_one = use_callback_with::<i32, i32, _, _>(|x: i32| x + 1, ());
			assert_eq!(add_one.call(5), 6);

			let concat =
				use_callback_with::<String, String, _, _>(|s: String| format!("Hello, {}", s), ());
			assert_eq!(concat.call("World".to_string()), "Hello, World");
		});
	}
}
