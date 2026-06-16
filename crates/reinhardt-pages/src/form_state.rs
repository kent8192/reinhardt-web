//! Runtime behavior for `form!` generated forms.

use std::any::{Any, type_name};
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::reactive::{Effect, EffectTiming, Signal};

/// Default reset behavior when runtime dependencies change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetOnDeps {
	/// Keep dirty field values and update pristine fields from new defaults.
	KeepDirtyValues,
	/// Replace every value with the new defaults.
	ResetAll,
	/// Record the dependency change without changing values.
	ExplicitOnly,
}

/// Validation timing for runtime-managed forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevalidateOn {
	/// Do not revalidate automatically.
	Submit,
	/// Revalidate when dependencies change.
	DepsChange,
	/// Revalidate whenever values are written through the runtime handle.
	Change,
}

/// No dependency marker for `use_form(&form).build()`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoDeps;

/// A field-level error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
	message: String,
}

impl FieldError {
	/// Creates a field error from a displayable message.
	pub fn new(message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
		}
	}

	/// Returns the error message.
	pub fn message(&self) -> &str {
		&self.message
	}
}

/// Validation failure for a generated form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormValidationError<Field>
where
	Field: Copy + Eq + Hash,
{
	details: Box<FormValidationErrorDetails<Field>>,
	form_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FormValidationErrorDetails<Field>
where
	Field: Copy + Eq + Hash,
{
	field_errors: HashMap<Field, FieldError>,
}

impl<Field> FormValidationErrorDetails<Field>
where
	Field: Copy + Eq + Hash,
{
	fn new() -> Self {
		Self {
			field_errors: HashMap::new(),
		}
	}
}

impl<Field> FormValidationError<Field>
where
	Field: Copy + Eq + Hash,
{
	/// Creates an empty validation error.
	pub fn new() -> Self {
		Self {
			details: Box::new(FormValidationErrorDetails::new()),
			form_error: None,
		}
	}

	/// Creates a field validation error.
	pub fn field(field: Field, message: impl Into<String>) -> Self {
		let mut details = FormValidationErrorDetails::new();
		details.field_errors.insert(field, FieldError::new(message));
		Self {
			details: Box::new(details),
			form_error: None,
		}
	}

	/// Creates a form-level validation error.
	pub fn form(message: impl Into<String>) -> Self {
		Self {
			details: Box::new(FormValidationErrorDetails::new()),
			form_error: Some(message.into()),
		}
	}

	/// Returns field errors.
	pub fn field_errors(&self) -> &HashMap<Field, FieldError> {
		&self.details.field_errors
	}

	/// Returns the form-level error, if any.
	pub fn form_error(&self) -> Option<&str> {
		self.form_error.as_deref()
	}

	/// Adds or replaces one field validation error.
	pub fn add_field_error(&mut self, field: Field, message: impl Into<String>) {
		self.details
			.field_errors
			.insert(field, FieldError::new(message));
	}

	/// Sets the form-level validation error.
	pub fn set_form_error(&mut self, message: impl Into<String>) {
		self.form_error = Some(message.into());
	}

	/// Returns whether this error contains no field or form error.
	pub fn is_empty(&self) -> bool {
		self.details.field_errors.is_empty() && self.form_error.is_none()
	}
}

impl<Field> Default for FormValidationError<Field>
where
	Field: Copy + Eq + Hash,
{
	fn default() -> Self {
		Self::new()
	}
}

/// Aggregate state for a runtime form.
#[derive(Clone)]
pub struct FormState<Field>
where
	Field: Copy + Eq + Hash + 'static,
{
	/// Whether current values differ from defaults.
	pub is_dirty: Signal<bool>,
	/// Whether any value was changed through the runtime.
	pub is_touched: Signal<bool>,
	/// Field-level errors.
	pub field_errors: Signal<HashMap<Field, FieldError>>,
	/// Form-level validation error.
	pub form_error: Signal<Option<String>>,
	/// Last submit-level error.
	pub submit_error: Signal<Option<String>>,
	/// First visible validation or submit error.
	pub error: Signal<Option<String>>,
	/// Whether submit is pending.
	pub is_submitting: Signal<bool>,
	/// Whether the last submit succeeded.
	pub is_submit_successful: Signal<bool>,
}

/// Snapshot for one generated field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldState {
	/// Whether this field differs from its default value.
	pub is_dirty: bool,
	/// Whether this field was changed through the runtime.
	pub is_touched: bool,
	/// Current field-level error.
	pub error: Option<FieldError>,
}

