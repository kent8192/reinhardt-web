//! Custom regex validator with preset patterns and capture group support

use crate::{ValidationError, ValidationResult, Validator};
use regex::Regex;

/// Custom regex validator with support for user-defined patterns, preset patterns,
/// and capture group extraction.
///
/// This validator provides:
/// - User-defined regex patterns
/// - Preset patterns (alphanumeric, slug, username)
/// - Inverse matching (validation fails if pattern matches)
/// - Custom error messages
/// - Capture group extraction
pub struct CustomRegexValidator {
	pattern: Regex,
	inverse_match: bool,
	message: Option<String>,
}

impl CustomRegexValidator {
	/// Creates a new CustomRegexValidator with the specified regex pattern.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{CustomRegexValidator, Validator};
	///
	/// let validator = CustomRegexValidator::new(r"^\d{3}-\d{4}$").unwrap();
	/// assert!(validator.validate("123-4567").is_ok());
	/// assert!(validator.validate("invalid").is_err());
	/// ```
	///
	/// # Errors
	///
	/// Returns an error if the regex pattern is invalid.
	pub fn new(pattern: &str) -> Result<Self, ValidationError> {
		let regex = Regex::new(pattern)
			.map_err(|e| ValidationError::Custom(format!("Invalid regex pattern: {}", e)))?;
		Ok(Self {
			pattern: regex,
			inverse_match: false,
			message: None,
		})
	}

	/// Creates a validator for alphanumeric strings (letters and numbers only).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{CustomRegexValidator, Validator};
	///
	/// let validator = CustomRegexValidator::alphanumeric();
	/// assert!(validator.validate("abc123").is_ok());
	/// assert!(validator.validate("ABC123").is_ok());
	/// assert!(validator.validate("abc-123").is_err());
	/// assert!(validator.validate("hello_world").is_err());
	/// ```
	pub fn alphanumeric() -> Self {
		Self {
			pattern: Regex::new(r"^[a-zA-Z0-9]+$").unwrap(),
			inverse_match: false,
			message: Some("Value must contain only alphanumeric characters".to_string()),
		}
	}

	/// Creates a validator for URL slugs (lowercase letters, numbers, hyphens, and underscores).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{CustomRegexValidator, Validator};
	///
	/// let validator = CustomRegexValidator::slug();
	/// assert!(validator.validate("my-valid-slug").is_ok());
	/// assert!(validator.validate("my_slug_123").is_ok());
	/// assert!(validator.validate("invalid slug").is_err());
	/// assert!(validator.validate("UPPERCASE").is_err());
	/// ```
	pub fn slug() -> Self {
		Self {
			pattern: Regex::new(r"^[a-z0-9_-]+$").unwrap(),
			inverse_match: false,
			message: Some(
				"Value must be a valid slug (lowercase letters, numbers, hyphens, and underscores)"
					.to_string(),
			),
		}
	}

	/// Creates a validator for usernames (letters, numbers, underscores, 3-20 characters).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{CustomRegexValidator, Validator};
	///
	/// let validator = CustomRegexValidator::username();
	/// assert!(validator.validate("john_doe").is_ok());
	/// assert!(validator.validate("user123").is_ok());
	/// assert!(validator.validate("ab").is_err()); // Too short
	/// assert!(validator.validate("this_is_a_very_long_username").is_err()); // Too long
	/// assert!(validator.validate("user-name").is_err()); // Hyphens not allowed
	/// ```
	pub fn username() -> Self {
		Self {
            pattern: Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap(),
            inverse_match: false,
            message: Some("Username must be 3-20 characters and contain only letters, numbers, and underscores".to_string()),
        }
	}

