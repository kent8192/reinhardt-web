//! Signal - Fine-grained Reactive Primitive
//!
//! `Signal<T>` is the core reactive primitive that holds a value and automatically
//! tracks dependencies when accessed.
//!
//! ## Key Features
//!
//! - **Automatic Dependency Tracking**: When `get()` is called inside an Effect or Memo,
//!   the dependency is automatically recorded.
//! - **Change Notification**: When `set()` or `update()` is called, all dependent Effects
//!   are automatically scheduled for re-execution.
//! - **Lightweight**: `Signal<T>` is a copied key, so it can be passed around directly.
//! - **Type-safe**: The value type is enforced at compile time.
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_core::reactive::{ReactiveScope, Signal};
//!
//! ReactiveScope::run(|| {
//!     // Create a signal
//!     let count = Signal::new(0);
//!
//!     // Read the value
//!     assert_eq!(count.get(), 0);
//!
//!     // Update the value
//!     count.set(42);
//!     assert_eq!(count.get(), 42);
//!
//!     // Update with a function
//!     count.update(|n| *n += 1);
//!     assert_eq!(count.get(), 43);
//! });
//! ```

use core::fmt;
use core::marker::PhantomData;

#[cfg(target_arch = "wasm32")]
use core::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::RwLock;

use super::runtime::{NodeId, with_runtime};
use super::scope::{
	NodeKey, NodeKind, ReactiveScopeError, allocate_node, require_active_scope, with_node,
	with_node_mut,
};

#[cfg(not(target_arch = "wasm32"))]
type SignalValue<T> = RwLock<T>;
#[cfg(target_arch = "wasm32")]
type SignalValue<T> = RefCell<T>;

struct SignalSlot<T> {
	value: SignalValue<T>,
}

impl<T> SignalSlot<T> {
	fn new(value: T) -> Self {
		Self {
			value: SignalValue::new(value),
		}
	}
}

/// A reactive signal that holds a value and tracks dependencies
///
/// `Signal<T>` is the fundamental building block of the reactive system. It represents
/// a piece of state that can change over time, and automatically notifies dependent
/// computations when it changes.
///
/// ## Type Parameter
///
/// * `T` - The type of value stored in the signal. Must be `'static` to ensure memory safety.
///
/// ## Copying
///
/// `Signal<T>` is a copied key into the current [`super::ReactiveScope`]. The
/// owning scope stores the value and controls its lifetime.
///
/// ## Thread affinity
///
/// A signal key records the thread that owns its reactive scope. Access from a
/// different thread is rejected rather than resolving a same-numbered scope
/// from that thread's local arena.
///
/// Signals are neither `Send` nor `Sync`, even when `T` is. Their storage is
/// owned by a thread-local reactive scope.
///
/// ```compile_fail
/// use reinhardt_core::reactive::Signal;
///
/// fn assert_send_sync<T: Send + Sync>() {}
///
/// assert_send_sync::<Signal<i32>>();
/// ```
pub struct Signal<T: 'static> {
	key: NodeKey,
	_marker: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for Signal<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: 'static> Copy for Signal<T> {}

/// Converts an owned or borrowed signal expression into its copied handle.
pub trait IntoSignalHandle<T: 'static> {
	/// Returns the copied signal handle.
	fn into_signal_handle(self) -> Signal<T>;
}

impl<T: 'static> IntoSignalHandle<T> for Signal<T> {
	fn into_signal_handle(self) -> Signal<T> {
		self
	}
}

impl<T: 'static> IntoSignalHandle<T> for &Signal<T> {
	fn into_signal_handle(self) -> Signal<T> {
		*self
	}
}

/// Copies a reactive signal handle from either an owned or borrowed expression.
pub fn copy_signal_handle<T: 'static>(signal: impl IntoSignalHandle<T>) -> Signal<T> {
	signal.into_signal_handle()
}

