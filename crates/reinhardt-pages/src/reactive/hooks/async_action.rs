//! Async action hook: `use_action`
//!
//! Provides an async mutation hook with pending/success/error state tracking.
//! This is designed for handling async operations like API calls, form submissions,
//! and other side effects that return a `Result`.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::marker::PhantomData;
use std::rc::Rc;

use super::action::OptimisticState;
use crate::callback::Callback;
use crate::reactive::Signal;
use reinhardt_core::reactive::deps::Trackable;

type ErrorCallback<E> = Rc<dyn Fn(&E)>;
type SuccessCallback<T> = Rc<dyn Fn(&T)>;
type SharedErrorCallback<E> = Rc<RefCell<ErrorCallback<E>>>;
type SharedSuccessCallback<T> = Rc<RefCell<SuccessCallback<T>>>;

/// Represents the current phase of an async action.
///
/// An action progresses through phases: `Idle` -> `Pending` -> `Success`/`Error`.
///
/// # Type Parameters
///
/// * `T` - The success value type
/// * `E` - The error value type
///
/// # Example
///
/// ```no_run
/// use reinhardt_pages::reactive::hooks::ActionPhase;
///
/// let phase: ActionPhase<String, String> = ActionPhase::Idle;
/// assert!(phase.is_idle());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum ActionPhase<T, E> {
	/// No action has been dispatched yet, or the action has been reset.
	Idle,
	/// An action is currently in progress.
	Pending,
	/// The action completed successfully with a value.
	Success(T),
	/// The action failed with an error.
	Error(E),
}

impl<T, E> ActionPhase<T, E> {
	/// Returns `true` if the phase is `Idle`.
	pub fn is_idle(&self) -> bool {
		matches!(self, ActionPhase::Idle)
	}

	/// Returns `true` if the phase is `Pending`.
	pub fn is_pending(&self) -> bool {
		matches!(self, ActionPhase::Pending)
	}

	/// Returns `true` if the phase is `Success`.
	pub fn is_success(&self) -> bool {
		matches!(self, ActionPhase::Success(_))
	}

	/// Returns `true` if the phase is `Error`.
	pub fn is_error(&self) -> bool {
		matches!(self, ActionPhase::Error(_))
	}

	/// Returns the success value if available.
	pub fn result(&self) -> Option<&T> {
		match self {
			ActionPhase::Success(val) => Some(val),
			_ => None,
		}
	}

	/// Returns the error value if available.
	pub fn error(&self) -> Option<&E> {
		match self {
			ActionPhase::Error(err) => Some(err),
			_ => None,
		}
	}
}

/// Handle returned by [`use_action`] for dispatching async mutations.
///
/// `Action` wraps the lifecycle of an async operation, tracking its phase
/// through `Idle` -> `Pending` -> `Success`/`Error`. The payload type `P`
/// is captured in the dispatch closure and does not appear in the struct type,
/// keeping the API ergonomic.
///
/// # Type Parameters
///
/// * `T` - The success value type (must be `Clone + 'static`)
/// * `E` - The error value type (must be `Clone + 'static`)
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::use_action;
///
/// let action = use_action(|user_id: u32| async move {
///     api_delete_user(user_id).await
/// });
///
/// // Dispatch the action
/// action.dispatch(42);
///
/// // Check the phase
/// if action.is_pending() {
///     // show spinner
/// }
/// ```
pub struct Action<T: Clone + 'static, E: Clone + 'static> {
	state: Signal<ActionPhase<T, E>>,
	dispatch_fn: Rc<dyn Fn()>,
	/// Stores the payload setter so dispatch can pass payload before triggering.
	payload_setter: Rc<dyn Fn(Box<dyn std::any::Any>)>,
	on_error: SharedErrorCallback<E>,
	on_success: SharedSuccessCallback<T>,
	reset_on_success: Rc<Cell<bool>>,
}

