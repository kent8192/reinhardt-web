//! Conditional validator for applying validators based on runtime conditions
//!
//! This module provides conditional validation that applies validators only when
//! certain conditions are met.
//!
//! # Examples
//!
//! ## Apply validator only when condition is true
//!
//! ```
//! use crate::validators::{ConditionalValidator, MinLengthValidator, Validator};
//!
//! let validator = ConditionalValidator::when(
//!     |value: &str| value.starts_with("admin_"),
//!     Box::new(MinLengthValidator::new(10)),
//! );
//!
//! // For admin users, minimum length is 10
//! assert!(validator.validate("admin_john").is_ok());
//! assert!(validator.validate("admin_j").is_err());
//!
//! // For non-admin users, no validation
//! assert!(validator.validate("john").is_ok());
//! ```
//!
//! ## Apply validator unless condition is true
//!
//! ```
//! use crate::validators::{ConditionalValidator, EmailValidator, Validator};
//!
//! let validator = ConditionalValidator::unless(
//!     |value: &str| value.starts_with("system:"),
//!     Box::new(EmailValidator::new()),
//! );
//!
//! // System users are exempt from email validation
//! assert!(validator.validate("system:admin").is_ok());
//!
//! // Regular values must be valid emails
//! assert!(validator.validate("user@example.com").is_ok());
//! assert!(validator.validate("invalid").is_err());
//! ```

use super::{ValidationResult, Validator};

/// Conditional validator that applies validation based on a condition
///
/// The validator contains a condition function and a validator to apply.
/// The condition is evaluated, and if it matches the expected state
/// (`validate_when_true`), the validator is applied.
pub struct ConditionalValidator<T: ?Sized, C>
where
	C: Fn(&T) -> bool,
{
	condition: C,
	validator: Box<dyn Validator<T>>,
	validate_when_true: bool,
}

impl<T: ?Sized, C> ConditionalValidator<T, C>
where
	C: Fn(&T) -> bool,
{
	/// Create a conditional validator that applies when condition is true
	///
	/// # Example
	///
	/// ```
	/// use crate::validators::{ConditionalValidator, MinLengthValidator, Validator};
	///
	/// let validator = ConditionalValidator::when(
	///     |value: &str| value.starts_with("admin_"),
	///     Box::new(MinLengthValidator::new(10)),
	/// );
	///
	/// assert!(validator.validate("admin_john_doe").is_ok());
	/// assert!(validator.validate("admin_joe").is_err()); // Too short
	/// assert!(validator.validate("regular_user").is_ok()); // No validation
	/// ```
	pub fn when(condition: C, validator: Box<dyn Validator<T>>) -> Self {
		Self {
			condition,
			validator,
			validate_when_true: true,
		}
	}

	/// Create a conditional validator that applies when condition is false
	///
	/// # Example
	///
	/// ```
	/// use crate::validators::{ConditionalValidator, EmailValidator, Validator};
	///
	/// let validator = ConditionalValidator::unless(
	///     |value: &str| value.starts_with("system:"),
	///     Box::new(EmailValidator::new()),
	/// );
	///
	/// assert!(validator.validate("system:admin").is_ok()); // Exempt
	/// assert!(validator.validate("user@example.com").is_ok()); // Valid email
	/// assert!(validator.validate("invalid").is_err()); // Invalid email
	/// ```
	pub fn unless(condition: C, validator: Box<dyn Validator<T>>) -> Self {
		Self {
			condition,
			validator,
			validate_when_true: false,
		}
	}
}