	/// Enables inverse matching - validation fails if the pattern matches.
	///
	/// This is useful for blacklist-style validation where you want to reject
	/// certain patterns rather than accept them.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{CustomRegexValidator, Validator};
	///
	/// // Reject strings containing special characters
	/// let validator = CustomRegexValidator::new(r"[!@#$%^&*()]").unwrap()
	///     .inverse_match();
	/// assert!(validator.validate("hello").is_ok());
	/// assert!(validator.validate("hello!").is_err()); // Contains special character
	/// ```
	pub fn inverse_match(mut self) -> Self {
		self.inverse_match = true;
		self
	}

	/// Sets a custom error message for validation failures.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{CustomRegexValidator, Validator, ValidationError};
	///
	/// let validator = CustomRegexValidator::new(r"^\d+$").unwrap()
	///     .with_message("Value must contain only digits");
	///
	/// match validator.validate("abc") {
	///     Err(ValidationError::PatternMismatch(msg)) => {
	///         assert_eq!(msg, "Value must contain only digits");
	///     }
	///     _ => panic!("Expected PatternMismatch error"),
	/// }
	/// ```
	pub fn with_message(mut self, message: impl Into<String>) -> Self {
		self.message = Some(message.into());
		self
	}

	/// Validates a string against the regex pattern.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{CustomRegexValidator, Validator};
	///
	/// let validator = CustomRegexValidator::new(r"^\d{3}$").unwrap();
	/// assert!(validator.validate("123").is_ok());
	/// assert!(validator.validate("abc").is_err());
	/// ```
	pub fn validate(&self, value: &str) -> Result<(), ValidationError> {
		let matches = self.pattern.is_match(value);
		let valid = if self.inverse_match {
			!matches
		} else {
			matches
		};

		if valid {
			Ok(())
		} else {
			let message = self.message.clone().unwrap_or_else(|| {
				if self.inverse_match {
					format!("Value must not match pattern: {}", self.pattern.as_str())
				} else {
					format!("Value must match pattern: {}", self.pattern.as_str())
				}
			});
			Err(ValidationError::PatternMismatch(message))
		}
	}

	/// Validates a string and extracts capture groups if the pattern matches.
	///
	/// Returns a vector of captured strings. The first element is the entire match,
	/// and subsequent elements are the captured groups.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::CustomRegexValidator;
	///
	/// let validator = CustomRegexValidator::new(r"^(\d{3})-(\d{4})$").unwrap();
	///
	/// // Valid match with captures
	/// let captures = validator.validate_with_captures("123-4567").unwrap();
	/// assert_eq!(captures.len(), 3);
	/// assert_eq!(captures[0], "123-4567"); // Full match
	/// assert_eq!(captures[1], "123");      // First capture group
	/// assert_eq!(captures[2], "4567");     // Second capture group
	///
	/// // Invalid match
	/// assert!(validator.validate_with_captures("invalid").is_err());
	/// ```
	pub fn validate_with_captures(&self, value: &str) -> Result<Vec<String>, ValidationError> {
		if self.inverse_match {
			// Inverse match doesn't support capture groups
			self.validate(value)?;
			return Ok(vec![value.to_string()]);
		}

		if let Some(captures) = self.pattern.captures(value) {
			let captured: Vec<String> = captures
				.iter()
				.filter_map(|m| m.map(|m| m.as_str().to_string()))
				.collect();
			Ok(captured)
		} else {
			let message = self
				.message
				.clone()
				.unwrap_or_else(|| format!("Value must match pattern: {}", self.pattern.as_str()));
			Err(ValidationError::PatternMismatch(message))
		}
	}
}

impl Validator<String> for CustomRegexValidator {
	fn validate(&self, value: &String) -> ValidationResult<()> {
		self.validate(value.as_str())
	}
}