/// Form runtime event sent to subscribers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormEvent<Form>
where
	Form: FormRuntimeSource,
{
	/// A field value changed through the runtime handle.
	ValueChanged {
		/// Field whose value changed.
		field: Form::Field,
	},
	/// Validation was run.
	Validated,
	/// Submit started.
	SubmitStarted,
	/// Submit completed successfully.
	Submitted,
	/// Submit failed before dispatch or during validation.
	SubmitFailed,
	/// Runtime dependencies changed.
	DepsChanged,
}

/// Result of `UseFormReturn::handle_submit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseFormSubmitOutcome {
	/// Submit was accepted.
	Submitted,
	/// Submit was rejected because another submit is pending.
	AlreadyPending,
	/// Submit was rejected by validation.
	ValidationFailed,
}

/// Error returned by focus operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusError {
	/// Focus is not available for the active target.
	Unsupported,
	/// The generated form does not expose a focus target for the field.
	MissingTarget,
}

/// Trait implemented by `form!` generated forms.
pub trait FormRuntimeSource: Clone + 'static {
	/// Generated value struct for this form.
	type Values: Clone + 'static;
	/// Generated field token enum for this form.
	type Field: Copy + Eq + Hash + Debug + 'static;

	/// Returns current defaults captured by the form definition.
	fn runtime_initial_values(&self) -> Self::Values;

	/// Reads current values from generated field controls.
	fn runtime_current_values(&self) -> Self::Values;

	/// Applies all values to generated field controls.
	fn runtime_apply_values(&self, values: &Self::Values);

	/// Applies one typed value to one generated field.
	fn runtime_set_field_value<T>(&self, field: Self::Field, value: T)
	where
		T: Any + 'static;

	/// Applies the matching value from `values` to one generated field.
	fn runtime_apply_field_value(&self, field: Self::Field, values: &Self::Values);

	/// Returns whether one field differs between `current` and `defaults`.
	fn runtime_field_is_dirty(
		&self,
		field: Self::Field,
		current: &Self::Values,
		defaults: &Self::Values,
	) -> bool;

	/// Returns the generated signal for a field when `T` matches the field type.
	fn runtime_watch_field<T>(&self, field: Self::Field) -> Option<Signal<T>>
	where
		T: Clone + 'static;

	/// Runs generated validation.
	fn runtime_validate(&self) -> Result<(), FormValidationError<Self::Field>> {
		Ok(())
	}

	/// Attempts to focus a generated field.
	fn runtime_set_focus(&self, _field: Self::Field) -> Result<(), FocusError> {
		Err(FocusError::Unsupported)
	}

	/// Returns generated field tokens in source order.
	fn runtime_fields(&self) -> &'static [Self::Field];
}

/// Builder returned by `use_form(&form)`.
pub struct UseFormBuilder<Form, Deps = NoDeps>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	form: Form,
	deps: Deps,
	reset_on_deps: ResetOnDeps,
	keep_errors: bool,
	revalidate_on: RevalidateOn,
	on_submit_start: Option<SubmitCallback<Form, Deps>>,
	on_submit_success: Option<SubmitCallback<Form, Deps>>,
	on_submit_error: Option<SubmitCallback<Form, Deps>>,
}

type SubmitCallback<Form, Deps> = Rc<dyn Fn(&UseFormReturn<Form, Deps>)>;
type Subscriber<Form> = Rc<dyn Fn(FormEvent<Form>)>;
type SubscriberSlots<Form> = Rc<RefCell<Vec<Option<Subscriber<Form>>>>>;

fn form_values_are_dirty<Form>(form: &Form, current: &Form::Values, defaults: &Form::Values) -> bool
where
	Form: FormRuntimeSource,
{
	form.runtime_fields()
		.iter()
		.copied()
		.any(|field| form.runtime_field_is_dirty(field, current, defaults))
}

