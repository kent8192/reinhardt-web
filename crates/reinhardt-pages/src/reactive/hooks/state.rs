//! State hooks: use_state, use_shared_state, and use_reducer
//!
//! These hooks provide React-like state management built on top of Signal.
//!
//! ## Choosing Between use_state and use_shared_state
//!
//! - **use_state**: For single-threaded, WASM-only contexts
//!   - Lighter weight (uses `Rc<RefCell<T>>`)
//!   - Not Send + Sync
//!   - Best for client-side UI state
//!
//! - **use_shared_state**: For multi-threaded contexts
//!   - Thread-safe (uses `Arc<Mutex<T>>`)
//!   - Implements Send + Sync
//!   - Required for server-side event handlers
//!   - Slightly higher overhead due to mutex locking

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::reactive::Signal;
use crate::reactive::runtime::{NodeId, with_runtime};

/// A setter function for updating state.
///
/// This is a cloneable function wrapper that updates the associated Signal.
/// Import [`SetStateExt`] to use previous-value updates with `set.update(...)`.
pub type SetState<T> = Rc<dyn Fn(T)>;

thread_local! {
	static SET_STATE_SIGNALS: RefCell<HashMap<usize, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

struct SetStateRegistration {
	key: usize,
}

impl Drop for SetStateRegistration {
	fn drop(&mut self) {
		let _ = SET_STATE_SIGNALS.try_with(|signals| {
			signals.borrow_mut().remove(&self.key);
		});
	}
}

struct RegisteredSetState<T: 'static> {
	signal: Signal<T>,
	_registration: Rc<RefCell<Option<SetStateRegistration>>>,
}

impl<T: 'static> RegisteredSetState<T> {
	fn set(&self, value: T) {
		self.signal.set(value);
	}
}

/// Extension methods for setters returned by [`use_state`].
pub trait SetStateExt<T: Clone + 'static> {
	/// Replace the state value.
	///
	/// This is equivalent to calling the setter as a function.
	fn set(&self, value: T);

	/// Derive and store the next state value from the current value.
	///
	/// The updater receives the current value by shared reference and returns
	/// the replacement value. The underlying signal notifies dependents once.
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_pages::reactive::hooks::{SetStateExt, use_state};
	///
	/// let (count, set_count) = use_state(0);
	/// set_count.update(|current| current + 1);
	/// ```
	fn update<F>(&self, f: F)
	where
		F: FnOnce(&T) -> T;
}

impl<T: Clone + 'static> SetStateExt<T> for SetState<T> {
	fn set(&self, value: T) {
		self.as_ref()(value);
	}

	fn update<F>(&self, f: F)
	where
		F: FnOnce(&T) -> T,
	{
		let signal = registered_set_state_signal(self).unwrap_or_else(|| {
			panic!("SetStateExt::update is only available on setters returned by use_state")
		});
		let current = signal.get_untracked();
		signal.set(f(&current));
	}
}

fn set_state_key<T>(setter: &SetState<T>) -> usize {
	Rc::as_ptr(setter) as *const () as usize
}

fn register_set_state_signal<T: Clone + 'static>(
	setter: &SetState<T>,
	signal: Signal<T>,
) -> SetStateRegistration {
	let key = set_state_key(setter);
	SET_STATE_SIGNALS.with(|signals| {
		signals.borrow_mut().insert(key, Box::new(signal));
	});
	SetStateRegistration { key }
}

fn registered_set_state_signal<T: Clone + 'static>(setter: &SetState<T>) -> Option<Signal<T>> {
	SET_STATE_SIGNALS.with(|signals| {
		signals
			.borrow()
			.get(&set_state_key(setter))
			.and_then(|signal| signal.downcast_ref::<Signal<T>>())
			.cloned()
	})
}

/// A dispatch function for reducer actions.
///
/// This is a cloneable function wrapper that dispatches actions to the reducer.
pub type Dispatch<A> = Rc<dyn Fn(A)>;