impl<T: ?Sized, C> Validator<T> for ConditionalValidator<T, C>
where
	C: Fn(&T) -> bool,
{
	fn validate(&self, value: &T) -> ValidationResult<()> {
		let condition_result = (self.condition)(value);

		// Apply validator if condition matches expected state
		if condition_result == self.validate_when_true {
			self.validator.validate(value)
		} else {
			// Condition not met, skip validation
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::validators::{EmailValidator, MaxLengthValidator, MinLengthValidator};

	// Tests for ConditionalValidator::when
	#[test]
	fn test_when_condition_true_and_valid() {
		let validator = ConditionalValidator::when(
			|value: &str| value.starts_with("admin_"),
			Box::new(MinLengthValidator::new(10)),
		);

		// Condition true, validation passes
		assert!(validator.validate("admin_john_doe").is_ok());
	}

	#[test]
	fn test_when_condition_true_and_invalid() {
		let validator = ConditionalValidator::when(
			|value: &str| value.starts_with("admin_"),
			Box::new(MinLengthValidator::new(10)),
		);

		// Condition true, validation fails
		assert!(validator.validate("admin_joe").is_err());
	}

	#[test]
	fn test_when_condition_false() {
		let validator = ConditionalValidator::when(
			|value: &str| value.starts_with("admin_"),
			Box::new(MinLengthValidator::new(10)),
		);

		// Condition false, no validation (always passes)
		assert!(validator.validate("joe").is_ok());
		assert!(validator.validate("regular_user").is_ok());
	}

	// Tests for ConditionalValidator::unless
	#[test]
	fn test_unless_condition_false_and_valid() {
		let validator = ConditionalValidator::unless(
			|value: &str| value.starts_with("system:"),
			Box::new(EmailValidator::new()),
		);

		// Condition false, validation applies and passes
		assert!(validator.validate("user@example.com").is_ok());
	}

	#[test]
	fn test_unless_condition_false_and_invalid() {
		let validator = ConditionalValidator::unless(
			|value: &str| value.starts_with("system:"),
			Box::new(EmailValidator::new()),
		);

		// Condition false, validation applies and fails
		assert!(validator.validate("invalid").is_err());
	}

	#[test]
	fn test_unless_condition_true() {
		let validator = ConditionalValidator::unless(
			|value: &str| value.starts_with("system:"),
			Box::new(EmailValidator::new()),
		);

		// Condition true, no validation (always passes)
		assert!(validator.validate("system:admin").is_ok());
		assert!(validator.validate("system:root").is_ok());
	}

	// Complex condition tests
	#[test]
	fn test_complex_condition() {
		let validator = ConditionalValidator::when(
			|value: &str| value.len() > 5 && value.contains("@"),
			Box::new(EmailValidator::new()),
		);

		// Condition true, email validation applies
		assert!(validator.validate("user@example.com").is_ok());
		assert!(validator.validate("invalid@").is_err());

		// Condition false (no @), no validation
		assert!(validator.validate("short").is_ok());
		assert!(validator.validate("longusername").is_ok());
	}

	// Numeric validator tests
	#[test]
	fn test_when_with_numeric_validator() {
		use crate::validators::RangeValidator;

		let validator = ConditionalValidator::when(
			|value: &i32| *value >= 0,
			Box::new(RangeValidator::new(0, 100)),
		);

		// Condition true (positive), range validation applies
		assert!(validator.validate(&50).is_ok());
		assert!(validator.validate(&150).is_err()); // Out of range

		// Condition false (negative), no validation
		assert!(validator.validate(&-50).is_ok());
	}

	// Nested conditional validators
	#[test]
	fn test_nested_conditional() {
		let inner_validator = ConditionalValidator::when(
			|value: &str| value.len() > 10,
			Box::new(MaxLengthValidator::new(20)),
		);

		let outer_validator = ConditionalValidator::unless(
			|value: &str| value.starts_with("skip:"),
			Box::new(inner_validator),
		);

		// Outer condition false, inner condition true, validation applies
		assert!(outer_validator.validate("test_username").is_ok()); // 13 chars, < 20
		assert!(
			outer_validator
				.validate("test_very_long_username_exceeds")
				.is_err()
		); // > 20

		// Outer condition false, inner condition false, no validation
		assert!(outer_validator.validate("short").is_ok());

		// Outer condition true, skip all validation
		assert!(outer_validator.validate("skip:anything").is_ok());
		assert!(
			outer_validator
				.validate("skip:very_long_username_exceeds_limit")
				.is_ok()
		);
	}

	// Edge cases
	#[test]
	fn test_always_true_condition() {
		let validator =
			ConditionalValidator::when(|_: &str| true, Box::new(MinLengthValidator::new(5)));

		// Condition always true, validation always applies
		assert!(validator.validate("hello").is_ok());
		assert!(validator.validate("hi").is_err());
	}

	#[test]
	fn test_always_false_condition() {
		let validator =
			ConditionalValidator::when(|_: &str| false, Box::new(MinLengthValidator::new(5)));

		// Condition always false, validation never applies
		assert!(validator.validate("hello").is_ok());
		assert!(validator.validate("hi").is_ok());
	}

	// Real-world scenario tests
	#[test]
	fn test_password_strength_for_admin() {
		let validator = ConditionalValidator::when(
			|username: &str| username.starts_with("admin_") || username.starts_with("root_"),
			Box::new(MinLengthValidator::new(12)),
		);

		// Admin users need strong passwords (12+ chars)
		assert!(validator.validate("admin_12345678901").is_ok());
		assert!(validator.validate("admin_short").is_err());

		// Regular users can have shorter passwords
		assert!(validator.validate("user_short").is_ok());
	}

	#[test]
	fn test_email_validation_for_external_users() {
		let validator = ConditionalValidator::unless(
			|email: &str| email.ends_with("@internal.company.com"),
			Box::new(EmailValidator::new()),
		);

		// Internal emails are exempt from validation
		assert!(validator.validate("user@internal.company.com").is_ok());

		// External emails must be valid
		assert!(validator.validate("user@example.com").is_ok());
		assert!(validator.validate("invalid@").is_err());
	}
}
