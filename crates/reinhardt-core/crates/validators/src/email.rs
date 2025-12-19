//! Email validator

use crate::lazy_patterns::EMAIL_REGEX;
use crate::{ValidationError, ValidationResult, Validator};

/// Email address validator
pub struct EmailValidator {
	message: Option<String>,
}

impl EmailValidator {
	/// Creates a new EmailValidator with RFC 5322 compliant validation
	///
	/// This implementation follows RFC 5322 specifications with the following rules:
	/// - Local part (before @):
	///   - Can contain alphanumeric characters, dots, underscores, percent signs, plus and minus signs
	///   - Cannot start or end with a dot
	///   - Cannot have consecutive dots
	///   - Maximum 64 characters
	/// - Domain part (after @):
	///   - Can contain alphanumeric characters, dots, and hyphens
	///   - Cannot start or end with a dot or hyphen
	///   - Must have at least one dot
	///   - Each label must be 1-63 characters
	///   - TLD must be at least 2 characters
	///   - Maximum 255 characters
	/// - Total length must not exceed 320 characters (64 + @ + 255)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::EmailValidator;
	///
	/// let validator = EmailValidator::new();
	/// ```
	pub fn new() -> Self {
		Self { message: None }
	}

	/// Sets a custom error message for validation failures.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{EmailValidator, Validator};
	///
	/// let validator = EmailValidator::new().with_message("Invalid email address");
	/// let result = validator.validate("not-an-email");
	/// assert!(result.is_err());
	/// ```
	pub fn with_message(mut self, message: impl Into<String>) -> Self {
		self.message = Some(message.into());
		self
	}

	/// Validates an email address with additional RFC 5322 length constraints
	fn validate_with_length_check(&self, email: &str) -> bool {
		// Check total length (max 320 characters: 64 for local + 1 for @ + 255 for domain)
		if email.len() > 320 {
			return false;
		}

		// Split email into local and domain parts
		let parts: Vec<&str> = email.split('@').collect();
		if parts.len() != 2 {
			return false;
		}

		let local_part = parts[0];
		let domain_part = parts[1];

		// Check local part length (max 64 characters)
		if local_part.is_empty() || local_part.len() > 64 {
			return false;
		}

		// Check domain part length (max 255 characters)
		if domain_part.is_empty() || domain_part.len() > 255 {
			return false;
		}

		// Check for consecutive dots in local part
		if local_part.contains("..") {
			return false;
		}

		// Check each domain label length (max 63 characters per label)
		for label in domain_part.split('.') {
			if label.is_empty() || label.len() > 63 {
				return false;
			}
		}

		// Finally, check against the regex pattern
		EMAIL_REGEX.is_match(email)
	}
}

impl Default for EmailValidator {
	fn default() -> Self {
		Self::new()
	}
}

impl Validator<String> for EmailValidator {
	fn validate(&self, value: &String) -> ValidationResult<()> {
		if self.validate_with_length_check(value) {
			Ok(())
		} else if let Some(ref msg) = self.message {
			Err(ValidationError::Custom(msg.clone()))
		} else {
			Err(ValidationError::InvalidEmail(value.clone()))
		}
	}
}

