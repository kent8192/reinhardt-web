//! Typed form state runtime API.

use std::cell::Cell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::reactive::{Effect, EffectTiming, Signal};

/// Field-level validation errors keyed by form field name.
pub type FieldErrors = HashMap<String, Vec<String>>;

type SubmitFuture = Pin<Box<dyn Future<Output = Result<(), String>>>>;
type SubmitHandler<V> = Rc<dyn Fn(V) -> SubmitFuture>;
type ValidateHandler<V> = Rc<dyn Fn(&V) -> Result<(), FormValidationError>>;

/// Validation failure emitted by typed form validation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormValidationError {
	field_errors: FieldErrors,
	form_error: Option<String>,
}

impl FormValidationError {
	/// Creates an empty validation error.
	pub fn new() -> Self {
		Self::default()
	}

	/// Creates a validation error for one field.
	pub fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
		let mut field_errors = FieldErrors::new();
		field_errors
			.entry(field.into())
			.or_default()
			.push(message.into());
		Self {
			field_errors,
			form_error: None,
		}
	}

	/// Creates a form-level validation error.
	pub fn form(message: impl Into<String>) -> Self {
		Self {
			field_errors: FieldErrors::new(),
			form_error: Some(message.into()),
		}
	}

	/// Returns field-level validation errors.
	pub fn field_errors(&self) -> &FieldErrors {
		&self.field_errors
	}

	/// Returns the form-level validation error, if any.
	pub fn form_error(&self) -> Option<&str> {
		self.form_error.as_deref()
	}

	fn first_message_for<V>(&self) -> Option<String>
	where
		V: FormValues,
	{
		for field_name in V::field_names() {
			if let Some(message) = self
				.field_errors
				.get(*field_name)
				.and_then(|messages| messages.first())
			{
				return Some(message.clone());
			}
		}

		self.field_errors
			.values()
			.find_map(|messages| messages.first().cloned())
			.or_else(|| self.form_error.clone())
	}
}

/// Typed field signal container used by [`use_form`].
pub trait FormFields: Clone + 'static {
	/// Value struct represented by this field signal container.
	type Values: Clone + PartialEq + 'static;

	/// Creates field signals from an initial value struct.
	fn from_values(values: &Self::Values) -> Self;

	/// Reads the current typed values from field signals.
	fn values(&self) -> Self::Values;

	/// Applies typed values to the field signals.
	fn apply_values(&self, values: &Self::Values);
}

/// Typed value struct used by [`use_form`].
pub trait FormValues: Clone + PartialEq + 'static {
	/// Field signal struct for this value type.
	type Fields: FormFields<Values = Self>;

	/// Stable field names in source order.
	fn field_names() -> &'static [&'static str];
}

/// Validation hook for typed form values.
pub trait FormValidate {
	/// Validates the current form values.
	fn validate(&self) -> Result<(), FormValidationError> {
		Ok(())
	}
}

/// Builder options for [`use_form`].
pub struct FormOptions<V>
where
	V: FormValues,
{
	initial_values: V,
	on_submit: Option<SubmitHandler<V>>,
	validate: Option<ValidateHandler<V>>,
}

impl<V> FormOptions<V>
where
	V: FormValues,
{
	/// Creates typed form options with initial values.
	pub fn new(initial_values: V) -> Self {
		Self {
			initial_values,
			on_submit: None,
			validate: None,
		}
	}

	/// Sets the async submit action.
	///
	/// On `wasm`, [`FormHandle::submit`] awaits the returned future through the
	/// platform task spawner and updates loading/success/error signals from the
	/// result. On `native`, submit still runs synchronous validation but does not
	/// await this future; the async body is not executed.
	pub fn on_submit<F, Fut, E>(mut self, on_submit: F) -> Self
	where
		F: Fn(V) -> Fut + 'static,
		Fut: Future<Output = Result<(), E>> + 'static,
		E: Into<String> + 'static,
	{
		self.on_submit = Some(Rc::new(move |values| {
			let fut = on_submit(values);
			Box::pin(async move { fut.await.map_err(Into::into) })
		}));
		self
	}

	/// Sets an additional sync validation action.
	pub fn validate<F>(mut self, validate: F) -> Self
	where
		F: Fn(&V) -> Result<(), FormValidationError> + 'static,
	{
		self.validate = Some(Rc::new(validate));
		self
	}
}

