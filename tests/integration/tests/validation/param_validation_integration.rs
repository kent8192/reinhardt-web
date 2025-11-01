//! Integration tests for parameter validation
//!
//! These tests verify that reinhardt-validators work correctly with parameters.

use reinhardt_validators::{
	EmailValidator, MaxLengthValidator, MaxValueValidator, MinLengthValidator, MinValueValidator,
	RangeValidator, RegexValidator, Validator,
};

// ============================================================================
// Numeric Validation Tests
// ============================================================================

#[test]
fn test_range_validator_valid() {
	let validator = RangeValidator::new(2, 10);

	// Valid value within range
	assert!(validator.validate(&5).is_ok());
	assert!(validator.validate(&2).is_ok()); // Boundary: min
	assert!(validator.validate(&10).is_ok()); // Boundary: max
}

#[test]
fn test_range_validator_invalid() {
	let validator = RangeValidator::new(2, 10);

	// Invalid: too small
	let result = validator.validate(&1);
	assert!(result.is_err());

	// Invalid: too large
	let result = validator.validate(&123);
	assert!(result.is_err());
}

#[test]
fn test_param_validation_min_value() {
	let validator = MinValueValidator::new(1);

	// Valid
	assert!(validator.validate(&1).is_ok());
	assert!(validator.validate(&100).is_ok());

	// Invalid
	assert!(validator.validate(&0).is_err());
}

#[test]
fn test_max_value_validator() {
	let validator = MaxValueValidator::new(50);

	// Valid
	assert!(validator.validate(&50).is_ok());
	assert!(validator.validate(&1).is_ok());

	// Invalid
	assert!(validator.validate(&200).is_err());
}

// ============================================================================
// String Length Validation Tests
// ============================================================================

#[test]
fn test_min_length_validator() {
	let validator = MinLengthValidator::new(3);

	// Valid
	assert!(validator.validate("abc").is_ok());
	assert!(validator.validate("abcdef").is_ok());

	// Invalid: too short
	assert!(validator.validate("ab").is_err());
}

#[test]
fn test_param_validation_max_length() {
	let validator = MaxLengthValidator::new(20);

	// Valid
	assert!(validator.validate("hello").is_ok());
	assert!(validator.validate("12345678901234567890").is_ok()); // Exactly 20

	// Invalid: too long
	assert!(validator.validate("123456789012345678901").is_err()); // 21 chars
}

#[test]
fn test_string_length_combined() {
	let min_validator = MinLengthValidator::new(3);
	let max_validator = MaxLengthValidator::new(20);

	let test_value = "username";

	// Valid: passes both validators
	assert!(min_validator.validate(test_value).is_ok());
	assert!(max_validator.validate(test_value).is_ok());

	// Invalid: too short
	let short_value = "ab";
	assert!(min_validator.validate(short_value).is_err());

	// Invalid: too long
	let long_value = "a".repeat(21);
	assert!(max_validator.validate(&long_value).is_err());
}

// ============================================================================
// Email Validation Tests
// ============================================================================

#[test]
fn test_email_validator_valid() {
	let validator = EmailValidator::new();

	// Valid emails
	let valid_emails = vec![
		"user@example.com",
		"test.user@example.com",
		"user+tag@example.co.uk",
		"admin@subdomain.example.com",
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
fn test_email_validator_invalid() {
	let validator = EmailValidator::new();

	// Invalid emails
	let invalid_emails = vec![
		"not-an-email",
		"@example.com",
		"user@",
		"user..name@example.com",
		".user@example.com",
	];

	for email in invalid_emails {
		assert!(
			validator.validate(email).is_err(),
			"Expected {} to be invalid",
			email
		);
	}
}

// ============================================================================
// Regex/Pattern Validation Tests
// ============================================================================

#[test]
fn test_regex_validator_username() {
	// Username must be alphanumeric or underscore
	let validator = RegexValidator::new(r"^[a-zA-Z0-9_]+$")
		.unwrap()
		.with_message("Username must be alphanumeric or underscore");

	// Valid usernames
	assert!(validator.validate("john_doe").is_ok());
	assert!(validator.validate("user123").is_ok());
	assert!(validator.validate("Admin_User").is_ok());

	// Invalid usernames
	assert!(validator.validate("john-doe").is_err()); // Has hyphen
	assert!(validator.validate("user@name").is_err()); // Has @
	assert!(validator.validate("test user").is_err()); // Has space
}

#[test]
fn test_regex_validator_phone() {
	// Phone format: 123-4567
	let validator = RegexValidator::new(r"^\d{3}-\d{4}$").unwrap();

	// Valid
	assert!(validator.validate("123-4567").is_ok());
	assert!(validator.validate("999-0000").is_ok());

	// Invalid
	assert!(validator.validate("1234567").is_err()); // No hyphen
	assert!(validator.validate("12-34567").is_err()); // Wrong format
	assert!(validator.validate("abc-defg").is_err()); // Not digits
}

// ============================================================================
// Multiple Validators Combined Tests
// ============================================================================

#[test]
fn test_multiple_validators_all_pass() {
	// Simulate validating a registration form field
	let username = "john_doe";

	let min_len = MinLengthValidator::new(3);
	let max_len = MaxLengthValidator::new(20);
	let pattern = RegexValidator::new(r"^[a-zA-Z0-9_]+$").unwrap();

	// All validators should pass
	assert!(min_len.validate(username).is_ok());
	assert!(max_len.validate(username).is_ok());
	assert!(pattern.validate(username).is_ok());
}

#[test]
fn test_multiple_validators_collect_errors() {
	// Test that we can detect multiple validation failures
	let invalid_username = "ab"; // Too short, doesn't match pattern

	let min_len = MinLengthValidator::new(3);
	let pattern = RegexValidator::new(r"^[a-zA-Z0-9_]+$").unwrap();

	let mut errors = Vec::new();

	if let Err(e) = min_len.validate(invalid_username) {
		errors.push(format!("min_length: {:?}", e));
	}
	if let Err(e) = pattern.validate(invalid_username) {
		errors.push(format!("pattern: {:?}", e));
	}

	// Should have at least one error
	assert!(!errors.is_empty());
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_validator_boundary_values() {
	let range = RangeValidator::new(0, 100);

	// Boundary: exactly min
	assert!(range.validate(&0).is_ok());

	// Boundary: exactly max
	assert!(range.validate(&100).is_ok());

	// Just below min
	assert!(range.validate(&-1).is_err());

	// Just above max
	assert!(range.validate(&101).is_err());
}

#[test]
fn test_string_validator_unicode() {
	let min_len = MinLengthValidator::new(3);

	// Unicode characters should count correctly
	assert!(min_len.validate("你好世界").is_ok()); // 4 characters (12 bytes)
	assert!(min_len.validate("🎉🎉🎉").is_ok()); // 3 emoji (12 bytes)
}

#[test]
fn test_email_validator_edge_cases() {
	let validator = EmailValidator::new();

	// Single letter local part
	assert!(validator.validate("a@example.com").is_ok());

	// Numbers in domain
	assert!(validator.validate("user@123.com").is_ok());

	// Multiple subdomains
	assert!(validator.validate("user@mail.sub.example.com").is_ok());
}