/// Creates a reactive state with a setter function.
///
/// This is the React-like equivalent of `useState`. It returns a tuple containing
/// the Signal (for reading) and a setter function (for updating).
///
/// # Arguments
///
/// * `initial` - The initial value for the state
///
/// # Returns
///
/// A tuple of `(Signal<T>, SetState<T>)` where:
/// - `Signal<T>` - The reactive state that can be read with `.get()`
/// - `SetState<T>` - A function to update the state
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::reactive::hooks::{SetStateExt, use_state};
///
/// let (count, set_count) = use_state(0);
///
/// // Read the value
/// let current = count.get();
///
/// // Update the value
/// set_count(current + 1);
///
/// // Or derive the next value from the current one
/// set_count.update(|current| current + 1);
/// ```
pub fn use_state<T: Clone + 'static>(initial: T) -> (Signal<T>, SetState<T>) {
	let signal = Signal::new(initial);
	let registration = Rc::new(RefCell::new(None));
	let setter: SetState<T> = {
		let state = RegisteredSetState {
			signal: signal.clone(),
			_registration: Rc::clone(&registration),
		};
		Rc::new(move |value| state.set(value))
	};
	*registration.borrow_mut() = Some(register_set_state_signal(&setter, signal.clone()));
	(signal, setter)
}

/// Creates state with a reducer function for complex state logic.
///
/// This is the React-like equivalent of `useReducer`. It's useful when state
/// logic is complex or when the next state depends on the previous state.
///
/// # Type Parameters
///
/// * `S` - The state type
/// * `A` - The action type
/// * `R` - The reducer function type
///
/// # Arguments
///
/// * `reducer` - A function that takes the current state and an action, returning the new state
/// * `initial` - The initial state value
///
/// # Returns
///
/// A tuple of `(Signal<S>, Dispatch<A>)` where:
/// - `Signal<S>` - The reactive state
/// - `Dispatch<A>` - A function to dispatch actions
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::reactive::hooks::use_reducer;
///
/// #[derive(Clone)]
/// struct State { count: i32 }
///
/// enum Action { Increment, Decrement, Reset }
///
/// fn reducer(state: &State, action: Action) -> State {
///     match action {
///         Action::Increment => State { count: state.count + 1 },
///         Action::Decrement => State { count: state.count - 1 },
///         Action::Reset => State { count: 0 },
///     }
/// }
///
/// let (state, dispatch) = use_reducer(reducer, State { count: 0 });
///
/// // Dispatch actions
/// dispatch(Action::Increment);
/// dispatch(Action::Increment);
/// assert_eq!(state.get().count, 2);
/// ```
///
/// # Reactivity semantics
///
/// The reducer receives `&S` (the current state) and an action — it does not
/// interact with reactive primitives directly. The surrounding `dispatch`
/// closure calls `Signal::get()` and `Signal::set()` outside the reducer
/// invocation, so the reducer body itself cannot create subscriptions.
/// This aligns with React's `useReducer` semantics where the reducer is a
/// pure function of (state, action) → state (Refs #4195).
pub fn use_reducer<S, A, R>(reducer: R, initial: S) -> (Signal<S>, Dispatch<A>)
where
	S: Clone + 'static,
	A: 'static,
	R: Fn(&S, A) -> S + 'static,
{
	let state = Signal::new(initial);
	let dispatch: Dispatch<A> = {
		let state = state.clone();
		let reducer = Rc::new(reducer);
		Rc::new(move |action: A| {
			let current = state.get();
			let new_state = reducer(&current, action);
			state.set(new_state);
		})
	};
	(state, dispatch)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Thread-safe state hooks (Send + Sync)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Thread-safe reactive signal using `Arc<Mutex<T>>`.
///
/// This is a Send + Sync version of `Signal<T>` for use in multi-threaded
/// contexts (e.g., server-side event handlers).
///
/// # Type Parameter
///
/// * `T` - The type of value stored in the signal. Must be `'static` to ensure memory safety.
///
/// # Cloning
///
/// `SharedSignal<T>` implements `Clone` and shares the value via `Arc<Mutex<T>>`.
/// All clones of the same SharedSignal share the same underlying value and reference count.
#[derive(Clone)]
pub struct SharedSignal<T: 'static> {
	/// Unique identifier for this signal
	id: NodeId,
	/// The actual value, shared via `Arc<Mutex<T>>`
	value: Arc<Mutex<T>>,
}