impl<T: Clone + 'static, E: Clone + 'static> Action<T, E> {
	/// Returns the current phase of the action, tracking the dependency.
	pub fn phase(&self) -> ActionPhase<T, E> {
		self.state.get()
	}

	/// Returns `true` if the action is idle.
	pub fn is_idle(&self) -> bool {
		self.phase().is_idle()
	}

	/// Returns `true` if the action is pending.
	pub fn is_pending(&self) -> bool {
		self.phase().is_pending()
	}

	/// Returns `true` if the action completed successfully.
	pub fn is_success(&self) -> bool {
		self.phase().is_success()
	}

	/// Returns `true` if the action failed.
	pub fn is_error(&self) -> bool {
		self.phase().is_error()
	}

	/// Returns the success value if available.
	pub fn result(&self) -> Option<T> {
		match self.state.get() {
			ActionPhase::Success(val) => Some(val),
			_ => None,
		}
	}

	/// Returns the latest successful result, if available.
	///
	/// This is an alias for [`Action::result`] with naming that reads naturally
	/// at UI call sites that render the last mutation outcome.
	pub fn last_result(&self) -> Option<T> {
		self.result()
	}

	/// Returns the error value if available.
	pub fn error(&self) -> Option<E> {
		match self.state.get() {
			ActionPhase::Error(err) => Some(err),
			_ => None,
		}
	}

	/// Returns the latest error, if available.
	///
	/// This is an alias for [`Action::error`] with naming that reads naturally
	/// at UI call sites that render the last mutation outcome.
	pub fn last_error(&self) -> Option<E> {
		self.error()
	}

	/// Renders the current successful result with the provided closure.
	pub fn render_result<R>(&self, render: impl FnOnce(&T) -> R) -> Option<R> {
		match self.state.get() {
			ActionPhase::Success(val) => Some(render(&val)),
			_ => None,
		}
	}

	/// Renders the current error with the provided closure.
	pub fn render_error<R>(&self, render: impl FnOnce(&E) -> R) -> Option<R> {
		match self.state.get() {
			ActionPhase::Error(err) => Some(render(&err)),
			_ => None,
		}
	}

	/// Resets the action back to `Idle` phase.
	pub fn reset(&self) {
		self.state.set(ActionPhase::Idle);
	}

	/// Returns an event callback that dispatches this action with `payload`.
	///
	/// Use [`Action::dispatching_with`] when the payload should be read at click
	/// time, or when the payload type is not cheaply cloneable.
	#[cfg(wasm)]
	pub fn dispatching<Event: 'static, P: Clone + 'static>(
		&self,
		payload: P,
	) -> Callback<Event, ()> {
		let action = self.clone();
		Callback::new(move |_| {
			action.dispatch(payload.clone());
		})
	}

	/// Returns an event callback that dispatches this action with `payload`.
	#[cfg(native)]
	pub fn dispatching<Event: 'static, P: Clone + 'static>(
		&self,
		payload: P,
	) -> Callback<Event, ()> {
		let action = self.clone();
		Callback::new(move |_| {
			action.dispatch(payload.clone());
		})
	}

	/// Returns an event callback that computes its payload at dispatch time.
	#[cfg(wasm)]
	pub fn dispatching_with<Event: 'static, P: 'static, F>(&self, payload: F) -> Callback<Event, ()>
	where
		F: Fn() -> P + 'static,
	{
		let action = self.clone();
		Callback::new(move |_| {
			action.dispatch(payload());
		})
	}

	/// Returns an event callback that computes its payload at dispatch time.
	#[cfg(native)]
	pub fn dispatching_with<Event: 'static, P: 'static, F>(&self, payload: F) -> Callback<Event, ()>
	where
		F: Fn() -> P + 'static,
	{
		let action = self.clone();
		Callback::new(move |_| {
			action.dispatch(payload());
		})
	}

	fn append_success_callback(&self, callback: SuccessCallback<T>) {
		let previous = self.on_success.borrow().clone();
		*self.on_success.borrow_mut() = Rc::new(move |value: &T| {
			previous(value);
			callback(value);
		});
	}