#[allow(
	clippy::too_many_arguments,
	reason = "The sync effect needs the independent form state handles it updates atomically."
)]
fn build_signal_sync_effect<Form>(
	form: Form,
	default_values: Rc<RefCell<Form::Values>>,
	state: FormState<Form::Field>,
	touched_fields: Rc<RefCell<HashMap<Form::Field, bool>>>,
	values_signal: Signal<Form::Values>,
	subscribers: SubscriberSlots<Form>,
	observed_values: Rc<RefCell<Form::Values>>,
	signal_sync_suppressed: Rc<Cell<bool>>,
	revalidate_on: RevalidateOn,
) -> Rc<Effect>
where
	Form: FormRuntimeSource,
{
	Rc::new(Effect::new_with_timing(
		move || {
			let current = form.runtime_current_values();
			let previous = observed_values.borrow().clone();
			let changed_fields: Vec<Form::Field> = form
				.runtime_fields()
				.iter()
				.copied()
				.filter(|field| form.runtime_field_is_dirty(*field, &current, &previous))
				.collect();
			*observed_values.borrow_mut() = current.clone();

			if signal_sync_suppressed.get() || changed_fields.is_empty() {
				return;
			}

			for field in &changed_fields {
				touched_fields.borrow_mut().insert(*field, true);
			}
			state.is_touched.set(true);
			state.is_dirty.set(form_values_are_dirty(
				&form,
				&current,
				&default_values.borrow(),
			));
			values_signal.set(current);

			if revalidate_on == RevalidateOn::Change {
				let result = form.runtime_validate();
				apply_validation_result_to_state(&state, &result);
				notify_subscribers(&subscribers, FormEvent::Validated);
			}
			for field in changed_fields {
				notify_subscribers(&subscribers, FormEvent::ValueChanged { field });
			}
		},
		EffectTiming::Layout,
	))
}

fn adapt_no_deps_callback<Form, Deps>(
	callback: SubmitCallback<Form, NoDeps>,
) -> SubmitCallback<Form, Deps>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	Rc::new(move |handle| {
		let no_deps_handle = UseFormReturn {
			form: handle.form.clone(),
			default_values: Rc::clone(&handle.default_values),
			deps: Rc::new(RefCell::new(NoDeps)),
			reset_on_deps: handle.reset_on_deps,
			keep_errors: handle.keep_errors,
			revalidate_on: handle.revalidate_on,
			state: handle.state.clone(),
			touched_fields: Rc::clone(&handle.touched_fields),
			values_signal: handle.values_signal.clone(),
			subscribers: Rc::clone(&handle.subscribers),
			observed_values: Rc::clone(&handle.observed_values),
			signal_sync_suppressed: Rc::clone(&handle.signal_sync_suppressed),
			_signal_sync_effect: Rc::clone(&handle._signal_sync_effect),
			on_submit_start: None,
			on_submit_success: None,
			on_submit_error: None,
		};
		callback(&no_deps_handle);
	})
}

fn apply_validation_result_to_state<Field>(
	state: &FormState<Field>,
	result: &Result<(), FormValidationError<Field>>,
) where
	Field: Copy + Eq + Hash + 'static,
{
	match result {
		Ok(()) => {
			state.field_errors.set(HashMap::new());
			state.form_error.set(None);
			sync_first_error_in_state(state);
		}
		Err(error) => {
			state.field_errors.set(error.field_errors().clone());
			state.form_error.set(error.form_error().map(str::to_string));
			sync_first_error_in_state(state);
		}
	}
}

fn sync_first_error_in_state<Field>(state: &FormState<Field>)
where
	Field: Copy + Eq + Hash + 'static,
{
	let first_field_error = state
		.field_errors
		.get()
		.values()
		.next()
		.map(|error| error.message().to_string());
	state.error.set(
		first_field_error
			.or_else(|| state.form_error.get())
			.or_else(|| state.submit_error.get()),
	);
}

fn notify_subscribers<Form>(subscribers: &SubscriberSlots<Form>, event: FormEvent<Form>)
where
	Form: FormRuntimeSource,
{
	for subscriber in subscribers.borrow().iter().flatten() {
		subscriber(event.clone());
	}
}