impl<T: 'static> Signal<T> {
	/// Create a new Signal with the given initial value
	///
	/// # Arguments
	///
	/// * `value` - Initial value for the signal
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_core::reactive::{ReactiveScope, Signal};
	///
	/// ReactiveScope::run(|| {
	///     let count = Signal::new(0);
	///     assert_eq!(count.get(), 0);
	/// });
	/// ```
	pub fn new(value: T) -> Self {
		require_active_scope("Signal::new");
		Self {
			key: allocate_node(NodeKind::Signal, SignalSlot::new(value)),
			_marker: PhantomData,
		}
	}

	/// Get the current value of the signal
	///
	/// This automatically tracks the dependency if called from within an Effect or Memo.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_core::reactive::{ReactiveScope, Signal};
	///
	/// ReactiveScope::run(|| {
	///     let count = Signal::new(42);
	///     assert_eq!(count.get(), 42);
	/// });
	/// ```
	pub fn get(&self) -> T
	where
		T: Clone,
	{
		// Track dependency with the runtime
		with_runtime(|rt| rt.track_dependency(self.id()));

		// Get the value from storage
		self.get_untracked()
	}

	/// Get the current value without tracking dependencies
	///
	/// This is useful when you want to read a signal's value without creating
	/// a dependency relationship.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_core::reactive::{ReactiveScope, Signal};
	///
	/// ReactiveScope::run(|| {
	///     let count = Signal::new(42);
	///     // This won't create a dependency
	///     let value = count.get_untracked();
	///     assert_eq!(value, 42);
	/// });
	/// ```
	pub fn get_untracked(&self) -> T
	where
		T: Clone,
	{
		self.try_get_untracked()
			.unwrap_or_else(|err| panic!("{err}"))
	}

	/// Get the current value without tracking dependencies if the owner is live.
	///
	/// This non-panicking form is intended for asynchronous completion paths and
	/// external notifications that can arrive after the owning scope is disposed.
	pub fn try_get_untracked(&self) -> Result<T, ReactiveScopeError>
	where
		T: Clone,
	{
		with_node::<SignalSlot<T>, _>(self.key, |slot| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				slot.value.read().expect("Signal lock poisoned").clone()
			}
			#[cfg(target_arch = "wasm32")]
			{
				slot.value.borrow().clone()
			}
		})
	}

	/// Applies a function to the current value without cloning or tracking dependencies.
	///
	/// This is more efficient than `get_untracked` when you only need to inspect
	/// the value and do not need an owned copy.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_core::reactive::{ReactiveScope, Signal};
	///
	/// ReactiveScope::run(|| {
	///     let count = Signal::new(42);
	///     let is_positive = count.with_untracked(|n| *n > 0);
	///     assert!(is_positive);
	/// });
	/// ```
	pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
		with_node::<SignalSlot<T>, _>(self.key, |slot| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				f(&slot.value.read().expect("Signal lock poisoned"))
			}
			#[cfg(target_arch = "wasm32")]
			{
				f(&slot.value.borrow())
			}
		})
		.unwrap_or_else(|err| panic!("{err}"))
	}

	/// Set the signal to a new value
	///
	/// This notifies all dependent Effects and Memos that the signal has changed.
	///
	/// # Arguments
	///
	/// * `value` - New value for the signal
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_core::reactive::{ReactiveScope, Signal};
	///
	/// ReactiveScope::run(|| {
	///     let count = Signal::new(0);
	///     count.set(42);
	///     assert_eq!(count.get(), 42);
	/// });
	/// ```
	pub fn set(&self, value: T) {
		self.try_set(value).unwrap_or_else(|err| panic!("{err}"));
	}

	pub(crate) fn set_without_notify(&self, value: T) {
		with_node_mut::<SignalSlot<T>, _>(self.key, |slot| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				*slot.value.write().expect("Signal lock poisoned") = value;
			}
			#[cfg(target_arch = "wasm32")]
			{
				*slot.value.borrow_mut() = value;
			}
		})
		.unwrap_or_else(|err| panic!("{err}"));
	}

	pub(crate) fn notify_subscribers(&self) {
		with_runtime(|rt| rt.notify_signal_change(self.id()));
	}

	/// Set the signal if its owning scope is still alive.
	///
	/// This non-panicking form is intended for asynchronous completion paths
	/// that may race with scope disposal.
	pub fn try_set(&self, value: T) -> Result<(), ReactiveScopeError> {
		with_node_mut::<SignalSlot<T>, _>(self.key, |slot| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				*slot.value.write().expect("Signal lock poisoned") = value;
			}
			#[cfg(target_arch = "wasm32")]
			{
				*slot.value.borrow_mut() = value;
			}
		})?;
		with_runtime(|rt| rt.notify_signal_change(self.id()));
		Ok(())
	}

	/// Update the signal's value using a function
	///
	/// This is more efficient than `get()` + `set()` because it only notifies
	/// dependents once.
	///
	/// # Arguments
	///
	/// * `f` - Function that takes a mutable reference to the current value
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_core::reactive::{ReactiveScope, Signal};
	///
	/// ReactiveScope::run(|| {
	///     let count = Signal::new(0);
	///     count.update(|n| *n += 1);
	///     assert_eq!(count.get(), 1);
	/// });
	/// ```
	pub fn update<F>(&self, f: F)
	where
		F: FnOnce(&mut T),
	{
		with_node_mut::<SignalSlot<T>, _>(self.key, |slot| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				f(&mut slot.value.write().expect("Signal lock poisoned"));
			}
			#[cfg(target_arch = "wasm32")]
			{
				f(&mut slot.value.borrow_mut());
			}
		})
		.unwrap_or_else(|err| panic!("{err}"));
		with_runtime(|rt| rt.notify_signal_change(self.id()));
	}

	/// Get the NodeId of this signal
	///
	/// This is mainly for internal use by the runtime and tests.
	pub fn id(&self) -> NodeId {
		self.key.node_id()
	}
}