	fn append_error_callback(&self, callback: ErrorCallback<E>) {
		let previous = self.on_error.borrow().clone();
		*self.on_error.borrow_mut() = Rc::new(move |error: &E| {
			previous(error);
			callback(error);
		});
	}

	fn enable_reset_on_success(&self) {
		self.reset_on_success.set(true);
	}

	/// Connects this action to an optimistic state.
	///
	/// Successful completions confirm the optimistic value with the action
	/// result; failures revert it to the last confirmed value.
	pub fn with_optimistic(self, optimistic: OptimisticState<T>) -> Self {
		let optimistic_for_error = optimistic.clone();
		self.append_error_callback(Rc::new(move |_| {
			optimistic_for_error.revert();
		}));

		self.append_success_callback(Rc::new(move |value: &T| {
			optimistic.confirm(value.clone());
		}));

		self
	}

	/// Registers a callback to run after a successful WASM action.
	///
	/// Native actions do not poll the future, so callbacks only run on WASM.
	pub fn on_success<Callback>(self, callback: Callback) -> Self
	where
		Callback: Fn(&T) + 'static,
	{
		self.append_success_callback(Rc::new(callback));
		self
	}

	/// Registers a callback to run after a failed WASM action.
	///
	/// Native actions do not poll the future, so callbacks only run on WASM.
	pub fn on_error<Callback>(self, callback: Callback) -> Self
	where
		Callback: Fn(&E) + 'static,
	{
		self.append_error_callback(Rc::new(callback));
		self
	}

	#[cfg(test)]
	pub(crate) fn force_error_for_test(&self, err: E) {
		let on_error = self.on_error.borrow().clone();
		crate::reactive::batch(|| {
			self.state.set(ActionPhase::Error(err.clone()));
			on_error(&err);
		});
	}