/// A thread-safe setter function for updating shared state.
///
/// This is a cloneable function wrapper that updates the associated SharedSignal.
pub type SharedSetState<T> = Arc<dyn Fn(T) + Send + Sync>;

impl<T: 'static> SharedSignal<T> {
	/// Create a new SharedSignal with the given initial value
	///
	/// # Arguments
	///
	/// * `value` - Initial value for the signal
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_pages::reactive::hooks::SharedSignal;
	///
	/// let count = SharedSignal::new(0);
	/// assert_eq!(count.get(), 0);
	/// ```
	pub fn new(value: T) -> Self
	where
		T: Send + Sync,
	{
		Self {
			id: NodeId::new(),
			value: Arc::new(Mutex::new(value)),
		}
	}

	/// Get the current value of the signal
	///
	/// This automatically tracks the dependency if called from within an Effect or Memo.
	/// This locks the mutex to read the value. Panics if the mutex is poisoned.
	///
	/// # Panics
	///
	/// Panics if the mutex is poisoned (another thread panicked while holding the lock).
	/// This is intentional as mutex poisoning indicates a serious bug.
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_pages::reactive::hooks::SharedSignal;
	///
	/// let count = SharedSignal::new(42);
	/// assert_eq!(count.get(), 42);
	/// ```
	pub fn get(&self) -> T
	where
		T: Clone,
	{
		// Track dependency with the runtime
		with_runtime(|rt| rt.track_dependency(self.id));

		// Get the value from storage
		self.get_untracked()
	}

	/// Get the current value without tracking dependencies
	///
	/// This is useful when you want to read a signal's value without creating
	/// a dependency relationship.
	///
	/// # Panics
	///
	/// Panics if the mutex is poisoned.
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_pages::reactive::hooks::SharedSignal;
	///
	/// let count = SharedSignal::new(42);
	/// // This won't create a dependency
	/// let value = count.get_untracked();
	/// ```
	pub fn get_untracked(&self) -> T
	where
		T: Clone,
	{
		self.value
			.lock()
			.expect("SharedSignal mutex poisoned")
			.clone()
	}

	/// Set the signal to a new value
	///
	/// This notifies all dependent Effects and Memos that the signal has changed.
	///
	/// # Arguments
	///
	/// * `value` - New value for the signal
	///
	/// # Panics
	///
	/// Panics if the mutex is poisoned.
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_pages::reactive::hooks::SharedSignal;
	///
	/// let count = SharedSignal::new(0);
	/// count.set(42);
	/// assert_eq!(count.get(), 42);
	/// ```
	pub fn set(&self, value: T) {
		*self.value.lock().expect("SharedSignal mutex poisoned") = value;
		with_runtime(|rt| rt.notify_signal_change(self.id));
	}

	/// Update the signal's value using a function
	///
	/// This is more efficient than `get()` + `set()` because it only locks
	/// the mutex once and notifies dependents once.
	///
	/// # Arguments
	///
	/// * `f` - Function that takes a mutable reference to the current value
	///
	/// # Panics
	///
	/// Panics if the mutex is poisoned.
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_pages::reactive::hooks::SharedSignal;
	///
	/// let count = SharedSignal::new(0);
	/// count.update(|n| *n += 1);
	/// assert_eq!(count.get(), 1);
	/// ```
	pub fn update<F>(&self, f: F)
	where
		F: FnOnce(&mut T),
	{
		f(&mut *self.value.lock().expect("SharedSignal mutex poisoned"));
		with_runtime(|rt| rt.notify_signal_change(self.id));
	}

	/// Get the NodeId of this signal
	///
	/// This is mainly for internal use by the runtime and tests.
	pub fn id(&self) -> NodeId {
		self.id
	}
}

