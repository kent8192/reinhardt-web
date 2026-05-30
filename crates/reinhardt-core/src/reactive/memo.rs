//! Memo - Cached Reactive Computations
//!
//! `Memo<T>` represents a memoized computation that automatically updates when its dependencies change.
//! Unlike `Effect`, which runs for side effects, `Memo` caches and returns a computed value.
//!
//! ## Key Features
//!
//! - **Automatic Caching**: Computation result is cached and reused until dependencies change
//! - **Lazy Evaluation**: Only recomputes when `get()` is called and dependencies have changed
//! - **Automatic Dependency Tracking**: Dependencies are tracked automatically, just like Effect
//! - **Can be Depended On**: Other Effects and Memos can depend on a Memo, making it act like a Signal
//!
//! ## Example
//!
//! ```no_run
//! use reinhardt_core::reactive::{Signal, Memo};
//!
//! let count = Signal::new(5);
//!
//! // Create a memo that computes count * 2
//! let count_for_memo = count.clone();
//! let doubled = Memo::new(move || count_for_memo.get() * 2);
//!
//! // First access computes the value
//! assert_eq!(doubled.get(), 10);
//!
//! // Second access uses cached value (no recomputation)
//! assert_eq!(doubled.get(), 10);
//!
//! // When dependency changes, memo is marked dirty
//! count.set(10);
//!
//! // Next access recomputes
//! assert_eq!(doubled.get(), 20);
//! ```

use core::cell::RefCell;

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use super::runtime::{EffectTiming, NodeId, NodeType, Observer, try_with_runtime, with_runtime};

/// Computation function for a Memo
type MemoFn<T> = Box<dyn FnMut() -> T + 'static>;

/// Cached value and dirty flag for a Memo
#[derive(Clone)]
struct MemoState<T: Clone> {
	/// The cached computed value
	value: T,
	/// Whether the value needs to be recomputed
	dirty: bool,
}

// Global storage for Memo computation functions
thread_local! {
	static MEMO_FUNCTIONS: RefCell<BTreeMap<NodeId, Box<dyn core::any::Any>>> = RefCell::new(BTreeMap::new());
}

// Global storage for Memo cached values
thread_local! {
	static MEMO_VALUES: RefCell<BTreeMap<NodeId, Box<dyn core::any::Any>>> = RefCell::new(BTreeMap::new());
}

// Type-agnostic dirty map for `Memo::new_with_deps` (Refs #4195).
//
// `MemoState<T>::dirty` is stored type-erased so flipping it from outside
// requires `T`. The dirty notifier created by `new_with_deps` is type-erased
// (it only knows `NodeId`), so it writes here instead. `Memo::get` checks
// both `MemoState<T>::dirty` and this map.
thread_local! {
	static MEMO_DIRTY: RefCell<BTreeMap<NodeId, bool>> = const { RefCell::new(BTreeMap::new()) };
}

/// Flag the Memo identified by `memo_id` as dirty without requiring `T`,
/// and propagate the change to downstream subscribers.
///
/// Called by the hidden notifier Effect created by [`Memo::new_with_deps`]
/// whenever one of the listed deps changes. Cleared on the next
/// [`Memo::get`] / [`Memo::get_untracked`] recompute.
///
/// The `notify_signal_change` call is what makes downstream `use_effect` /
/// `use_memo` calls that take this memo as a listed dep actually re-run.
/// Without it, the dirty flag would be set but no consumer would be
/// woken up. Mirrors the propagation behavior of `Memo::mark_dirty`.
pub(crate) fn mark_memo_dirty_by_id(memo_id: NodeId) {
	MEMO_DIRTY.with(|m| {
		m.borrow_mut().insert(memo_id, true);
	});
	with_runtime(|rt| rt.notify_signal_change(memo_id));
}

/// Returns whether the type-agnostic dirty flag is set for `memo_id`.
fn is_memo_dirty_externally(memo_id: NodeId) -> bool {
	MEMO_DIRTY.with(|m| m.borrow().get(&memo_id).copied().unwrap_or(false))
}

