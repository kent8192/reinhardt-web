//! Validator composition for combining multiple validators with AND/OR logic
//!
//! This module provides composable validators that allow combining multiple validation
//! rules using logical operations.
//!
//! # Examples
//!
//! ## AND composition - All validators must pass
//!
//! ```
//! use reinhardt_core::validators::{AndValidator, MinLengthValidator, MaxLengthValidator, Validator};
//!
//! let validator = AndValidator::new(vec![
//!     Box::new(MinLengthValidator::new(3)),
//!     Box::new(MaxLengthValidator::new(20)),
//! ]);
//!
//! assert!(validator.validate("john").is_ok());
//! assert!(validator.validate("jo").is_err()); // Too short
//! assert!(validator.validate("verylongusernamethatexceedslimit").is_err()); // Too long
//! ```
//!
//! ## OR composition - At least one validator must pass
//!
//! ```
//! use reinhardt_core::validators::{OrValidator, EmailValidator, UrlValidator, Validator};
//!
//! let validator = OrValidator::new(vec![
//!     Box::new(EmailValidator::new()),
//!     Box::new(UrlValidator::new()),
//! ]);
//!
//! assert!(validator.validate("user@example.com").is_ok()); // Valid email
//! assert!(validator.validate("http://example.com").is_ok()); // Valid URL
//! assert!(validator.validate("invalid").is_err()); // Neither email nor URL
//! ```

use super::{ValidationError, ValidationResult, Validator};

/// Combines multiple validators with AND logic - all must pass
///
/// This validator succeeds only if all contained validators succeed.
/// Short-circuits on the first failure.
pub struct AndValidator<T: ?Sized> {
	validators: Vec<Box<dyn Validator<T>>>,
}

impl<T: ?Sized> AndValidator<T> {
	/// Create a new AND validator with the given validators
	pub fn new(validators: Vec<Box<dyn Validator<T>>>) -> Self {
		Self { validators }
	}

	/// Add a validator to the composition
	pub fn with_validator(mut self, validator: Box<dyn Validator<T>>) -> Self {
		self.validators.push(validator);
		self
	}
}

impl<T: ?Sized> Validator<T> for AndValidator<T> {
	fn validate(&self, value: &T) -> ValidationResult<()> {
		for validator in &self.validators {
			validator.validate(value)?;
		}
		Ok(())
	}
}

/// Combines multiple validators with OR logic - at least one must pass
///
/// This validator succeeds if any of the contained validators succeeds.
/// Can optionally collect all errors if all validators fail.
pub struct OrValidator<T: ?Sized> {
	validators: Vec<Box<dyn Validator<T>>>,
	collect_errors: bool,
}

impl<T: ?Sized> OrValidator<T> {
	/// Create a new OR validator with the given validators
	pub fn new(validators: Vec<Box<dyn Validator<T>>>) -> Self {
		Self {
			validators,
			collect_errors: false,
		}
	}

	/// Enable error collection from all validators
	///
	/// When enabled, if all validators fail, the error will include
	/// messages from all validators instead of just the first one.
	pub fn with_error_collection(mut self, collect: bool) -> Self {
		self.collect_errors = collect;
		self
	}
}