impl<Form> UseFormBuilder<Form, NoDeps>
where
	Form: FormRuntimeSource,
{
	/// Sets runtime dependencies.
	pub fn deps<Deps>(self, deps: Deps) -> UseFormBuilder<Form, Deps>
	where
		Deps: Clone + PartialEq + 'static,
	{
		UseFormBuilder {
			form: self.form,
			deps,
			reset_on_deps: self.reset_on_deps,
			keep_errors: self.keep_errors,
			revalidate_on: self.revalidate_on,
			on_submit_start: self.on_submit_start.map(adapt_no_deps_callback),
			on_submit_success: self.on_submit_success.map(adapt_no_deps_callback),
			on_submit_error: self.on_submit_error.map(adapt_no_deps_callback),
		}
	}
}

impl<Form, Deps> UseFormBuilder<Form, Deps>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	/// Sets dependency reset behavior.
	pub fn reset_on_deps(mut self, policy: ResetOnDeps) -> Self {
		self.reset_on_deps = policy;
		self
	}

	/// Sets whether dependency changes keep existing errors.
	pub fn keep_errors(mut self, keep: bool) -> Self {
		self.keep_errors = keep;
		self
	}

	/// Sets revalidation timing.
	pub fn revalidate_on(mut self, timing: RevalidateOn) -> Self {
		self.revalidate_on = timing;
		self
	}

	/// Registers a submit-start callback.
	pub fn on_submit_start<Callback>(mut self, callback: Callback) -> Self
	where
		Callback: Fn(&UseFormReturn<Form, Deps>) + 'static,
	{
		self.on_submit_start = Some(Rc::new(callback));
		self
	}

	/// Registers a submit-success callback.
	pub fn on_submit_success<Callback>(mut self, callback: Callback) -> Self
	where
		Callback: Fn(&UseFormReturn<Form, Deps>) + 'static,
	{
		self.on_submit_success = Some(Rc::new(callback));
		self
	}

	/// Registers a submit-error callback.
	pub fn on_submit_error<Callback>(mut self, callback: Callback) -> Self
	where
		Callback: Fn(&UseFormReturn<Form, Deps>) + 'static,
	{
		self.on_submit_error = Some(Rc::new(callback));
		self
	}

	/// Builds the runtime form handle.
	pub fn build(self) -> UseFormReturn<Form, Deps> {
		let default_values = self.form.runtime_initial_values();
		let current_values = self.form.runtime_current_values();
		let is_dirty = form_values_are_dirty(&self.form, &current_values, &default_values);
		let form = self.form;
		let default_values = Rc::new(RefCell::new(default_values));
		let deps = Rc::new(RefCell::new(self.deps));
		let state = FormState {
			is_dirty: Signal::new(is_dirty),
			is_touched: Signal::new(false),
			field_errors: Signal::new(HashMap::new()),
			form_error: Signal::new(None),
			submit_error: Signal::new(None),
			error: Signal::new(None),
			is_submitting: Signal::new(false),
			is_submit_successful: Signal::new(false),
		};
		let touched_fields = Rc::new(RefCell::new(HashMap::new()));
		let values_signal = Signal::new(current_values.clone());
		let subscribers = Rc::new(RefCell::new(Vec::new()));
		let observed_values = Rc::new(RefCell::new(current_values));
		let signal_sync_suppressed = Rc::new(Cell::new(false));
		let signal_sync_effect = build_signal_sync_effect(
			form.clone(),
			Rc::clone(&default_values),
			state.clone(),
			Rc::clone(&touched_fields),
			values_signal.clone(),
			Rc::clone(&subscribers),
			Rc::clone(&observed_values),
			Rc::clone(&signal_sync_suppressed),
			self.revalidate_on,
		);
		UseFormReturn {
			form,
			default_values,
			deps,
			reset_on_deps: self.reset_on_deps,
			keep_errors: self.keep_errors,
			revalidate_on: self.revalidate_on,
			state,
			touched_fields,
			values_signal,
			subscribers,
			observed_values,
			signal_sync_suppressed,
			_signal_sync_effect: signal_sync_effect,
			on_submit_start: self.on_submit_start,
			on_submit_success: self.on_submit_success,
			on_submit_error: self.on_submit_error,
		}
	}
}