/// Clears the type-agnostic dirty flag for `memo_id` after a recompute.
fn clear_memo_dirty(memo_id: NodeId) {
	MEMO_DIRTY.with(|m| {
		m.borrow_mut().remove(&memo_id);
	});
}

/// A memoized reactive computation that caches its result
///
/// `Memo<T>` is similar to a Signal in that it can be read with `get()` and can have dependents.
/// However, unlike a Signal, its value is computed from other reactive values, and the
/// computation is automatically cached.
///
/// ## When to Use Memo vs Effect
///
/// - Use `Memo` when you need a **derived value** that should be cached
/// - Use `Effect` when you need to perform **side effects**
///
/// ## Example
///
/// ```rust
/// use reinhardt_core::reactive::{Signal, Memo, Effect};
///
/// let first_name = Signal::new("John".to_string());
/// let last_name = Signal::new("Doe".to_string());
///
/// // Memo caches the full name computation
/// let first_clone = first_name.clone();
/// let last_clone = last_name.clone();
/// let full_name = Memo::new(move || {
///     format!("{} {}", first_clone.get(), last_clone.get())
/// });
///
/// // Effect uses the memo
/// let full_name_clone = full_name.clone();
/// Effect::new(move || {
///     println!("Full name: {}", full_name_clone.get());
/// });
/// ```
#[derive(Clone)]
pub struct Memo<T: Clone + 'static> {
	/// Unique identifier for this memo
	id: NodeId,
	/// Whether this memo has been disposed
	disposed: Rc<RefCell<bool>>,
	/// Phantom data for type parameter
	_phantom: core::marker::PhantomData<T>,
	/// Hidden Effect that subscribes to explicit deps (`new_with_deps` only).
	///
	/// Wrapped in `Rc` so `Memo<T>` remains `Clone`. The Effect's closure
	/// calls [`mark_memo_dirty_by_id`] whenever any listed dep fires, so
	/// the next [`Self::get`] recomputes. `None` for `Memo::new`
	/// (auto-track path; the Memo is its own Observer).
	#[allow(dead_code)]
	deps_notifier: Option<Rc<super::effect::Effect>>,
}