impl Validator<str> for EmailValidator {
	fn validate(&self, value: &str) -> ValidationResult<()> {
		if self.validate_with_length_check(value) {
			Ok(())
		} else if let Some(ref msg) = self.message {
			Err(ValidationError::Custom(msg.clone()))
		} else {
			Err(ValidationError::InvalidEmail(value.to_string()))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_emails() {
		let validator = EmailValidator::new();
		let valid_emails = vec![
			"test@example.com",
			"user.name@example.com",
			"user+tag@example.co.uk",
			"user_name@example.com",
			"user%test@example.com",
			"user-name@sub.example.com",
			"a@example.com",
			"test@sub.sub.example.com",
			"123@example.com",
		];

		for email in valid_emails {
			assert!(
				validator.validate(email).is_ok(),
				"Expected {} to be valid",
				email
			);
		}
	}

	#[test]
	fn test_invalid_emails() {
		let validator = EmailValidator::new();
		let invalid_emails = vec![
			"invalid-email",          // No @ symbol
			"@example.com",           // No local part
			"user@",                  // No domain
			"user..name@example.com", // Consecutive dots
			".user@example.com",      // Starts with dot
			"user.@example.com",      // Ends with dot
			"user@-example.com",      // Domain starts with hyphen
			"user@example-.com",      // Domain label ends with hyphen
			"user@example",           // No TLD
			"user@example.c",         // TLD too short
			"user name@example.com",  // Space in local part
			"user@exam ple.com",      // Space in domain
			"user@@example.com",      // Double @
			"user@.example.com",      // Domain starts with dot
			"user@example.com.",      // Domain ends with dot
		];

		for email in invalid_emails {
			assert!(
				validator.validate(email).is_err(),
				"Expected {} to be invalid",
				email
			);
		}
	}

	#[test]
	fn test_length_constraints() {
		let validator = EmailValidator::new();

		// Local part too long (> 64 characters)
		let long_local = format!("{}@example.com", "a".repeat(65));
		assert!(validator.validate(&long_local).is_err());

		// Domain too long (> 255 characters)
		let long_domain = format!("user@{}.com", "a".repeat(252));
		assert!(validator.validate(&long_domain).is_err());

		// Total length too long (> 320 characters)
		let very_long_email = format!("{}@{}.com", "a".repeat(64), "b".repeat(252));
		assert!(validator.validate(&very_long_email).is_err());

		// Domain label too long (> 63 characters)
		let long_label = format!("user@{}.example.com", "a".repeat(64));
		assert!(validator.validate(&long_label).is_err());

		// Valid at maximum lengths
		let max_local = format!("{}@example.com", "a".repeat(64));
		assert!(validator.validate(&max_local).is_ok());
	}

	#[test]
	fn test_case_insensitivity() {
		let validator = EmailValidator::new();
		assert!(validator.validate("Test@Example.COM").is_ok());
		assert!(validator.validate("USER@EXAMPLE.COM").is_ok());
	}

	// Additional tests based on Django validators/tests.py - TestValidatorEquality::test_email_equality
	#[test]
	fn test_email_validator_with_numbers() {
		let validator = EmailValidator::new();
		assert!(validator.validate("123@example.com").is_ok());
		assert!(validator.validate("user123@example.com").is_ok());
		assert!(validator.validate("123user@example123.com").is_ok());
	}

	#[test]
	fn test_email_validator_with_special_characters() {
		let validator = EmailValidator::new();
		// Valid special characters
		assert!(validator.validate("user+tag@example.com").is_ok());
		assert!(validator.validate("user_name@example.com").is_ok());
		assert!(validator.validate("user-name@example.com").is_ok());
		assert!(validator.validate("user.name@example.com").is_ok());
		assert!(validator.validate("user%test@example.com").is_ok());
	}

	#[test]
	fn test_email_validator_subdomains() {
		let validator = EmailValidator::new();
		assert!(validator.validate("user@mail.example.com").is_ok());
		assert!(validator.validate("user@sub.mail.example.com").is_ok());
		assert!(validator.validate("user@a.b.c.d.example.com").is_ok());
	}

	#[test]
	fn test_email_validator_tld_variations() {
		let validator = EmailValidator::new();
		assert!(validator.validate("user@example.co").is_ok());
		assert!(validator.validate("user@example.com").is_ok());
		assert!(validator.validate("user@example.org").is_ok());
		assert!(validator.validate("user@example.net").is_ok());
		assert!(validator.validate("user@example.info").is_ok());
		assert!(validator.validate("user@example.museum").is_ok());
	}

	#[test]
	fn test_email_validator_edge_cases() {
		let validator = EmailValidator::new();
		// Single character local and domain parts
		assert!(validator.validate("a@b.co").is_ok());

		// Numbers in domain
		assert!(validator.validate("user@123.com").is_ok());
		assert!(validator.validate("user@example123.com").is_ok());
	}

	#[test]
	fn test_email_validator_invalid_formats() {
		let validator = EmailValidator::new();
		// Multiple @ symbols
		assert!(validator.validate("user@domain@example.com").is_err());

		// Missing parts
		assert!(validator.validate("@").is_err());
		assert!(validator.validate("user@").is_err());
		assert!(validator.validate("@domain.com").is_err());

		// Invalid characters
		assert!(validator.validate("user name@example.com").is_err());
		assert!(validator.validate("user@exam ple.com").is_err());
		assert!(validator.validate("user@example,com").is_err());
	}

	#[test]
	fn test_email_validator_dot_rules() {
		let validator = EmailValidator::new();
		// Consecutive dots in local part
		assert!(validator.validate("user..name@example.com").is_err());

		// Starting with dot
		assert!(validator.validate(".user@example.com").is_err());

		// Ending with dot
		assert!(validator.validate("user.@example.com").is_err());

		// Valid dot usage
		assert!(validator.validate("user.name.test@example.com").is_ok());
	}

	#[test]
	fn test_email_validator_hyphen_rules() {
		let validator = EmailValidator::new();
		// Hyphens in domain are allowed in middle
		assert!(validator.validate("user@my-domain.com").is_ok());
		assert!(validator.validate("user@my-long-domain-name.com").is_ok());

		// But not at start or end of domain labels
		assert!(validator.validate("user@-invalid.com").is_err());
		assert!(validator.validate("user@invalid-.com").is_err());
		assert!(validator.validate("user@invalid.-com").is_err());
		assert!(validator.validate("user@invalid.com-").is_err());
	}

	#[test]
	fn test_email_validator_returns_correct_error() {
		let validator = EmailValidator::new();
		let invalid_email = "invalid";
		match validator.validate(invalid_email) {
			Err(ValidationError::InvalidEmail(email)) => {
				assert_eq!(email, invalid_email);
			}
			_ => panic!("Expected InvalidEmail error"),
		}
	}

	#[test]
	fn test_email_validator_with_string_type() {
		let validator = EmailValidator::new();
		let email = String::from("test@example.com");
		assert!(validator.validate(&email).is_ok());

		let invalid = String::from("invalid");
		assert!(validator.validate(&invalid).is_err());
	}
}