/// Dynamic behavior handle for a `form!` generated form.
pub struct UseFormReturn<Form, Deps = NoDeps>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	form: Form,
	default_values: Rc<RefCell<Form::Values>>,
	deps: Rc<RefCell<Deps>>,
	reset_on_deps: ResetOnDeps,
	keep_errors: bool,
	revalidate_on: RevalidateOn,
	state: FormState<Form::Field>,
	touched_fields: Rc<RefCell<HashMap<Form::Field, bool>>>,
	values_signal: Signal<Form::Values>,
	subscribers: SubscriberSlots<Form>,
	observed_values: Rc<RefCell<Form::Values>>,
	signal_sync_suppressed: Rc<Cell<bool>>,
	_signal_sync_effect: Rc<Effect>,
	on_submit_start: Option<SubmitCallback<Form, Deps>>,
	on_submit_success: Option<SubmitCallback<Form, Deps>>,
	on_submit_error: Option<SubmitCallback<Form, Deps>>,
}

impl<Form, Deps> Clone for UseFormReturn<Form, Deps>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	fn clone(&self) -> Self {
		Self {
			form: self.form.clone(),
			default_values: Rc::clone(&self.default_values),
			deps: Rc::clone(&self.deps),
			reset_on_deps: self.reset_on_deps,
			keep_errors: self.keep_errors,
			revalidate_on: self.revalidate_on,
			state: self.state.clone(),
			touched_fields: Rc::clone(&self.touched_fields),
			values_signal: self.values_signal.clone(),
			subscribers: Rc::clone(&self.subscribers),
			observed_values: Rc::clone(&self.observed_values),
			signal_sync_suppressed: Rc::clone(&self.signal_sync_suppressed),
			_signal_sync_effect: Rc::clone(&self._signal_sync_effect),
			on_submit_start: self.on_submit_start.clone(),
			on_submit_success: self.on_submit_success.clone(),
			on_submit_error: self.on_submit_error.clone(),
		}
	}
}