impl<T: fmt::Debug + Clone + 'static> fmt::Debug for Signal<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Signal")
			.field("id", &self.id())
			.field("value", &self.get_untracked())
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::runtime::NodeType;
	use crate::reactive::{Effect, batch};
	use rstest::rstest;
	use serial_test::serial;
	use std::cell::Cell;
	use std::rc::Rc;

	#[rstest]
	#[serial(reactive_runtime)]
	fn signal_is_copy() {
		fn assert_copy<T: Copy>() {}

		assert_copy::<Signal<i32>>();
	}

	#[rstest]
	#[serial(reactive_runtime)]
	#[should_panic(expected = "Signal::new requires an active ReactiveScope")]
	fn signal_new_requires_scope() {
		let _ = Signal::new(1_i32);
	}

	#[rstest]
	#[serial(reactive_runtime)]
	#[should_panic(expected = "disposed reactive node access")]
	fn signal_panics_after_scope_dispose() {
		let signal = crate::reactive::ReactiveScope::run(|| Signal::new(1_i32));
		let _ = signal.get();
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn signal_try_set_discards_completion_after_scope_dispose() {
		let signal = crate::reactive::ReactiveScope::run(|| Signal::new(1_i32));

		let result = signal.try_set(2);

		assert!(result.is_err());
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn signal_update_can_read_a_different_signal() {
		crate::reactive::ReactiveScope::run(|| {
			let source = Signal::new(41_i32);
			let target = Signal::new(0_i32);

			target.update(|value| *value = source.get() + 1);

			assert_eq!(target.get(), 42);
		});
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn signal_with_untracked_can_reenter_a_different_signal() {
		crate::reactive::ReactiveScope::run(|| {
			let source = Signal::new(1_i32);
			let target = Signal::new(0_i32);

			source.with_untracked(|_| target.set(42));

			assert_eq!(target.get(), 42);
		});
	}

	#[rstest]
	#[serial(reactive_runtime)]
	fn signal_tracks_and_updates_inside_scope() {
		crate::reactive::ReactiveScope::run(|| {
			let signal = Signal::new(1_i32);
			assert_eq!(signal.get(), 1);
			signal.set(2);
			assert_eq!(signal.get_untracked(), 2);
			signal.update(|value| *value += 3);
			assert_eq!(signal.get(), 5);
		});
	}

	#[test]
	#[serial]
	fn test_signal_creation() {
		crate::reactive::ReactiveScope::run(|| {
			let signal = Signal::new(42);
			assert_eq!(signal.get_untracked(), 42);
		});
	}

	#[test]
	#[serial]
	fn test_signal_set() {
		crate::reactive::ReactiveScope::run(|| {
			let signal = Signal::new(0);
			assert_eq!(signal.get_untracked(), 0);

			signal.set(100);
			assert_eq!(signal.get_untracked(), 100);
		});
	}

	#[test]
	#[serial]
	fn test_signal_update() {
		crate::reactive::ReactiveScope::run(|| {
			let signal = Signal::new(0);

			signal.update(|n| *n += 1);
			assert_eq!(signal.get_untracked(), 1);

			signal.update(|n| *n *= 2);
			assert_eq!(signal.get_untracked(), 2);
		});
	}

	#[test]
	#[serial]
	fn test_signal_clone() {
		crate::reactive::ReactiveScope::run(|| {
			let signal1 = Signal::new(42);
			let signal2 = signal1;

			assert_eq!(signal1.get_untracked(), 42);
			assert_eq!(signal2.get_untracked(), 42);

			signal1.set(100);
			assert_eq!(signal1.get_untracked(), 100);
			assert_eq!(signal2.get_untracked(), 100);
		});
	}

	#[test]
	#[serial]
	fn test_multiple_signals() {
		crate::reactive::ReactiveScope::run(|| {
			let signal1 = Signal::new(10);
			let signal2 = Signal::new(20);
			let signal3 = Signal::new("hello");

			assert_eq!(signal1.get_untracked(), 10);
			assert_eq!(signal2.get_untracked(), 20);
			assert_eq!(signal3.get_untracked(), "hello");

			signal1.set(30);
			signal2.set(40);
			signal3.set("world");

			assert_eq!(signal1.get_untracked(), 30);
			assert_eq!(signal2.get_untracked(), 40);
			assert_eq!(signal3.get_untracked(), "world");
		});
	}

	#[test]
	#[serial]
	fn test_signal_dependency_tracking() {
		crate::reactive::ReactiveScope::run(|| {
			let signal = Signal::new(42);

			// Without observer, get() should still work
			assert_eq!(signal.get(), 42);

			// With observer, dependency should be tracked
			with_runtime(|rt| {
				let observer_id = NodeId::new();
				rt.push_observer(crate::reactive::runtime::Observer {
					id: observer_id,
					node_type: NodeType::Effect,
					timing: crate::reactive::runtime::EffectTiming::default(),
					cleanup: None,
				});

				// This should track the dependency
				let _ = signal.get();

				rt.pop_observer();

				// Verify dependency was tracked
				let graph = rt.dependency_graph.borrow();
				let signal_node = graph.get(&signal.id()).unwrap();
				assert!(signal_node.subscribers.contains(&observer_id));
			});
		});
	}

	#[test]
	#[serial]
	fn test_signal_change_notification() {
		crate::reactive::ReactiveScope::run(|| {
			let signal = Signal::new(0);
			let signal_for_effect = signal;
			let effect = crate::reactive::Effect::new(move || {
				let _ = signal_for_effect.get();
			});

			// Change the signal.
			signal.set(42);

			// Verify the effect was scheduled for update.
			with_runtime(|rt| {
				let pending = rt.pending_updates.borrow();
				assert!(pending.contains(&effect.id()));
			});
		});
	}

	#[test]
	#[serial]
	fn test_batched_signal_change_notifies_effect() {
		crate::reactive::ReactiveScope::run(|| {
			let signal = Signal::new(0);
			let runs = Rc::new(Cell::new(0));
			let runs_for_effect = Rc::clone(&runs);
			let _effect = Effect::new(move || {
				let _ = signal.get();
				runs_for_effect.set(runs_for_effect.get() + 1);
			});

			assert_eq!(runs.get(), 1);

			batch(|| signal.set(42));

			assert_eq!(runs.get(), 2);
		});
	}
}