	#[cfg(test)]
	pub(crate) fn force_success_for_test(&self, value: T) {
		let on_success = self.on_success.borrow().clone();
		crate::reactive::batch(|| {
			self.state.set(ActionPhase::Success(value.clone()));
			on_success(&value);
			if self.reset_on_success.get() {
				self.reset();
			}
		});
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Clone for Action<T, E> {
	fn clone(&self) -> Self {
		Self {
			state: self.state.clone(),
			dispatch_fn: Rc::clone(&self.dispatch_fn),
			payload_setter: Rc::clone(&self.payload_setter),
			on_error: Rc::clone(&self.on_error),
			on_success: Rc::clone(&self.on_success),
			reset_on_success: Rc::clone(&self.reset_on_success),
		}
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Trackable for Action<T, E> {
	fn node_id(&self) -> reinhardt_core::reactive::runtime::NodeId {
		self.state.id()
	}
}

/// Builder returned by [`use_action_state`].
///
/// The builder configures lifecycle callbacks around the same [`Action`]
/// handle returned by [`use_action`]. Call [`ActionStateBuilder::build`] after
/// attaching callbacks.
pub struct ActionStateBuilder<P, T, E, F, Fut> {
	action_fn: F,
	on_success: Vec<SuccessCallback<T>>,
	on_error: Vec<ErrorCallback<E>>,
	reset_on_success: bool,
	_payload: PhantomData<fn(P) -> Fut>,
}

impl<P, T, E, F, Fut> ActionStateBuilder<P, T, E, F, Fut>
where
	P: 'static,
	T: Clone + 'static,
	E: Clone + 'static,
	F: Fn(P) -> Fut + 'static,
	Fut: Future<Output = Result<T, E>> + 'static,
{
	/// Runs `callback` after the action completes successfully.
	pub fn on_success<Handler>(mut self, callback: Handler) -> Self
	where
		Handler: Fn(&T) + 'static,
	{
		self.on_success.push(Rc::new(callback));
		self
	}

	/// Runs `callback` after the action completes with an error.
	pub fn on_error<Handler>(mut self, callback: Handler) -> Self
	where
		Handler: Fn(&E) + 'static,
	{
		self.on_error.push(Rc::new(callback));
		self
	}

	/// Resets the action to `Idle` after success callbacks run.
	pub fn reset_on_success(mut self) -> Self {
		self.reset_on_success = true;
		self
	}

	/// Builds the configured action.
	pub fn build(self) -> Action<T, E> {
		let action = use_action(self.action_fn);

		for callback in self.on_success {
			action.append_success_callback(callback);
		}

		for callback in self.on_error {
			action.append_error_callback(callback);
		}

		if self.reset_on_success {
			action.enable_reset_on_success();
		}

		action
	}
}

/// Creates a builder for an async action with lifecycle callbacks.
///
/// This is a higher-level wrapper around [`use_action`] for UI mutations that
/// want the dispatch handle, pending/result/error state, and success/error
/// handling configured as one API surface.
pub fn use_action_state<P, T, E, F, Fut>(action_fn: F) -> ActionStateBuilder<P, T, E, F, Fut>
where
	P: 'static,
	T: Clone + 'static,
	E: Clone + 'static,
	F: Fn(P) -> Fut + 'static,
	Fut: Future<Output = Result<T, E>> + 'static,
{
	ActionStateBuilder {
		action_fn,
		on_success: Vec::new(),
		on_error: Vec::new(),
		reset_on_success: false,
		_payload: PhantomData,
	}
}

/// Creates an async action hook for performing mutations with state tracking.
///
/// `use_action` accepts an async function that takes a payload and returns a `Result`.
/// It returns an [`Action`] handle that tracks the lifecycle phases:
/// `Idle` -> `Pending` -> `Success(T)` / `Error(E)`.
///
/// # Type Parameters
///
/// * `P` - The payload type passed to dispatch
/// * `T` - The success value type
/// * `E` - The error value type
/// * `F` - The action function type
/// * `Fut` - The future type returned by the action function
///
/// # Arguments
///
/// * `action_fn` - An async function `Fn(P) -> Future<Output = Result<T, E>>`
///
/// # Returns
///
/// An [`Action<T, E>`] handle for dispatching and observing the action
///
/// # Dual-target behavior
///
/// - **WASM**: Uses `spawn_task` to run the future asynchronously. The phase
///   transitions `Idle -> Pending -> Success/Error` over time.
/// - **Non-WASM**: The future is not awaited (dropped). The phase transitions
///   `Idle -> Pending -> Idle` synchronously. This is intentional for SSR where
///   async mutations are not meaningful.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::use_action;
///
/// async fn submit_form(data: FormData) -> Result<String, String> {
///     // Perform API call...
///     Ok("Success".to_string())
/// }
///
/// let action = use_action(submit_form);
///
/// // Dispatch with payload
/// action.dispatch(form_data);
///
/// // Observe phase
/// match action.phase() {
///     ActionPhase::Idle => { /* show form */ }
///     ActionPhase::Pending => { /* show spinner */ }
///     ActionPhase::Success(msg) => { /* show success */ }
///     ActionPhase::Error(err) => { /* show error */ }
/// }
/// ```
///
/// # Reactivity semantics
///
/// The action closure runs outside any active reactive Observer. Reading
/// `Signal::get()`, `Memo::get()`, or `Resource::get()` inside returns the
/// latest value WITHOUT subscribing for future changes (Option A, Refs
/// #4195). The async action body further crosses an await boundary; no
/// Observer would survive that regardless of the surrounding context.
pub fn use_action<P, T, E, F, Fut>(action_fn: F) -> Action<T, E>
where
	P: 'static,
	T: Clone + 'static,
	E: Clone + 'static,
	F: Fn(P) -> Fut + 'static,
	Fut: Future<Output = Result<T, E>> + 'static,
{
	let state = Signal::new(ActionPhase::Idle);
	let on_error: SharedErrorCallback<E> = Rc::new(RefCell::new(Rc::new(|_: &E| {})));
	let on_success: SharedSuccessCallback<T> = Rc::new(RefCell::new(Rc::new(|_: &T| {})));
	let reset_on_success = Rc::new(Cell::new(false));

	// Store the payload in a shared cell so dispatch_fn can access it
	let payload_cell: Rc<RefCell<Option<Box<dyn std::any::Any>>>> = Rc::new(RefCell::new(None));

	let payload_setter: Rc<dyn Fn(Box<dyn std::any::Any>)> = {
		let payload_cell = Rc::clone(&payload_cell);
		Rc::new(move |payload: Box<dyn std::any::Any>| {
			*payload_cell.borrow_mut() = Some(payload);
		})
	};

	#[cfg(wasm)]
	let on_error_for_dispatch = Rc::clone(&on_error);
	#[cfg(wasm)]
	let on_success_for_dispatch = Rc::clone(&on_success);
	#[cfg(wasm)]
	let reset_on_success_for_dispatch = Rc::clone(&reset_on_success);
	#[cfg(native)]
	let on_error_for_dispatch = Rc::clone(&on_error);
	#[cfg(native)]
	let on_success_for_dispatch = Rc::clone(&on_success);
	#[cfg(native)]
	let reset_on_success_for_dispatch = Rc::clone(&reset_on_success);

	let dispatch_fn: Rc<dyn Fn()> = {
		let state = state.clone();
		let action_fn = Rc::new(action_fn);
		let payload_cell = Rc::clone(&payload_cell);

		Rc::new(move || {
			let payload = payload_cell
				.borrow_mut()
				.take()
				.and_then(|p| p.downcast::<P>().ok())
				.expect("dispatch called without payload");

			state.set(ActionPhase::Pending);

			#[cfg(wasm)]
			{
				use crate::platform::spawn_task;
				let on_error = Rc::clone(&on_error_for_dispatch);
				let on_success = Rc::clone(&on_success_for_dispatch);
				let reset_on_success = Rc::clone(&reset_on_success_for_dispatch);
				let state = state.clone();
				let fut = action_fn(*payload);
				spawn_task(async move {
					match fut.await {
						Ok(val) => {
							let on_success = on_success.borrow().clone();
							crate::reactive::batch(|| {
								state.set(ActionPhase::Success(val.clone()));
								on_success(&val);
								if reset_on_success.get() {
									state.set(ActionPhase::Idle);
								}
							});
						}
						Err(err) => {
							let on_error = on_error.borrow().clone();
							crate::reactive::batch(|| {
								state.set(ActionPhase::Error(err.clone()));
								on_error(&err);
							});
						}
					}
				});
			}

			#[cfg(native)]
			{
				let task_state = state.clone();
				let on_error = Rc::clone(&on_error_for_dispatch);
				let on_success = Rc::clone(&on_success_for_dispatch);
				let reset_on_success = Rc::clone(&reset_on_success_for_dispatch);
				let fut = action_fn(*payload);
				let spawned = crate::platform::try_spawn_task(async move {
					match fut.await {
						Ok(val) => {
							let on_success = on_success.borrow().clone();
							crate::reactive::batch(|| {
								task_state.set(ActionPhase::Success(val.clone()));
								on_success(&val);
								if reset_on_success.get() {
									task_state.set(ActionPhase::Idle);
								}
							});
						}
						Err(err) => {
							let on_error = on_error.borrow().clone();
							crate::reactive::batch(|| {
								task_state.set(ActionPhase::Error(err.clone()));
								on_error(&err);
							});
						}
					}
				});
				if !spawned {
					state.set(ActionPhase::Idle);
				}
			}
		})
	};

	Action {
		state,
		dispatch_fn,
		payload_setter,
		on_error,
		on_success,
		reset_on_success,
	}
}

impl<T: Clone + 'static, E: Clone + 'static> Action<T, E> {
	/// Dispatches the action with the given payload.
	///
	/// This sets the phase to `Pending` and begins executing the async action.
	/// On WASM, the future runs asynchronously. On non-WASM, the phase resets to `Idle`.
	pub fn dispatch<P: 'static>(&self, payload: P) {
		(self.payload_setter)(Box::new(payload));
		(self.dispatch_fn)();
	}
}

#[cfg(test)]
mod tests {
	use std::{cell::RefCell, rc::Rc};

	use rstest::rstest;

	use super::*;

	#[rstest]
	fn test_action_phase_methods() {
		// Arrange
		let idle: ActionPhase<String, String> = ActionPhase::Idle;
		let pending: ActionPhase<String, String> = ActionPhase::Pending;
		let success: ActionPhase<String, String> = ActionPhase::Success("ok".to_string());
		let error: ActionPhase<String, String> = ActionPhase::Error("fail".to_string());

		// Act & Assert
		assert!(idle.is_idle());
		assert!(!idle.is_pending());
		assert!(!idle.is_success());
		assert!(!idle.is_error());
		assert_eq!(idle.result(), None);
		assert_eq!(idle.error(), None);

		assert!(!pending.is_idle());
		assert!(pending.is_pending());
		assert!(!pending.is_success());
		assert!(!pending.is_error());

		assert!(!success.is_idle());
		assert!(!success.is_pending());
		assert!(success.is_success());
		assert!(!success.is_error());
		assert_eq!(success.result(), Some(&"ok".to_string()));
		assert_eq!(success.error(), None);

		assert!(!error.is_idle());
		assert!(!error.is_pending());
		assert!(!error.is_success());
		assert!(error.is_error());
		assert_eq!(error.result(), None);
		assert_eq!(error.error(), Some(&"fail".to_string()));
	}

	#[rstest]
	fn test_use_action_initial_idle() {
		// Arrange & Act
		let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });

		// Assert
		assert!(action.is_idle());
		assert_eq!(action.phase(), ActionPhase::Idle);
		assert_eq!(action.result(), None);
		assert_eq!(action.error(), None);
	}

	#[rstest]
	fn test_use_action_dispatch_native() {
		// Arrange
		let action = use_action(|x: i32| async move {
			if x > 0 {
				Ok::<i32, String>(x * 2)
			} else {
				Err("negative".to_string())
			}
		});

		// Act
		action.dispatch(5);

		// Assert
		// On non-WASM, dispatch sets Pending then immediately resets to Idle
		assert!(action.is_idle());
	}

	#[cfg(native)]
	#[rstest]
	fn dispatching_callbacks_accept_typed_event_arguments() {
		use crate::event::ClickEvent;

		let action = use_action(|value: i32| async move { Ok::<i32, String>(value * 2) });

		let dispatch: Callback<ClickEvent, ()> = action.dispatching(5);
		let dispatch_with: Callback<ClickEvent, ()> = action.dispatching_with(|| 6);

		drop((dispatch, dispatch_with));
	}

	#[rstest]
	fn test_action_clone() {
		// Arrange
		let action1 = use_action(|_: ()| async { Ok::<(), String>(()) });

		// Act
		let action2 = action1.clone();

		// Assert - both share the same Signal
		assert!(action1.is_idle());
		assert!(action2.is_idle());

		// Dispatching via one affects the other
		action1.dispatch(());
		assert_eq!(action1.phase(), action2.phase());
	}

	#[rstest]
	fn test_action_reset() {
		// Arrange
		let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });
		action.dispatch(());

