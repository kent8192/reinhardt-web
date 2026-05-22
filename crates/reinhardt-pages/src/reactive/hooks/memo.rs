//! Memoization hooks: `use_memo`, `use_callback`, `use_callback_with`.
//!
//! React-parity hooks for memoized values and stable callbacks. All three
//! take an explicit deps tuple as the final positional argument — exact
//! parity with React's `useMemo(fn, [deps])` / `useCallback(fn, [deps])`.
//! Missing the deps argument is a compile error (`error[E0061]`).
//!
//! # Deps semantics
//!
//! Same as [`use_effect`](super::effect::use_effect):
//!
//! - `()` — mount-only.
//! - `(s,)` .. `(s1, .., s12)` — recompute / re-stabilize when any listed
//!   dep fires.

use crate::callback::Callback;
use crate::reactive::Memo;
use crate::reactive::deps::IntoDeps;

#[cfg(wasm)]
type EventArg = web_sys::Event;

#[cfg(native)]
type EventArg = crate::component::DummyEvent;

/// Memoizes an expensive calculation, recomputing only when a listed dep fires.
///
/// React parity: `useMemo(fn, [deps])`. The first argument is the
/// calculation closure; the second is a tuple of [`Trackable`]
/// dependencies. Pass `()` for a value that is computed once and never
/// updated.
///
/// # Type Parameters
///
/// - `T` — the cached value type.
/// - `F` — the calculation closure.
/// - `D` — the deps tuple shape.
///
/// # Returns
///
/// A [`Memo<T>`] that can be read with `.get()`.
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::reactive::Signal;
/// use reinhardt_pages::reactive::hooks::use_memo;
///
/// let items = Signal::new(vec![1, 2, 3, 4, 5]);
/// let filter = Signal::new(2);
///
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
///     (items.clone(), filter.clone()),
/// );
///
/// let _ = filtered.get();
/// ```
///
/// [`Trackable`]: crate::reactive::Trackable
pub fn use_memo<T, F, D>(f: F, deps: D) -> Memo<T>
where
	T: Clone + 'static,
	F: FnMut() -> T + 'static,
	D: IntoDeps,
{
	Memo::new_with_deps(f, deps.into_deps())
}

/// Memoizes a callback to maintain a stable reference (WASM).
///
/// React parity: `useCallback(fn, [deps])`. Wraps the closure in a
/// [`Callback`] that other components can clone cheaply. The deps tuple
/// is recorded for forward compatibility with future identity-based
/// re-stabilization; for now the callback's identity is stable for the
/// lifetime of the returned handle (it never re-creates), so the deps
/// argument behaves purely as a contract check.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::Signal;
/// use reinhardt_pages::reactive::hooks::use_callback;
///
/// let count = Signal::new(0);
///
/// let increment = use_callback(
///     {
///         let count = count.clone();
///         move |_event| {
///             count.update(|n| *n += 1);
///         }
///     },
///     (count.clone(),),
/// );
/// ```
#[cfg(wasm)]
pub fn use_callback<F, D>(f: F, deps: D) -> Callback<EventArg, ()>
where
	F: Fn(EventArg) + 'static,
	D: IntoDeps,
{
	let _ = deps.into_deps();
	Callback::new(f)
}

/// Memoizes a callback to maintain a stable reference (server-side).
///
/// See the WASM version for full documentation. Requires `Send + Sync`
/// bounds for thread-safe server-side usage.
#[cfg(native)]
pub fn use_callback<F, D>(f: F, deps: D) -> Callback<EventArg, ()>
where
	F: Fn(EventArg) + Send + Sync + 'static,
	D: IntoDeps,
{
	let _ = deps.into_deps();
	Callback::new(f)
}

/// Creates a memoized callback with custom argument and return types (WASM).
///
/// React-style escape hatch from the default `(EventArg) -> ()` shape.
/// Takes the same explicit deps tuple as [`use_callback`].
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
pub fn use_callback_with<Args, Ret, F, D>(f: F, deps: D) -> Callback<Args, Ret>
where
	F: Fn(Args) -> Ret + 'static,
	D: IntoDeps,
{
	let _ = deps.into_deps();
	Callback::new(f)
}

/// Creates a memoized callback with custom argument and return types (server-side).
///
/// See the WASM version for full documentation. Requires `Send + Sync`.
#[cfg(native)]
pub fn use_callback_with<Args, Ret, F, D>(f: F, deps: D) -> Callback<Args, Ret>
where
	F: Fn(Args) -> Ret + Send + Sync + 'static,
	D: IntoDeps,
{
	let _ = deps.into_deps();
	Callback::new(f)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use rstest::rstest;
	use serial_test::serial;

	#[rstest]
	#[serial]
	fn use_memo_with_explicit_deps() {
		// Arrange
		let count = Signal::new(3_i32);

		// Act
		let doubled = use_memo(
			{
				let count = count.clone();
				move || count.get() * 2
			},
			(count.clone(),),
		);

		// Assert
		assert_eq!(doubled.get(), 6);
	}

	#[test]
	#[serial]
	fn test_use_memo_basic() {
		let memo = use_memo(|| 42, ());
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
			(count.clone(),),
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
			(items.clone(),),
		);

		assert_eq!(sum.get(), 15);
	}

	#[cfg(native)]
	#[rstest]
	#[serial]
	fn use_callback_with_explicit_deps() {
		use crate::component::DummyEvent;
		let count = Signal::new(0_i32);

		let cb = use_callback(
			{
				let count = count.clone();
				move |_: DummyEvent| {
					let _ = count.get();
				}
			},
			(count.clone(),),
		);

		cb.call(DummyEvent::default());
	}

	#[cfg(native)]
	#[test]
	fn test_use_callback() {
		use crate::component::DummyEvent;

		let callback = use_callback(|_: DummyEvent| {}, ());
		callback.call(DummyEvent::default());
	}

	#[cfg(native)]
	#[test]
	fn test_use_callback_with() {
		let add_one = use_callback_with(|x: i32| x + 1, ());
		assert_eq!(add_one.call(5), 6);

		let concat = use_callback_with(|s: String| format!("Hello, {}", s), ());
		assert_eq!(concat.call("World".to_string()), "Hello, World");
	}
}