impl<T: ?Sized> Validator<T> for OrValidator<T> {
	fn validate(&self, value: &T) -> ValidationResult<()> {
		if self.validators.is_empty() {
			return Ok(());
		}

		let mut errors = Vec::new();

		for validator in &self.validators {
			match validator.validate(value) {
				Ok(()) => return Ok(()), // Success - short circuit
				Err(e) if self.collect_errors => errors.push(e.to_string()),
				Err(_) => {} // Ignore if not collecting
			}
		}

		// All validators failed
		if self.collect_errors && !errors.is_empty() {
			Err(ValidationError::AllValidatorsFailed {
				errors: errors.join("; "),
			})
		} else {
			Err(ValidationError::CompositeValidationFailed(
				"All validators failed".to_string(),
			))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::validators::{MaxLengthValidator, MinLengthValidator};

	// AND validator tests
	#[test]
	fn test_and_validator_all_pass() {
		let validator = AndValidator::new(vec![
			Box::new(MinLengthValidator::new(3)),
			Box::new(MaxLengthValidator::new(10)),
		]);

		assert!(validator.validate("hello").is_ok());
		assert!(validator.validate("test").is_ok());
	}

	#[test]
	fn test_and_validator_first_fails() {
		let validator = AndValidator::new(vec![
			Box::new(MinLengthValidator::new(5)),
			Box::new(MaxLengthValidator::new(10)),
		]);

		assert!(validator.validate("hi").is_err()); // Too short
	}

	#[test]
	fn test_and_validator_second_fails() {
		let validator = AndValidator::new(vec![
			Box::new(MinLengthValidator::new(3)),
			Box::new(MaxLengthValidator::new(5)),
		]);

		assert!(validator.validate("toolong").is_err()); // Too long
	}

	#[test]
	fn test_and_validator_all_fail() {
		let validator = AndValidator::new(vec![
			Box::new(MinLengthValidator::new(10)),
			Box::new(MaxLengthValidator::new(5)),
		]);

		// This is invalid config (min > max), but first validator fails
		assert!(validator.validate("test").is_err());
	}

	#[test]
	fn test_and_validator_empty() {
		let validator: AndValidator<str> = AndValidator::new(vec![]);
		assert!(validator.validate("anything").is_ok());
	}

	#[test]
	fn test_and_validator_add_method() {
		let validator = AndValidator::new(vec![Box::new(MinLengthValidator::new(3))])
			.with_validator(Box::new(MaxLengthValidator::new(10)));

		assert!(validator.validate("hello").is_ok());
		assert!(validator.validate("hi").is_err());
		assert!(validator.validate("verylongtext").is_err());
	}

	// OR validator tests
	#[test]
	fn test_or_validator_first_passes() {
		let validator = OrValidator::new(vec![
			Box::new(MinLengthValidator::new(3)),
			Box::new(MaxLengthValidator::new(10)),
		]);

		assert!(validator.validate("hello").is_ok()); // Both pass, but first is enough
	}

	#[test]
	fn test_or_validator_second_passes() {
		let validator = OrValidator::new(vec![
			Box::new(MinLengthValidator::new(10)), // Fails
			Box::new(MaxLengthValidator::new(10)), // Passes
		]);

		assert!(validator.validate("short").is_ok()); // Second validator passes
	}

	#[test]
	fn test_or_validator_all_fail() {
		let validator = OrValidator::new(vec![
			Box::new(MinLengthValidator::new(10)),
			Box::new(MinLengthValidator::new(20)),
		]);

		assert!(validator.validate("short").is_err());
	}

	#[test]
	fn test_or_validator_empty() {
		let validator: OrValidator<str> = OrValidator::new(vec![]);
		assert!(validator.validate("anything").is_ok());
	}

	#[test]
	fn test_or_validator_with_error_collection() {
		let validator = OrValidator::new(vec![
			Box::new(MinLengthValidator::new(10)),
			Box::new(MinLengthValidator::new(20)),
		])
		.with_error_collection(true);

		match validator.validate("short") {
			Err(ValidationError::AllValidatorsFailed { errors }) => {
				assert!(errors.contains("minimum: 10"));
				assert!(errors.contains("minimum: 20"));
			}
			_ => panic!("Expected AllValidatorsFailed error"),
		}
	}

	#[test]
	fn test_or_validator_without_error_collection() {
		let validator = OrValidator::new(vec![
			Box::new(MinLengthValidator::new(10)),
			Box::new(MinLengthValidator::new(20)),
		]);

		match validator.validate("short") {
			Err(ValidationError::CompositeValidationFailed(_)) => {}
			_ => panic!("Expected CompositeValidationFailed error"),
		}
	}

	// Nested composition tests
	#[test]
	fn test_nested_and_in_or() {
		let and_validator = AndValidator::new(vec![
			Box::new(MinLengthValidator::new(3)),
			Box::new(MaxLengthValidator::new(10)),
		]);

		let or_validator = OrValidator::new(vec![
			Box::new(and_validator),
			Box::new(MinLengthValidator::new(20)),
		]);

		assert!(or_validator.validate("hello").is_ok()); // Passes first (AND: 3-10 chars)
		assert!(
			or_validator
				.validate("verylongusernameexceeds20chars")
				.is_ok()
		); // Passes second (20+ chars)
		assert!(or_validator.validate("hi").is_err()); // Fails both (too short for both)
	}

	#[test]
	fn test_nested_or_in_and() {
		let or_validator = OrValidator::new(vec![
			Box::new(MinLengthValidator::new(3)),
			Box::new(MinLengthValidator::new(5)),
		]);

		let and_validator = AndValidator::new(vec![
			Box::new(or_validator),
			Box::new(MaxLengthValidator::new(10)),
		]);

		assert!(and_validator.validate("hello").is_ok()); // Passes both
		assert!(and_validator.validate("verylongtext").is_err()); // Fails max length
		assert!(and_validator.validate("hi").is_err()); // Fails min length
	}
}
