//! Memoization hooks: use_memo, use_callback, use_callback_with
//!
//! React-aligned hooks built on top of [`Memo::new_with_deps`] and the
//! `callback_with_deps` Rc-swap helper. Memo calculations accept explicit or
//! automatic dependency modes; callbacks require an explicit list (Refs #4195).

use crate::callback::{Callback, callback_with_deps};
use crate::reactive::{ExplicitDeps, Memo, ReactiveDeps};

/// Memoizes an expensive calculation.
///
/// This is the React-like equivalent of `useMemo`. The calculation is re-run
/// only when its reactive dependencies change.
///
/// Reinhardt Pages uses an explicit `deps![...]` list or `deps_auto!()` instead
/// of a React dependency array. Signal reads inside the calculation subscribe
/// only in automatic mode; an explicit list determines when the memo re-runs.
/// `deps![...]` subscribes only to the listed reactive values, `deps![]` runs
/// the calculation once and retains its value until disposal, and
/// `deps_auto!()` rebuilds subscriptions from tracked reads on every setup.
///
/// # Type Parameters
///
/// * `T` - The return type of the calculation
/// * `F` - The calculation function type
///
/// # Arguments
///
/// * `f` - A function that performs the calculation
/// * `deps` - Explicit dependency list or `deps_auto!()`; pass `deps![]` for
///   mount-only memoization
///
/// # Returns
///
/// A `Memo<T>` that can be read with `.get()`
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::deps;
/// use reinhardt_pages::reactive::hooks::{use_state, use_memo};
///
/// let (items, set_items) = use_state(vec![1, 2, 3, 4, 5]);
/// let (filter, set_filter) = use_state(2);
///
/// // Expensive filtering operation - only re-runs when items or filter change
/// let filtered = use_memo(
///     {
///         let items = items.clone();
///         let filter = filter.clone();
///         move || {
///             items.get()
///                 .into_iter()
///                 .filter(|&x| x > filter.get())
///                 .collect::<Vec<_>>()
///         }
///     },
///     deps![items, filter],
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
/// use reinhardt_pages::deps;
/// use reinhardt_pages::reactive::hooks::{SetStateExt, use_callback, use_state};
/// use reinhardt_pages::page;
///
/// let (count, set_count) = use_state(0);
///
/// // Memoized callback - reference won't change between renders
/// let increment = use_callback({
///     let set_count = set_count.clone();
///     move |_event| {
///         set_count.update(|current| current + 1);
///     }
/// }, deps![]);
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
/// Unlike React's dependency arrays, Reinhardt Pages uses an explicit
/// `deps![...]` dependency list. Capture Signals (which are cheap to clone) rather
/// than their values when the callback should observe the latest state.
/// Automatic dependencies are not supported because the callback body executes
/// after construction.
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
/// This hook requires explicit `deps![...]`; automatic dependencies are not
/// supported because the callback body executes after construction.
/// Requires `Send + Sync` bounds for thread-safe server-side usage.
#[cfg(native)]
#[track_caller]
pub fn use_callback<Args, F>(f: F, deps: ExplicitDeps) -> Callback<Args, ()>
where
	F: Fn(Args) + Send + Sync + 'static,
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
/// This hook requires explicit `deps![...]`; automatic dependencies are not
/// supported because the callback body executes after construction.
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::deps;
/// use reinhardt_pages::reactive::hooks::use_callback_with;
///
/// let add = use_callback_with(|x: i32| x + 1, deps![]);
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
/// This hook requires explicit `deps![...]`; automatic dependencies are not
/// supported because the callback body executes after construction.
/// Requires `Send + Sync` bounds for thread-safe server-side usage.
#[cfg(native)]
#[track_caller]
pub fn use_callback_with<Args, Ret, F>(f: F, deps: ExplicitDeps) -> Callback<Args, Ret>
where
	F: Fn(Args) -> Ret + Send + Sync + 'static,
	Args: 'static,
	Ret: 'static,
{
	callback_with_deps::<Args, Ret>(f, deps.into_deps())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use reinhardt_core::deps;
	use serial_test::serial;

	#[test]
	#[serial]
	fn test_use_memo_basic() {
		let memo = use_memo(|| 42, deps![]);
		assert_eq!(memo.get(), 42);
	}

	#[test]
	#[serial]
	fn test_use_memo_with_signal() {
		let count = Signal::new(5);

		let doubled = use_memo(
			{
				let count = count.clone();
				move || count.get() * 2
			},
			deps![count],
		);

		assert_eq!(doubled.get(), 10);
	}

	#[test]
	#[serial]
	fn test_use_memo_complex() {
		let items = Signal::new(vec![1, 2, 3, 4, 5]);

		let sum = use_memo(
			{
				let items = items.clone();
				move || items.get().iter().sum::<i32>()
			},
			deps![items],
		);

		assert_eq!(sum.get(), 15);
	}

	#[cfg(native)]
	#[test]
	fn test_use_callback_accepts_typed_event_arguments() {
		use crate::component::NativeEvent;
		use crate::event::{ClickEvent, EventPayload};
		use reinhardt_core::types::page::{EventType, NativeEventPayload, PointerEventData};

		let callback: Callback<ClickEvent, ()> = use_callback(
			|event: ClickEvent| {
				assert_eq!(event.event_type(), "click");
			},
			deps![],
		);
		let raw = NativeEvent::for_known(
			EventType::Click,
			NativeEventPayload::Pointer(PointerEventData::default()),
		);
		callback.call(ClickEvent::try_from_raw(raw).expect("click payload must convert"));
	}

	#[cfg(native)]
	#[test]
	fn test_use_callback_with() {
		let add_one = use_callback_with::<i32, i32, _>(|x: i32| x + 1, deps![]);
		assert_eq!(add_one.call(5), 6);

		let concat =
			use_callback_with::<String, String, _>(|s: String| format!("Hello, {}", s), deps![]);
		assert_eq!(concat.call("World".to_string()), "Hello, World");
	}
}