/// Creates a thread-safe reactive state with a setter function.
///
/// This is similar to `use_state()` but uses `Arc<Mutex<T>>` internally
/// to ensure thread-safety. Use this when you need to share state across
/// threads or in event handlers that require `Send + Sync`.
///
/// # Type Parameters
///
/// * `T` - The type of value to store. Must be `Clone + Send + Sync + 'static`
///
/// # Arguments
///
/// * `initial` - The initial value for the state
///
/// # Returns
///
/// A tuple of `(SharedSignal<T>, SharedSetState<T>)` where:
/// - `SharedSignal<T>` - The reactive state that can be read with `.get()`
/// - `SharedSetState<T>` - A thread-safe function to update the state
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::reactive::hooks::use_shared_state;
///
/// let (count, set_count) = use_shared_state(0);
///
/// // Read the value
/// let current = count.get();
///
/// // Update the value
/// set_count(current + 1);
///
/// // Clone and use in event handler
/// let handler = {
///     let set_count = set_count.clone();
///     move |_: ()| set_count(42)
/// };
/// ```
pub fn use_shared_state<T>(initial: T) -> (SharedSignal<T>, SharedSetState<T>)
where
	T: Clone + Send + Sync + 'static,
{
	let signal = SharedSignal::new(initial);
	let setter: SharedSetState<T> = {
		let signal = signal.clone();
		Arc::new(move |value: T| signal.set(value))
	};
	(signal, setter)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_use_state_basic() {
		let (count, set_count) = use_state(0);
		assert_eq!(count.get(), 0);

		set_count(5);
		assert_eq!(count.get(), 5);

		set_count(10);
		assert_eq!(count.get(), 10);
	}

	#[test]
	fn test_use_state_with_string() {
		let (name, set_name) = use_state("Alice".to_string());
		assert_eq!(name.get(), "Alice");

		set_name("Bob".to_string());
		assert_eq!(name.get(), "Bob");
	}

	#[test]
	fn test_use_state_setter_cloneable() {
		let (count, set_count) = use_state(0);
		let set_count2 = set_count.clone();

		set_count(1);
		assert_eq!(count.get(), 1);

		set_count2(2);
		assert_eq!(count.get(), 2);
	}

	#[test]
	fn test_use_state_setter_set_method() {
		let (count, set_count) = use_state(0);

		set_count.set(7);

		assert_eq!(count.get(), 7);
	}

	#[test]
	fn test_use_state_setter_functional_update() {
		let (count, set_count) = use_state(0);

		set_count.update(|current| current + 1);
		set_count.update(|current| current * 2);

		assert_eq!(count.get(), 2);
	}

	#[test]
	fn test_use_state_setter_functional_update_uses_latest_value() {
		let (name, set_name) = use_state("Alice".to_string());
		let set_name2 = set_name.clone();

		set_name("Bob".to_string());
		set_name2.update(|current| format!("{current} Smith"));

		assert_eq!(name.get(), "Bob Smith");
	}

	#[test]
	fn test_use_state_setter_functional_update_allows_reading_state() {
		let (count, set_count) = use_state(1);
		let count_for_update = count.clone();

		set_count.update(|current| current + count_for_update.get());

		assert_eq!(count.get(), 2);
	}

	#[test]
	fn test_use_reducer_basic() {
		#[derive(Clone, Debug, PartialEq)]
		struct State {
			count: i32,
		}

		enum Action {
			Increment,
			Decrement,
		}

		fn reducer(state: &State, action: Action) -> State {
			match action {
				Action::Increment => State {
					count: state.count + 1,
				},
				Action::Decrement => State {
					count: state.count - 1,
				},
			}
		}

		let (state, dispatch) = use_reducer(reducer, State { count: 0 });
		assert_eq!(state.get().count, 0);

		dispatch(Action::Increment);
		assert_eq!(state.get().count, 1);

		dispatch(Action::Increment);
		assert_eq!(state.get().count, 2);

		dispatch(Action::Decrement);
		assert_eq!(state.get().count, 1);
	}

	#[test]
	fn test_use_reducer_complex_state() {
		#[derive(Clone, Debug, PartialEq)]
		struct TodoState {
			items: Vec<String>,
			filter: String,
		}

		enum TodoAction {
			Add(String),
			Remove(usize),
			SetFilter(String),
		}

		fn reducer(state: &TodoState, action: TodoAction) -> TodoState {
			match action {
				TodoAction::Add(item) => {
					let mut items = state.items.clone();
					items.push(item);
					TodoState {
						items,
						filter: state.filter.clone(),
					}
				}
				TodoAction::Remove(index) => {
					let mut items = state.items.clone();
					if index < items.len() {
						items.remove(index);
					}
					TodoState {
						items,
						filter: state.filter.clone(),
					}
				}
				TodoAction::SetFilter(filter) => TodoState {
					items: state.items.clone(),
					filter,
				},
			}
		}

		let (state, dispatch) = use_reducer(
			reducer,
			TodoState {
				items: vec![],
				filter: "all".to_string(),
			},
		);

		dispatch(TodoAction::Add("Task 1".to_string()));
		dispatch(TodoAction::Add("Task 2".to_string()));
		assert_eq!(state.get().items.len(), 2);

		dispatch(TodoAction::Remove(0));
		assert_eq!(state.get().items.len(), 1);
		assert_eq!(state.get().items[0], "Task 2");

		dispatch(TodoAction::SetFilter("completed".to_string()));
		assert_eq!(state.get().filter, "completed");
	}

	// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
	// SharedSignal and use_shared_state tests
	// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

	#[test]
	fn test_use_shared_state_basic() {
		let (count, set_count) = use_shared_state(0);
		assert_eq!(count.get(), 0);

		set_count(5);
		assert_eq!(count.get(), 5);

		set_count(10);
		assert_eq!(count.get(), 10);
	}

	#[test]
	fn test_use_shared_state_with_string() {
		let (name, set_name) = use_shared_state("Alice".to_string());
		assert_eq!(name.get(), "Alice");

		set_name("Bob".to_string());
		assert_eq!(name.get(), "Bob");
	}

	#[test]
	fn test_use_shared_state_setter_cloneable() {
		let (count, set_count) = use_shared_state(0);
		let set_count2 = set_count.clone();

		set_count(1);
		assert_eq!(count.get(), 1);

		set_count2(2);
		assert_eq!(count.get(), 2);
	}

	#[test]
	fn test_shared_signal_send_sync() {
		// Compile-time verification that SharedSignal and SharedSetState are Send + Sync
		fn assert_send<T: Send>() {}
		fn assert_sync<T: Sync>() {}

		assert_send::<SharedSignal<i32>>();
		assert_sync::<SharedSignal<i32>>();
		assert_send::<SharedSetState<i32>>();
		assert_sync::<SharedSetState<i32>>();
	}

	#[test]
	fn test_shared_signal_update() {
		let (count, _) = use_shared_state(0);

		count.update(|n| *n += 1);
		assert_eq!(count.get(), 1);

		count.update(|n| *n *= 2);
		assert_eq!(count.get(), 2);
	}

	#[test]
	fn test_shared_signal_get_untracked() {
		let (count, set_count) = use_shared_state(42);

		// get_untracked should not create dependencies but still return the value
		let value = count.get_untracked();
		assert_eq!(value, 42);

		set_count(100);
		assert_eq!(count.get_untracked(), 100);
	}
}