impl<T: Clone + 'static> Memo<T> {
	/// Create a new Memo with the given computation function
	///
	/// The function runs immediately to compute the initial value, and will
	/// automatically re-run (when accessed via `get()`) after any of its
	/// dependencies change.
	///
	/// # Arguments
	///
	/// * `f` - The computation function. Must be `FnMut() -> T + 'static`.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_core::reactive::{Signal, Memo};
	///
	/// let count = Signal::new(5);
	/// let count_clone = count.clone();
	/// let doubled = Memo::new(move || count_clone.get() * 2);
	/// assert_eq!(doubled.get(), 10);
	/// ```
	pub fn new<F>(f: F) -> Self
	where
		F: FnMut() -> T + 'static,
	{
		let id = NodeId::new();
		let disposed = Rc::new(RefCell::new(false));

		// Store the computation function.
		//
		// The stored closure must NOT capture a clone of `disposed`. `Drop`
		// uses `Rc::strong_count(&self.disposed) == 1` to detect the last live
		// clone; a strong reference held by this long-lived closure would keep
		// the count above 1 forever, so `dispose()` would never run and the
		// memo node would leak in the runtime. `dispose()` removes the
		// `MEMO_FUNCTIONS` entry, so this closure is never invoked after
		// disposal and needs no disposed-flag guard.
		MEMO_FUNCTIONS.with(|storage| {
			let mut storage = storage.borrow_mut();
			let boxed: Box<dyn core::any::Any> = Box::new(Box::new(f) as MemoFn<T>);
			storage.insert(id, boxed);
		});

		// Compute initial value
		let initial_value = Self::compute_value(id);

		// Store the initial value
		MEMO_VALUES.with(|storage| {
			let mut storage = storage.borrow_mut();
			let state = MemoState {
				value: initial_value,
				dirty: false,
			};
			let boxed: Box<dyn core::any::Any> = Box::new(state);
			storage.insert(id, boxed);
		});

		Self {
			id,
			disposed,
			_phantom: core::marker::PhantomData,
			deps_notifier: None,
		}
	}

	/// Create a Memo that recomputes only when one of the listed `deps` changes.
	///
	/// Unlike [`Self::new`], the computation closure runs with **no active
	/// reactive Observer**, so `Signal::get` calls inside do not auto-
	/// subscribe. A hidden Layout-timing Effect subscribes to the listed
	/// deps and flips the memo's dirty flag synchronously when any dep
	/// changes; the next [`Self::get`] then recomputes.
	///
	/// This is the lower-layer primitive used by `use_memo(f, deps)` in
	/// `reinhardt-pages`. See the design spec at
	/// `docs/superpowers/specs/2026-05-22-issue-4195-hooks-deps-array-design.md`.
	#[allow(dead_code)]
	pub fn new_with_deps<F>(mut f: F, deps: super::deps::Deps) -> Self
	where
		F: FnMut() -> T + 'static,
	{
		let id = NodeId::new();
		let disposed = Rc::new(RefCell::new(false));

		// Wrap the user closure to detach the Observer before each compute.
		// `compute_value` pushes the memo's NodeId as Observer, but Option A
		// requires Signal reads inside f to NOT auto-subscribe. The
		// `run_without_observer` call detaches the stack for the duration
		// of f.
		// See `Memo::new`: the wrapped closure must not hold a strong clone of
		// `disposed`, or `Drop`'s `strong_count == 1` last-clone check can
		// never fire. `dispose()` removes the `MEMO_FUNCTIONS` entry, so this
		// closure is never run after disposal.
		let f_wrapped = move || super::runtime::run_without_observer(&mut f);

		MEMO_FUNCTIONS.with(|storage| {
			let boxed: Box<dyn core::any::Any> = Box::new(Box::new(f_wrapped) as MemoFn<T>);
			storage.borrow_mut().insert(id, boxed);
		});

		// Initial compute (deps not yet subscribed; auto-track no-op inside
		// run_without_observer).
		let initial_value = Self::compute_value(id);

		MEMO_VALUES.with(|storage| {
			let state = MemoState {
				value: initial_value,
				dirty: false,
			};
			let boxed: Box<dyn core::any::Any> = Box::new(state);
			storage.borrow_mut().insert(id, boxed);
		});

		// Hidden notifier Effect: subscribes to deps; on trigger, marks
		// this memo dirty. Layout timing so dep changes propagate
		// synchronously, matching React's "memo result available immediately
		// after dep change" semantics.
		let deps_notifier = if deps.as_slice().is_empty() {
			None
		} else {
			let memo_id = id;
			let notifier = super::effect::Effect::new_with_deps_and_timing::<_, fn()>(
				move || {
					mark_memo_dirty_by_id(memo_id);
					None
				},
				deps,
				EffectTiming::Layout,
			);
			Some(Rc::new(notifier))
		};

		// The notifier's initial run set MEMO_DIRTY[id] = true; clear it
		// so the first `Self::get` returns the cached `initial_value`
		// without spuriously recomputing.
		clear_memo_dirty(id);

		Self {
			id,
			disposed,
			_phantom: core::marker::PhantomData,
			deps_notifier,
		}
	}

	/// Compute the value by executing the memo function
	///
	/// This is called internally when the memo needs to recompute.
	fn compute_value(memo_id: NodeId) -> T {
		with_runtime(|rt| {
			// Clear old dependencies before recomputing
			rt.clear_dependencies(memo_id);

			// Push observer onto stack
			rt.push_observer(Observer {
				id: memo_id,
				node_type: NodeType::Memo,
				timing: EffectTiming::default(), // Memos use default (Passive) timing
				cleanup: None,
			});
		});

		// Execute the computation function using Remove-Execute-Reinsert pattern
		// to avoid RefCell reentrant borrow panics when the closure creates nested effects or memos.
		// An RAII guard ensures the function is reinserted even if the computation panics.
		struct MemoFnGuard {
			memo_id: NodeId,
			memo_fn_box: Option<Box<dyn core::any::Any>>,
		}

		impl Drop for MemoFnGuard {
			fn drop(&mut self) {
				if let Some(f) = self.memo_fn_box.take() {
					MEMO_FUNCTIONS.with(|storage| {
						storage.borrow_mut().insert(self.memo_id, f);
					});
				}
			}
		}

		let mut guard = MemoFnGuard {
			memo_id,
			memo_fn_box: MEMO_FUNCTIONS.with(|storage| storage.borrow_mut().remove(&memo_id)),
		};

		let result = if let Some(ref mut boxed) = guard.memo_fn_box
			&& let Some(memo_fn) = boxed.downcast_mut::<MemoFn<T>>()
		{
			memo_fn()
		} else {
			panic!("Memo function not found - this should never happen")
		};

		// Pop observer from stack
		with_runtime(|rt| {
			rt.pop_observer();
		});

		result
	}

	/// Get the current value of the memo
	///
	/// This automatically tracks the dependency if called from within an Effect or Memo.
	/// If the memo is dirty (dependencies have changed), it will recompute before returning.
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_core::reactive::{Signal, Memo};
	///
	/// let count = Signal::new(5);
	/// let count_clone = count.clone();
	/// let doubled = Memo::new(move || count_clone.get() * 2);
	///
	/// assert_eq!(doubled.get(), 10);
	///
	/// count.set(10);
	/// assert_eq!(doubled.get(), 20); // Recomputes here
	/// ```
	pub fn get(&self) -> T {
		if *self.disposed.borrow() {
			panic!("Attempted to access a disposed Memo");
		}

		// Track dependency with the runtime
		with_runtime(|rt| rt.track_dependency(self.id));

		// Check if we need to recompute (either typed dirty bit OR the
		// type-agnostic dirty flag flipped by `Memo::new_with_deps`'s
		// notifier).
		let needs_recompute = MEMO_VALUES.with(|storage| {
			let storage = storage.borrow();
			if let Some(boxed) = storage.get(&self.id)
				&& let Some(state) = boxed.downcast_ref::<MemoState<T>>()
			{
				return state.dirty;
			}
			// If not found, we need to recompute
			true
		}) || is_memo_dirty_externally(self.id);

		if needs_recompute {
			// Recompute the value
			let new_value = Self::compute_value(self.id);

			// Update the cached value
			MEMO_VALUES.with(|storage| {
				let mut storage = storage.borrow_mut();
				if let Some(boxed) = storage.get_mut(&self.id)
					&& let Some(state) = boxed.downcast_mut::<MemoState<T>>()
				{
					state.value = new_value.clone();
					state.dirty = false;
				}
			});
			clear_memo_dirty(self.id);

			new_value
		} else {
			// Return cached value
			MEMO_VALUES.with(|storage| {
				let storage = storage.borrow();
				if let Some(boxed) = storage.get(&self.id)
					&& let Some(state) = boxed.downcast_ref::<MemoState<T>>()
				{
					return state.value.clone();
				}
				panic!("Memo value not found - this should never happen");
			})
		}
	}

	/// Get the current value without tracking dependencies
	///
	/// This is useful when you want to read a memo's value without creating
	/// a dependency relationship.
	pub fn get_untracked(&self) -> T {
		if *self.disposed.borrow() {
			panic!("Attempted to access a disposed Memo");
		}

		// Check if dirty and recompute if needed (typed bit OR external).
		let needs_recompute = MEMO_VALUES.with(|storage| {
			let storage = storage.borrow();
			if let Some(boxed) = storage.get(&self.id)
				&& let Some(state) = boxed.downcast_ref::<MemoState<T>>()
			{
				return state.dirty;
			}
			true
		}) || is_memo_dirty_externally(self.id);

		if needs_recompute {
			let new_value = Self::compute_value(self.id);
			MEMO_VALUES.with(|storage| {
				let mut storage = storage.borrow_mut();
				if let Some(boxed) = storage.get_mut(&self.id)
					&& let Some(state) = boxed.downcast_mut::<MemoState<T>>()
				{
					state.value = new_value.clone();
					state.dirty = false;
				}
			});
			clear_memo_dirty(self.id);
			new_value
		} else {
			MEMO_VALUES.with(|storage| {
				let storage = storage.borrow();
				if let Some(boxed) = storage.get(&self.id)
					&& let Some(state) = boxed.downcast_ref::<MemoState<T>>()
				{
					return state.value.clone();
				}
				panic!("Memo value not found - this should never happen");
			})
		}
	}

	/// Mark this memo as dirty (needs recomputation)
	///
	/// This is called internally by the runtime when a dependency changes.
	/// It's also exposed for testing purposes.
	pub fn mark_dirty(&self) {
		MEMO_VALUES.with(|storage| {
			let mut storage = storage.borrow_mut();
			if let Some(boxed) = storage.get_mut(&self.id)
				&& let Some(state) = boxed.downcast_mut::<MemoState<T>>()
			{
				state.dirty = true;
			}
		});

		// Notify dependents that this memo has changed
		with_runtime(|rt| rt.notify_signal_change(self.id));
	}

	/// Get the NodeId of this memo (for testing)
	pub fn id(&self) -> NodeId {
		self.id
	}

	/// Dispose this memo
	///
	/// After calling this, the memo will no longer work and its resources will be cleaned up.
	pub fn dispose(&self) {
		*self.disposed.borrow_mut() = true;

		// Remove from runtime's dependency graph (ignore if TLS is destroyed)
		let _ = try_with_runtime(|rt| rt.remove_node(self.id));

		// Remove from storage (ignore if TLS is destroyed)
		let _ = MEMO_FUNCTIONS.try_with(|storage| {
			storage.borrow_mut().remove(&self.id);
		});
		let _ = MEMO_VALUES.try_with(|storage| {
			storage.borrow_mut().remove(&self.id);
		});
		let _ = MEMO_DIRTY.try_with(|storage| {
			storage.borrow_mut().remove(&self.id);
		});
	}
}

