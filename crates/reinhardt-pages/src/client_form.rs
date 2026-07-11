//! Runtime support types for DTO-derived client forms.

use std::hash::Hash;

/// One selectable option exposed by a DTO enum choice source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientFormChoice<T> {
	/// Typed value written to the generated form state.
	pub value: T,
	/// Serialized value used by generated HTML controls.
	pub serialized_value: &'static str,
	/// Human-readable label shown by generated controls.
	pub label: &'static str,
}

/// Source of selectable choices for DTO enum fields.
pub trait ClientFormChoiceSource: Sized + Clone + PartialEq + 'static {
	/// Returns all choices accepted by generated client forms.
	fn client_form_choices() -> &'static [ClientFormChoice<Self>];

	/// Returns the initial value for non-optional generated choice fields.
	fn client_form_default() -> Self;
}

/// Private helpers used by generated client-form code.
#[doc(hidden)]
pub mod __private {
	use super::*;
	use crate::form_state::FormValidationError;
	use reinhardt_core::validators::{Validate, ValidationErrors};

	/// Validator trait implemented by DTO validation derives.
	pub use reinhardt_core::validators::Validate as DtoValidate;
	/// Field error type returned by DTO validation derives.
	pub use reinhardt_core::validators::ValidationError as DtoValidationError;
	/// Aggregate error type returned by DTO validation derives.
	pub use reinhardt_core::validators::ValidationErrors as DtoValidationErrors;

	/// Converts DTO validation errors into generated form validation errors.
	pub fn validate_dto_request<Request, Field>(
		request: &Request,
		resolve_field: impl Fn(&str) -> Option<Field>,
	) -> Result<(), FormValidationError<Field>>
	where
		Request: Validate,
		Field: Copy + Eq + Hash,
	{
		request
			.validate()
			.map_err(|errors| dto_errors_to_form_errors(errors, resolve_field))
	}

	/// Converts already-collected DTO validation errors into form validation errors.
	pub fn dto_errors_to_form_errors<Field>(
		errors: ValidationErrors,
		resolve_field: impl Fn(&str) -> Option<Field>,
	) -> FormValidationError<Field>
	where
		Field: Copy + Eq + Hash,
	{
		let mut form_errors = FormValidationError::new();
		let mut form_level_messages = Vec::new();

		for (field_name, field_errors) in errors.field_errors() {
			let message = field_errors
				.iter()
				.map(ToString::to_string)
				.collect::<Vec<_>>()
				.join(", ");

			match resolve_field(field_name.as_ref()) {
				Some(field) => {
					form_errors.add_field_error(field, message);
				}
				None => {
					form_level_messages.push(format!("{field_name}: {message}"));
				}
			}
		}

		if !form_level_messages.is_empty() {
			form_errors.set_form_error(form_level_messages.join(", "));
		}

		form_errors
	}
}