impl<Form, Deps> UseFormReturn<Form, Deps>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	/// Returns a signal containing the current value struct.
	pub fn watch(&self) -> Signal<Form::Values> {
		self.values_signal.clone()
	}

	/// Returns a typed field signal.
	pub fn watch_field<T>(&self, field: Form::Field) -> Signal<T>
	where
		T: Clone + 'static,
	{
		self.form.runtime_watch_field(field).unwrap_or_else(|| {
			panic!(
				"field {:?} is not compatible with requested Signal<{}>",
				field,
				type_name::<T>()
			)
		})
	}

	/// Returns current typed values.
	pub fn get_values(&self) -> Form::Values {
		self.form.runtime_current_values()
	}

	/// Returns current default values.
	pub fn default_values(&self) -> Form::Values {
		self.default_values.borrow().clone()
	}

	/// Returns state for one field.
	pub fn get_field_state(&self, field: Form::Field) -> FieldState {
		let current = self.get_values();
		let defaults = self.default_values.borrow();
		let errors = self.state.field_errors.get();
		FieldState {
			is_dirty: self.form.runtime_field_is_dirty(field, &current, &defaults),
			is_touched: self
				.touched_fields
				.borrow()
				.get(&field)
				.copied()
				.unwrap_or(false),
			error: errors.get(&field).cloned(),
		}
	}

	/// Applies one typed field value.
	pub fn set_value<T>(&self, field: Form::Field, value: T)
	where
		T: Any + 'static,
	{
		let _guard = self.suppress_signal_sync();
		self.form.runtime_set_field_value(field, value);
		self.touched_fields.borrow_mut().insert(field, true);
		self.state.is_touched.set(true);
		self.refresh_dirty();
		self.values_signal.set(self.get_values());
		self.sync_observed_values();
		if self.revalidate_on == RevalidateOn::Change {
			let _ = self.trigger();
		}
		self.notify(FormEvent::ValueChanged { field });
	}

	/// Applies all typed values.
	pub fn set_values(&self, values: Form::Values) {
		let _guard = self.suppress_signal_sync();
		self.form.runtime_apply_values(&values);
		self.state.is_touched.set(true);
		self.refresh_dirty();
		self.values_signal.set(values);
		self.sync_observed_values();
		if self.revalidate_on == RevalidateOn::Change {
			let _ = self.trigger();
		}
	}

	/// Sets one field error.
	pub fn set_error(&self, field: Form::Field, error: FieldError) {
		let mut errors = self.state.field_errors.get();
		errors.insert(field, error);
		self.state.field_errors.set(errors);
		self.sync_first_error();
	}

	/// Clears all validation and submit errors.
	pub fn clear_errors(&self) {
		self.state.field_errors.set(HashMap::new());
		self.state.form_error.set(None);
		self.state.submit_error.set(None);
		self.state.error.set(None);
	}

	/// Clears one field error.
	pub fn clear_field_error(&self, field: Form::Field) {
		let mut errors = self.state.field_errors.get();
		errors.remove(&field);
		self.state.field_errors.set(errors);
		self.sync_first_error();
	}

	/// Runs generated validation.
	pub fn trigger(&self) -> Result<(), FormValidationError<Form::Field>> {
		let result = self.form.runtime_validate();
		self.apply_validation_result(&result);
		self.notify(FormEvent::Validated);
		result
	}

	/// Runs validation and returns whether one field remains error-free.
	pub fn trigger_field(
		&self,
		field: Form::Field,
	) -> Result<(), FormValidationError<Form::Field>> {
		let result = self.trigger();
		if self.state.field_errors.get().contains_key(&field) {
			result
		} else {
			Ok(())
		}
	}

	/// Returns aggregate form state signals.
	pub fn form_state(&self) -> FormState<Form::Field> {
		self.state.clone()
	}

	/// Resets all values to current defaults.
	pub fn reset(&self) {
		let defaults = self.default_values.borrow().clone();
		let _guard = self.suppress_signal_sync();
		self.form.runtime_apply_values(&defaults);
		self.touched_fields.borrow_mut().clear();
		self.state.is_touched.set(false);
		self.state.is_dirty.set(false);
		self.state.is_submitting.set(false);
		self.state.is_submit_successful.set(false);
		self.clear_errors();
		self.values_signal.set(defaults);
		self.sync_observed_values();
	}

	/// Resets one field to its current default value.
	pub fn reset_field(&self, field: Form::Field) {
		let defaults = self.default_values.borrow();
		let _guard = self.suppress_signal_sync();
		self.form.runtime_apply_field_value(field, &defaults);
		self.touched_fields.borrow_mut().remove(&field);
		self.refresh_dirty();
		self.values_signal.set(self.get_values());
		self.sync_observed_values();
	}

	/// Makes the current values the defaults and clears dirty state.
	pub fn reset_default_values(&self) {
		*self.default_values.borrow_mut() = self.get_values();
		self.state.is_dirty.set(false);
	}

	/// Attempts to focus one field.
	pub fn set_focus(&self, field: Form::Field) -> Result<(), FocusError> {
		self.form.runtime_set_focus(field)
	}

	/// Subscribes to runtime form events.
	pub fn subscribe<Callback>(&self, callback: Callback) -> FormSubscription<Form>
	where
		Callback: Fn(FormEvent<Form>) + 'static,
	{
		let mut subscribers = self.subscribers.borrow_mut();
		let index = subscribers.len();
		subscribers.push(Some(Rc::new(callback)));
		FormSubscription {
			index,
			subscribers: Rc::downgrade(&self.subscribers),
		}
	}

	/// Runs validation and submit lifecycle callbacks.
	pub fn handle_submit(&self) -> UseFormSubmitOutcome {
		if self.state.is_submitting.get() {
			return UseFormSubmitOutcome::AlreadyPending;
		}

		self.state.is_submitting.set(true);
		self.state.is_submit_successful.set(false);
		self.state.submit_error.set(None);
		self.notify(FormEvent::SubmitStarted);
		if let Some(callback) = &self.on_submit_start {
			callback(self);
		}

		if self.trigger().is_err() {
			self.state.is_submitting.set(false);
			if let Some(callback) = &self.on_submit_error {
				callback(self);
			}
			self.notify(FormEvent::SubmitFailed);
			return UseFormSubmitOutcome::ValidationFailed;
		}

		self.state.is_submitting.set(false);
		self.state.is_submit_successful.set(true);
		if let Some(callback) = &self.on_submit_success {
			callback(self);
		}
		self.notify(FormEvent::Submitted);
		UseFormSubmitOutcome::Submitted
	}

	/// Reconciles values and defaults from a newly generated form instance.
	pub fn reconcile_from(&self, form: &Form, deps: Deps) {
		self.reconcile_defaults(form.runtime_initial_values(), deps);
	}

	/// Reconciles values and defaults from a new default value struct.
	pub fn reconcile_defaults(&self, new_defaults: Form::Values, deps: Deps) {
		if *self.deps.borrow() == deps {
			return;
		}
		*self.deps.borrow_mut() = deps;

		let old_defaults = self.default_values.borrow().clone();
		let current = self.get_values();

		match self.reset_on_deps {
			ResetOnDeps::KeepDirtyValues => {
				let _guard = self.suppress_signal_sync();
				for field in self.form.runtime_fields() {
					let field = *field;
					if !self
						.form
						.runtime_field_is_dirty(field, &current, &old_defaults)
					{
						self.form.runtime_apply_field_value(field, &new_defaults);
					}
				}
			}
			ResetOnDeps::ResetAll => {
				let _guard = self.suppress_signal_sync();
				self.form.runtime_apply_values(&new_defaults);
				self.touched_fields.borrow_mut().clear();
			}
			ResetOnDeps::ExplicitOnly => {}
		}

		*self.default_values.borrow_mut() = new_defaults;
		if !self.keep_errors {
			self.clear_errors();
		}
		self.refresh_dirty();
		self.values_signal.set(self.get_values());
		self.sync_observed_values();
		if self.revalidate_on == RevalidateOn::DepsChange {
			let _ = self.trigger();
		}
		self.notify(FormEvent::DepsChanged);
	}

	fn apply_validation_result(&self, result: &Result<(), FormValidationError<Form::Field>>) {
		match result {
			Ok(()) => {
				self.state.field_errors.set(HashMap::new());
				self.state.form_error.set(None);
				self.sync_first_error();
			}
			Err(error) => {
				self.state.field_errors.set(error.field_errors().clone());
				self.state
					.form_error
					.set(error.form_error().map(str::to_string));
				self.sync_first_error();
			}
		}
	}

	fn refresh_dirty(&self) {
		let current = self.get_values();
		self.state.is_dirty.set(form_values_are_dirty(
			&self.form,
			&current,
			&self.default_values.borrow(),
		));
		self.state.is_touched.set(true);
	}

	fn sync_first_error(&self) {
		let first_field_error = self
			.state
			.field_errors
			.get()
			.values()
			.next()
			.map(|error| error.message().to_string());
		self.state.error.set(
			first_field_error
				.or_else(|| self.state.form_error.get())
				.or_else(|| self.state.submit_error.get()),
		);
	}

	fn notify(&self, event: FormEvent<Form>) {
		for subscriber in self.subscribers.borrow().iter().flatten() {
			subscriber(event.clone());
		}
	}

	fn suppress_signal_sync(&self) -> SignalSyncGuard {
		self.signal_sync_suppressed.set(true);
		SignalSyncGuard {
			suppressed: Rc::clone(&self.signal_sync_suppressed),
		}
	}

	fn sync_observed_values(&self) {
		*self.observed_values.borrow_mut() = self.get_values();
	}
}