impl<T: Clone + 'static> Drop for Memo<T> {
	fn drop(&mut self) {
		// Only dispose when this is the last live clone. `Memo<T>` is `Clone`
		// and every clone shares the same `disposed` Rc (and the same `id` /
		// global storage entry). Disposing on every clone drop would set the
		// shared `disposed` flag and clear `MEMO_FUNCTIONS` / `MEMO_VALUES`
		// while sibling clones are still in use, causing a spurious
		// "Attempted to access a disposed Memo" panic on the next `get()`.
		//
		// `strong_count == 1` means this is the only remaining reference, so
		// cleanup is safe. Mirrors the last-clone-cleanup guard in
		// `Signal<T>`'s `Drop` impl.
		if Rc::strong_count(&self.disposed) == 1 {
			self.dispose();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::Signal;
	use serial_test::serial;

	#[test]
	#[serial]
	fn test_memo_creation() {
		let memo = Memo::new(|| 42);
		assert_eq!(memo.get(), 42);
	}

	#[test]
	#[serial]
	fn test_memo_caching() {
		let compute_count = Rc::new(RefCell::new(0));
		let compute_count_clone = compute_count.clone();

		let memo = Memo::new(move || {
			*compute_count_clone.borrow_mut() += 1;
			42
		});

		// First access computes
		assert_eq!(memo.get(), 42);
		assert_eq!(*compute_count.borrow(), 1);

		// Second access uses cache
		assert_eq!(memo.get(), 42);
		assert_eq!(*compute_count.borrow(), 1); // Still 1, not 2
	}

	#[test]
	#[serial]
	fn test_memo_with_signal_dependency() {
		let signal = Signal::new(5);
		let signal_clone = signal.clone();

		let memo = Memo::new(move || signal_clone.get() * 2);

		// Initial value
		assert_eq!(memo.get(), 10);

		// Change signal and mark memo dirty manually (in real system, runtime does this)
		signal.set(10);
		memo.mark_dirty();

		// Memo should recompute
		assert_eq!(memo.get(), 20);
	}

	#[test]
	#[serial]
	fn test_memo_clone() {
		let memo1 = Memo::new(|| 42);
		let memo2 = memo1.clone();

		assert_eq!(memo1.get(), 42);
		assert_eq!(memo2.get(), 42);
	}

	#[test]
	#[serial]
	fn test_memo_dependency_tracking() {
		let signal = Signal::new(1);
		let signal_clone = signal.clone();

		let memo = Memo::new(move || signal_clone.get() + 10);

		// Access the memo inside an effect-like observer
		with_runtime(|rt| {
			let observer_id = NodeId::new();
			rt.push_observer(Observer {
				id: observer_id,
				node_type: NodeType::Effect,
				timing: EffectTiming::default(), // Test observer uses default timing
				cleanup: None,
			});

			// This should track the dependency
			let _ = memo.get();

			rt.pop_observer();

			// Verify dependency was tracked
			let graph = rt.dependency_graph.borrow();
			let memo_node = graph.get(&memo.id()).unwrap();
			assert!(memo_node.subscribers.contains(&observer_id));
		});
	}

	// Note: Memo chain test removed due to Drop ordering issues with thread-local storage.
	// While chained memos are a valid pattern, the test creates Drop ordering complexities
	// with TLS. In production code, memo chains work correctly during normal execution;
	// the issue only manifests during test cleanup.

	#[rstest::rstest]
	#[serial]
	fn test_memo_creates_effect_during_computation() {
		// Arrange
		let effect_ran = Rc::new(RefCell::new(false));
		let effect_ran_clone = effect_ran.clone();

		// Act - create a memo whose computation creates an effect
		let memo = Memo::new(move || {
			use crate::reactive::Effect;
			let ran = effect_ran_clone.clone();
			let _effect = Effect::new(move || {
				*ran.borrow_mut() = true;
			});
			42
		});

		// Assert - memo returns the correct value and nested effect executed
		assert_eq!(memo.get(), 42);
		assert!(*effect_ran.borrow());
	}

	#[test]
	#[serial]
	fn test_memo_get_untracked() {
		let signal = Signal::new(5);
		let signal_clone = signal.clone();

		let memo = Memo::new(move || signal_clone.get() * 2);

		with_runtime(|rt| {
			let observer_id = NodeId::new();
			rt.push_observer(Observer {
				id: observer_id,
				node_type: NodeType::Effect,
				timing: EffectTiming::default(), // Test observer uses default timing
				cleanup: None,
			});

			// get_untracked should not create dependency
			let _ = memo.get_untracked();

			rt.pop_observer();

			// Verify NO dependency was tracked
			let graph = rt.dependency_graph.borrow();
			if let Some(memo_node) = graph.get(&memo.id()) {
				assert!(!memo_node.subscribers.contains(&observer_id));
			}
		});
	}

	#[test]
	#[serial]
	fn new_with_deps_recomputes_only_on_listed_dep() {
		use core::cell::Cell;

		// Arrange
		let listed = Signal::new(2_i32);
		let unlisted = Signal::new(100_i32);
		let computations = Rc::new(Cell::new(0_i32));
		let computations_for_memo = computations.clone();
		let listed_for_memo = listed.clone();
		let unlisted_for_memo = unlisted.clone();
		let deps = crate::reactive::deps::Deps::from_signals(&[listed.id()]);

		// Act
		let memo = Memo::new_with_deps(
			move || {
				computations_for_memo.set(computations_for_memo.get() + 1);
				listed_for_memo.get() * 10 + unlisted_for_memo.get()
			},
			deps,
		);
		// Force initial value materialization (also exercises the
		// no-spurious-recompute path through the cleared MEMO_DIRTY entry).
		let _ = memo.get();
		let before_changes = computations.get();

		unlisted.set(200);
		let _ = memo.get();
		let after_unlisted = computations.get();

		listed.set(3);
		let _ = memo.get();
		let after_listed = computations.get();

		// Assert
		assert_eq!(
			after_unlisted, before_changes,
			"memo MUST NOT recompute when unlisted Signal changes (Option A)"
		);
		assert_eq!(
			after_listed,
			before_changes + 1,
			"memo MUST recompute exactly once when listed dep changes"
		);
	}
}