/// Runtime handle returned by [`use_form`].
pub struct FormHandle<V>
where
	V: FormValues,
{
	initial_values: V,
	fields: V::Fields,
	on_submit: Option<SubmitHandler<V>>,
	validate: Option<ValidateHandler<V>>,
	dirty: Signal<bool>,
	touched: Signal<bool>,
	field_errors: Signal<FieldErrors>,
	form_error: Signal<Option<String>>,
	submit_error: Signal<Option<String>>,
	error: Signal<Option<String>>,
	loading: Signal<bool>,
	success: Signal<bool>,
	suppress_touch: Rc<Cell<bool>>,
	_dirty_effect: Rc<Effect>,
}

impl<V> Clone for FormHandle<V>
where
	V: FormValues,
{
	fn clone(&self) -> Self {
		Self {
			initial_values: self.initial_values.clone(),
			fields: self.fields.clone(),
			on_submit: self.on_submit.clone(),
			validate: self.validate.clone(),
			dirty: self.dirty.clone(),
			touched: self.touched.clone(),
			field_errors: self.field_errors.clone(),
			form_error: self.form_error.clone(),
			submit_error: self.submit_error.clone(),
			error: self.error.clone(),
			loading: self.loading.clone(),
			success: self.success.clone(),
			suppress_touch: Rc::clone(&self.suppress_touch),
			_dirty_effect: Rc::clone(&self._dirty_effect),
		}
	}
}