struct SignalSyncGuard {
	suppressed: Rc<Cell<bool>>,
}

impl Drop for SignalSyncGuard {
	fn drop(&mut self) {
		self.suppressed.set(false);
	}
}

/// RAII subscription guard returned by `UseFormReturn::subscribe`.
pub struct FormSubscription<Form>
where
	Form: FormRuntimeSource,
{
	index: usize,
	subscribers: Weak<RefCell<Vec<Option<Subscriber<Form>>>>>,
}

impl<Form> Drop for FormSubscription<Form>
where
	Form: FormRuntimeSource,
{
	fn drop(&mut self) {
		if let Some(subscribers) = self.subscribers.upgrade()
			&& let Some(slot) = subscribers.borrow_mut().get_mut(self.index)
		{
			*slot = None;
		}
	}
}

/// Starts a runtime builder for a generated form.
pub fn use_form<Form>(form: &Form) -> UseFormBuilder<Form, NoDeps>
where
	Form: FormRuntimeSource,
{
	UseFormBuilder {
		form: form.clone(),
		deps: NoDeps,
		reset_on_deps: ResetOnDeps::KeepDirtyValues,
		keep_errors: false,
		revalidate_on: RevalidateOn::Submit,
		on_submit_start: None,
		on_submit_success: None,
		on_submit_error: None,
	}
}