		// Act
		action.reset();

		// Assert
		assert!(action.is_idle());
		assert_eq!(action.phase(), ActionPhase::Idle);
	}

	#[rstest]
	fn test_action_last_result_error_and_render_helpers() {
		// Arrange
		let action = use_action(|_: ()| async { Ok::<i32, String>(1) });

		// Act
		action.force_success_for_test(7);

		// Assert
		assert_eq!(action.last_result(), Some(7));
		assert_eq!(action.last_error(), None);
		assert_eq!(action.render_result(|value| value * 2), Some(14));
		assert_eq!(action.render_error(|error| error.len()), None);

		// Act
		action.force_error_for_test("failed".to_string());

		// Assert
		assert_eq!(action.last_result(), None);
		assert_eq!(action.last_error(), Some("failed".to_string()));
		assert_eq!(action.render_result(|value| value * 2), None);
		assert_eq!(action.render_error(|error| error.len()), Some(6));
	}

	#[rstest]
	fn test_use_action_state_builder_runs_lifecycle_callbacks() {
		// Arrange
		let success_values = Rc::new(RefCell::new(Vec::new()));
		let error_values = Rc::new(RefCell::new(Vec::new()));
		let action = use_action_state(|_: ()| async { Ok::<i32, String>(1) })
			.on_success({
				let success_values = Rc::clone(&success_values);
				move |value| success_values.borrow_mut().push(*value)
			})
			.on_error({
				let error_values = Rc::clone(&error_values);
				move |error| error_values.borrow_mut().push(error.clone())
			})
			.build();

		// Act
		action.force_success_for_test(11);
		action.force_error_for_test("network".to_string());

		// Assert
		assert_eq!(*success_values.borrow(), vec![11]);
		assert_eq!(*error_values.borrow(), vec!["network".to_string()]);
	}

	#[rstest]
	fn test_use_action_state_builder_resets_on_success() {
		// Arrange
		let action = use_action_state(|_: ()| async { Ok::<i32, String>(1) })
			.reset_on_success()
			.build();

		// Act
		action.force_success_for_test(11);

		// Assert
		assert_eq!(action.phase(), ActionPhase::Idle);
		assert_eq!(action.last_result(), None);
	}

	#[rstest]
	fn test_action_with_optimistic_reverts_on_error() {
		// Arrange
		let optimistic = super::super::action::use_optimistic(10);
		optimistic.update_optimistic(20);
		let action = use_action(|_: ()| async { Err::<i32, String>("fail".to_string()) })
			.with_optimistic(optimistic.clone());

		// Act
		action.force_error_for_test("fail".to_string());

		// Assert
		assert_eq!(optimistic.get(), 10);
		assert!(!optimistic.is_optimistic());
		assert_eq!(action.phase(), ActionPhase::Error("fail".to_string()));
	}

	#[rstest]
	fn test_action_with_optimistic_confirms_on_success() {
		// Arrange
		let optimistic = super::super::action::use_optimistic(10);
		optimistic.update_optimistic(20);
		let action =
			use_action(|_: ()| async { Ok::<i32, String>(25) }).with_optimistic(optimistic.clone());

		// Act
		action.force_success_for_test(25);

		// Assert
		assert_eq!(optimistic.get(), 25);
		assert!(!optimistic.is_optimistic());
		assert_eq!(action.phase(), ActionPhase::Success(25));
	}

	#[rstest]
	fn test_action_success_callbacks_are_additive() {
		// Arrange
		let callback_count = Rc::new(RefCell::new(0));
		let first_count = Rc::clone(&callback_count);
		let second_count = Rc::clone(&callback_count);
		let action = use_action(|_: ()| async { Ok::<i32, String>(25) })
			.on_success(move |value| {
				assert_eq!(*value, 25);
				*first_count.borrow_mut() += 1;
			})
			.on_success(move |value| {
				assert_eq!(*value, 25);
				*second_count.borrow_mut() += 1;
			});

		// Act
		action.force_success_for_test(25);

		// Assert
		assert_eq!(*callback_count.borrow(), 2);
	}

	#[rstest]
	fn test_action_error_callbacks_receive_error() {
		// Arrange
		let captured_error = Rc::new(RefCell::new(None));
		let captured_error_for_callback = Rc::clone(&captured_error);
		let action = use_action(|_: ()| async { Err::<i32, String>("fail".to_string()) }).on_error(
			move |error| {
				*captured_error_for_callback.borrow_mut() = Some(error.clone());
			},
		);

		// Act
		action.force_error_for_test("fail".to_string());

		// Assert
		assert_eq!(captured_error.borrow().as_deref(), Some("fail"));
	}
}