impl Validator<str> for CustomRegexValidator {
	fn validate(&self, value: &str) -> ValidationResult<()> {
		self.validate(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_custom_regex_validator_basic() {
		let validator = CustomRegexValidator::new(r"^\d{3}-\d{4}$").unwrap();
		assert!(validator.validate("123-4567").is_ok());
		assert!(validator.validate("999-9999").is_ok());
		assert!(validator.validate("invalid").is_err());
		assert!(validator.validate("123-456").is_err()); // Too short
		assert!(validator.validate("1234-5678").is_err()); // Wrong format
	}

	#[test]
	fn test_alphanumeric_preset() {
		let validator = CustomRegexValidator::alphanumeric();
		assert!(validator.validate("abc123").is_ok());
		assert!(validator.validate("ABC123").is_ok());
		assert!(validator.validate("OnlyLetters").is_ok());
		assert!(validator.validate("12345").is_ok());
		assert!(validator.validate("abc-123").is_err());
		assert!(validator.validate("hello_world").is_err());
		assert!(validator.validate("hello world").is_err());
		assert!(validator.validate("").is_err());
	}

	#[test]
	fn test_slug_preset() {
		let validator = CustomRegexValidator::slug();
		assert!(validator.validate("my-valid-slug").is_ok());
		assert!(validator.validate("my_slug_123").is_ok());
		assert!(validator.validate("lowercase").is_ok());
		assert!(validator.validate("slug-with-numbers-123").is_ok());
		assert!(validator.validate("invalid slug").is_err()); // Space
		assert!(validator.validate("UPPERCASE").is_err()); // Uppercase
		assert!(validator.validate("slug@special").is_err()); // Special character
		assert!(validator.validate("").is_err());
	}

	#[test]
	fn test_username_preset() {
		let validator = CustomRegexValidator::username();
		assert!(validator.validate("john_doe").is_ok());
		assert!(validator.validate("user123").is_ok());
		assert!(validator.validate("ValidUsername").is_ok());
		assert!(validator.validate("abc").is_ok()); // Minimum length
		assert!(validator.validate("a1234567890123456789").is_ok()); // Maximum length
		assert!(validator.validate("ab").is_err()); // Too short
		assert!(validator.validate("a12345678901234567890").is_err()); // Too long
		assert!(validator.validate("user-name").is_err()); // Hyphen not allowed
		assert!(validator.validate("user name").is_err()); // Space not allowed
		assert!(validator.validate("user@name").is_err()); // Special character
	}

	#[test]
	fn test_inverse_match() {
		// Reject strings containing special characters
		let validator = CustomRegexValidator::new(r"[!@#$%^&*()]")
			.unwrap()
			.inverse_match();
		assert!(validator.validate("hello").is_ok());
		assert!(validator.validate("hello123").is_ok());
		assert!(validator.validate("hello-world").is_ok());
		assert!(validator.validate("hello!").is_err());
		assert!(validator.validate("user@domain").is_err());
		assert!(validator.validate("test#123").is_err());
	}

	#[test]
	fn test_custom_message() {
		let validator = CustomRegexValidator::new(r"^\d+$")
			.unwrap()
			.with_message("Value must contain only digits");

		assert!(validator.validate("12345").is_ok());

		match validator.validate("abc") {
			Err(ValidationError::PatternMismatch(msg)) => {
				assert_eq!(msg, "Value must contain only digits");
			}
			_ => panic!("Expected PatternMismatch error"),
		}
	}

	#[test]
	fn test_validate_with_captures() {
		let validator = CustomRegexValidator::new(r"^(\d{3})-(\d{4})$").unwrap();

		// Valid match with captures
		let captures = validator.validate_with_captures("123-4567").unwrap();
		assert_eq!(captures.len(), 3);
		assert_eq!(captures[0], "123-4567"); // Full match
		assert_eq!(captures[1], "123"); // First capture group
		assert_eq!(captures[2], "4567"); // Second capture group

		// Invalid match
		assert!(validator.validate_with_captures("invalid").is_err());
	}

	#[test]
	fn test_validate_with_captures_email_pattern() {
		let validator = CustomRegexValidator::new(r"^([a-z0-9]+)@([a-z0-9]+)\.([a-z]+)$").unwrap();

		let captures = validator
			.validate_with_captures("user@example.com")
			.unwrap();
		assert_eq!(captures.len(), 4);
		assert_eq!(captures[0], "user@example.com"); // Full match
		assert_eq!(captures[1], "user"); // Username
		assert_eq!(captures[2], "example"); // Domain
		assert_eq!(captures[3], "com"); // TLD
	}

	#[test]
	fn test_validate_with_captures_no_groups() {
		let validator = CustomRegexValidator::new(r"^\d{3}$").unwrap();

		let captures = validator.validate_with_captures("123").unwrap();
		assert_eq!(captures.len(), 1);
		assert_eq!(captures[0], "123"); // Only full match
	}

	#[test]
	fn test_validate_with_captures_inverse_match() {
		let validator = CustomRegexValidator::new(r"[!@#]").unwrap().inverse_match();

		// Inverse match doesn't support meaningful captures
		let captures = validator.validate_with_captures("hello").unwrap();
		assert_eq!(captures.len(), 1);
		assert_eq!(captures[0], "hello");

		// Should fail if pattern matches
		assert!(validator.validate_with_captures("hello!").is_err());
	}

	#[test]
	fn test_invalid_regex_pattern() {
		let result = CustomRegexValidator::new(r"[invalid(");
		assert!(result.is_err());
		match result {
			Err(ValidationError::Custom(msg)) => {
				assert!(msg.contains("Invalid regex pattern"));
			}
			_ => panic!("Expected Custom error for invalid regex"),
		}
	}

	#[test]
	fn test_preset_custom_message() {
		let validator =
			CustomRegexValidator::alphanumeric().with_message("Only letters and numbers allowed");

		match validator.validate("hello-world") {
			Err(ValidationError::PatternMismatch(msg)) => {
				assert_eq!(msg, "Only letters and numbers allowed");
			}
			_ => panic!("Expected PatternMismatch error"),
		}
	}

	#[test]
	fn test_validator_trait_string() {
		let validator = CustomRegexValidator::new(r"^\d+$").unwrap();
		let value = String::from("12345");
		assert!(Validator::<String>::validate(&validator, &value).is_ok());

		let invalid = String::from("abc");
		assert!(Validator::<String>::validate(&validator, &invalid).is_err());
	}

	#[test]
	fn test_validator_trait_str() {
		let validator = CustomRegexValidator::new(r"^\d+$").unwrap();
		assert!(Validator::<str>::validate(&validator, "12345").is_ok());
		assert!(Validator::<str>::validate(&validator, "abc").is_err());
	}

	#[test]
	fn test_complex_pattern_with_multiple_groups() {
		// URL pattern: protocol://domain:port/path
		let validator = CustomRegexValidator::new(r"^(https?)://([a-z.]+):(\d+)(/.*)?$").unwrap();

		let captures = validator
			.validate_with_captures("http://example.com:8080/path/to/resource")
			.unwrap();
		assert_eq!(captures.len(), 5);
		assert_eq!(captures[0], "http://example.com:8080/path/to/resource");
		assert_eq!(captures[1], "http");
		assert_eq!(captures[2], "example.com");
		assert_eq!(captures[3], "8080");
		assert_eq!(captures[4], "/path/to/resource");
	}

	#[test]
	fn test_optional_capture_groups() {
		// Pattern with optional group
		let validator = CustomRegexValidator::new(r"^(\d{3})(-(\d{4}))?$").unwrap();

		// With optional group
		let captures = validator.validate_with_captures("123-4567").unwrap();
		assert_eq!(captures.len(), 4);
		assert_eq!(captures[0], "123-4567");
		assert_eq!(captures[1], "123");
		assert_eq!(captures[2], "-4567");
		assert_eq!(captures[3], "4567");

		// Without optional group
		let captures = validator.validate_with_captures("123").unwrap();
		assert_eq!(captures.len(), 2); // Only captured groups that matched
		assert_eq!(captures[0], "123");
		assert_eq!(captures[1], "123");
	}
}