impl<V> FormHandle<V>
where
	V: FormValues,
{
	/// Returns typed field signals.
	pub fn fields(&self) -> V::Fields {
		self.fields.clone()
	}

	/// Returns current typed values.
	pub fn values(&self) -> V {
		self.fields.values()
	}

	/// Applies new typed values to the form fields.
	pub fn set_values(&self, values: V) {
		self.fields.apply_values(&values);
		self.dirty.set(values != self.initial_values);
		self.touched.set(true);
	}

	/// Restores the initial values and clears transient state.
	pub fn reset(&self) {
		self.suppress_touch.set(true);
		self.fields.apply_values(&self.initial_values);
		self.dirty.set(false);
		self.touched.set(false);
		self.suppress_touch.set(false);
		self.field_errors.set(FieldErrors::new());
		self.form_error.set(None);
		self.submit_error.set(None);
		self.error.set(None);
		self.loading.set(false);
		self.success.set(false);
	}

	/// Returns whether current values differ from the initial values.
	pub fn dirty(&self) -> Signal<bool> {
		self.dirty.clone()
	}

	/// Returns whether the form has been changed since creation or reset.
	pub fn touched(&self) -> Signal<bool> {
		self.touched.clone()
	}

	/// Returns field-level validation errors.
	pub fn field_errors(&self) -> Signal<FieldErrors> {
		self.field_errors.clone()
	}

	/// Returns the form-level validation error.
	pub fn form_error(&self) -> Signal<Option<String>> {
		self.form_error.clone()
	}

	/// Returns the last submit error.
	pub fn submit_error(&self) -> Signal<Option<String>> {
		self.submit_error.clone()
	}

	/// Returns the first visible validation or submit error.
	pub fn error(&self) -> Signal<Option<String>> {
		self.error.clone()
	}

	/// Returns whether an async submit action is running.
	pub fn loading(&self) -> Signal<bool> {
		self.loading.clone()
	}

	/// Returns whether the last async submit action succeeded.
	pub fn success(&self) -> Signal<bool> {
		self.success.clone()
	}

	/// Runs synchronous validation and updates error signals.
	pub fn validate(&self) -> Result<(), FormValidationError> {
		let values = self.values();
		let result = match &self.validate {
			Some(validate) => validate(&values),
			None => Ok(()),
		};

		match result {
			Ok(()) => {
				self.field_errors.set(FieldErrors::new());
				self.form_error.set(None);
				self.sync_error();
				Ok(())
			}
			Err(error) => {
				self.apply_validation_error(&error);
				self.success.set(false);
				Err(error)
			}
		}
	}

	/// Runs validation and starts the submit action when validation passes.
	///
	/// On `wasm`, the async submit action runs in the platform task spawner and
	/// updates loading/success/error when it completes. On `native`, the
	/// returned future is created and immediately dropped, so only synchronous
	/// validation state is observable.
	pub fn submit(&self) {
		self.submit_error.set(None);
		self.success.set(false);
		self.sync_error();

		if self.validate().is_err() {
			self.loading.set(false);
			return;
		}

		let Some(on_submit) = &self.on_submit else {
			self.loading.set(false);
			self.success.set(true);
			self.sync_error();
			return;
		};

		let values = self.values();

		#[cfg(native)]
		{
			let _future = on_submit(values);
			self.loading.set(false);
		}

		#[cfg(wasm)]
		{
			let on_submit = Rc::clone(on_submit);
			let loading = self.loading.clone();
			let success = self.success.clone();
			let submit_error = self.submit_error.clone();
			let error = self.error.clone();
			loading.set(true);
			crate::platform::spawn_task(async move {
				match on_submit(values).await {
					Ok(()) => {
						loading.set(false);
						success.set(true);
						submit_error.set(None);
						error.set(None);
					}
					Err(message) => {
						loading.set(false);
						success.set(false);
						submit_error.set(Some(message.clone()));
						error.set(Some(message));
					}
				}
			});
		}
	}

	fn apply_validation_error(&self, validation_error: &FormValidationError) {
		self.field_errors
			.set(validation_error.field_errors().clone());
		self.form_error
			.set(validation_error.form_error().map(str::to_string));
		self.error.set(validation_error.first_message_for::<V>());
	}

	fn sync_error(&self) {
		let field_errors = self.field_errors.get();
		let form_error = self.form_error.get();
		let submit_error = self.submit_error.get();
		let validation_error = FormValidationError {
			field_errors,
			form_error,
		};
		self.error
			.set(validation_error.first_message_for::<V>().or(submit_error));
	}
}

/// Creates a typed form runtime handle.
pub fn use_form<V>(options: FormOptions<V>) -> FormHandle<V>
where
	V: FormValues,
{
	let fields = V::Fields::from_values(&options.initial_values);
	let dirty = Signal::new(false);
	let touched = Signal::new(false);
	let field_errors = Signal::new(FieldErrors::new());
	let form_error = Signal::new(None);
	let submit_error = Signal::new(None);
	let error = Signal::new(None);
	let loading = Signal::new(false);
	let success = Signal::new(false);
	let suppress_touch = Rc::new(Cell::new(false));

	let fields_for_effect = fields.clone();
	let initial_values_for_effect = options.initial_values.clone();
	let dirty_for_effect = dirty.clone();
	let touched_for_effect = touched.clone();
	let suppress_touch_for_effect = Rc::clone(&suppress_touch);
	let mut first_run = true;
	let dirty_effect = Effect::new_with_timing(
		move || {
			let values = fields_for_effect.values();
			dirty_for_effect.set(values != initial_values_for_effect);
			if first_run {
				first_run = false;
			} else if !suppress_touch_for_effect.get() {
				touched_for_effect.set(true);
			}
		},
		EffectTiming::Layout,
	);

	FormHandle {
		initial_values: options.initial_values,
		fields,
		on_submit: options.on_submit,
		validate: options.validate,
		dirty,
		touched,
		field_errors,
		form_error,
		submit_error,
		error,
		loading,
		success,
		suppress_touch,
		_dirty_effect: Rc::new(dirty_effect),
	}
}
