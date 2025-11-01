//! Phone number validation
//!
//! This module provides validators for international phone numbers in E.164 format.

use crate::{ValidationError, ValidationResult, Validator};
use regex::Regex;

/// Phone number validator for international phone numbers
///
/// Validates phone numbers in E.164 format: +[country code][number]
pub struct PhoneNumberValidator {
	/// Optional list of allowed country codes (e.g., ["1", "81", "44"])
	pub country_codes: Option<Vec<String>>,
	/// Whether to allow extension numbers
	pub allow_extensions: bool,
	regex: Regex,
	extension_regex: Regex,
}

impl PhoneNumberValidator {
	/// Creates a new PhoneNumberValidator that allows all country codes.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{PhoneNumberValidator, Validator};
	///
	/// let validator = PhoneNumberValidator::new();
	/// assert!(validator.validate("+1234567890").is_ok());
	/// assert!(validator.validate("+81-90-1234-5678").is_ok());
	/// ```
	pub fn new() -> Self {
		Self {
			country_codes: None,
			allow_extensions: false,
			// E.164 format: + followed by country code (1-3 digits starting with non-zero) and number
			// Allows optional hyphens, spaces, dots, and parentheses for readability
			regex: Regex::new(r"^\+([1-9]\d{0,2})[\s.\-()]*\d+[\s.\-\d()]*$").unwrap(),
			extension_regex: Regex::new(r"^(.+?)(?:\s*(?:ext\.?|x|extension)\s*(\d+))?$").unwrap(),
		}
	}

	/// Creates a validator that only allows specific country codes.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{PhoneNumberValidator, Validator};
	///
	/// let validator = PhoneNumberValidator::with_countries(vec![
	///     "1".to_string(),   // US/Canada
	///     "81".to_string(),  // Japan
	///     "44".to_string(),  // UK
	/// ]);
	///
	/// assert!(validator.validate("+1234567890").is_ok());
	/// assert!(validator.validate("+81-90-1234-5678").is_ok());
	/// assert!(validator.validate("+33123456789").is_err()); // France not allowed
	/// ```
	pub fn with_countries(codes: Vec<String>) -> Self {
		Self {
			country_codes: Some(codes),
			allow_extensions: false,
			regex: Regex::new(r"^\+([1-9]\d{0,2})[\s.\-()]*\d+[\s.\-\d()]*$").unwrap(),
			extension_regex: Regex::new(r"^(.+?)(?:\s*(?:ext\.?|x|extension)\s*(\d+))?$").unwrap(),
		}
	}

	/// Configures whether to allow extension numbers.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{PhoneNumberValidator, Validator};
	///
	/// let validator = PhoneNumberValidator::new().with_extensions(true);
	/// assert!(validator.validate("+1234567890 ext. 123").is_ok());
	/// assert!(validator.validate("+1234567890 x123").is_ok());
	/// assert!(validator.validate("+1234567890 extension 123").is_ok());
	/// ```
	pub fn with_extensions(mut self, allow: bool) -> Self {
		self.allow_extensions = allow;
		self
	}

	/// Validates a phone number string.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::{PhoneNumberValidator, Validator};
	///
	/// let validator = PhoneNumberValidator::new();
	/// assert!(validator.validate("+1234567890").is_ok());
	/// assert!(validator.validate("+81-90-1234-5678").is_ok());
	/// assert!(validator.validate("1234567890").is_err()); // Missing +
	/// assert!(validator.validate("+123").is_err()); // Too short
	/// ```
	pub fn validate(&self, value: &str) -> Result<(), ValidationError> {
		let trimmed = value.trim();

		if trimmed.is_empty() {
			return Err(ValidationError::InvalidPhoneNumber(
				"Phone number cannot be empty".to_string(),
			));
		}

		// Extract base number and extension (if any)
		let (base_number, extension) = if let Some(caps) = self.extension_regex.captures(trimmed) {
			let base = caps.get(1).map_or("", |m| m.as_str());
			let ext = caps.get(2).map(|m| m.as_str());
			(base, ext)
		} else {
			(trimmed, None)
		};

		// If extension exists but not allowed, return error
		if extension.is_some() && !self.allow_extensions {
			return Err(ValidationError::InvalidPhoneNumber(
				"Extensions are not allowed".to_string(),
			));
		}

		// Validate base number format
		if !self.regex.is_match(base_number) {
			return Err(ValidationError::InvalidPhoneNumber(
				"Phone number must be in E.164 format: +[country code][number]".to_string(),
			));
		}

		// Extract country code
		let country_code = self.extract_country_code(base_number)?;

		// Validate country code if whitelist exists
		if let Some(ref allowed_codes) = self.country_codes
			&& !allowed_codes.contains(&country_code) {
				return Err(ValidationError::CountryCodeNotAllowed {
					country_code,
					allowed_countries: allowed_codes.join(", "),
				});
			}

		// Validate total length (E.164 allows max 15 digits including country code)
		let digit_count = base_number.chars().filter(|c| c.is_ascii_digit()).count();
		if !(5..=15).contains(&digit_count) {
			return Err(ValidationError::InvalidPhoneNumber(format!(
				"Phone number must contain 5-15 digits, got {}",
				digit_count
			)));
		}

		Ok(())
	}

	/// Extracts the country code from a phone number
	fn extract_country_code(&self, number: &str) -> Result<String, ValidationError> {
		if !number.starts_with('+') {
			return Err(ValidationError::InvalidPhoneNumber(
				"Phone number must start with +".to_string(),
			));
		}

		// Remove the + prefix
		let digits_part = &number[1..];

		// Extract leading digits (country code can be 1-3 digits)
		let country_code_digits: String = digits_part
			.chars()
			.take_while(|c| c.is_ascii_digit())
			.collect();

		if country_code_digits.is_empty() {
			return Err(ValidationError::InvalidPhoneNumber(
				"No digits found after +".to_string(),
			));
		}

		// Country code extraction logic:
		// This is a simplified heuristic. A complete implementation would use
		// an actual country code database.

		// Known 1-digit country codes (North America and Russia)
		const SINGLE_DIGIT_CODES: &[&str] = &["1", "7"];

		// Known 3-digit country codes (selected examples)
		const THREE_DIGIT_CODES: &[&str] = &[
			"353", "358", "372", "374", "375", "376", "377", "378", "380", "381", "382", "385",
			"386", "387", "389",
		];

		// Try 1-digit codes first
		if !country_code_digits.is_empty() {
			let first_digit = &country_code_digits[0..1];
			if SINGLE_DIGIT_CODES.contains(&first_digit) {
				return Ok(first_digit.to_string());
			}
		}

		// Try 3-digit codes if we have enough digits
		if country_code_digits.len() >= 3 {
			let three_digit = &country_code_digits[0..3];
			if THREE_DIGIT_CODES.contains(&three_digit) {
				return Ok(three_digit.to_string());
			}
		}

		// Default to 2-digit code
		if country_code_digits.len() >= 2 {
			return Ok(country_code_digits[0..2].to_string());
		}

		// Fallback to whatever we have
		Ok(country_code_digits)
	}
}

impl Default for PhoneNumberValidator {
	fn default() -> Self {
		Self::new()
	}
}

impl Validator<String> for PhoneNumberValidator {
	fn validate(&self, value: &String) -> ValidationResult<()> {
		PhoneNumberValidator::validate(self, value.as_str())
	}
}

impl Validator<str> for PhoneNumberValidator {
	fn validate(&self, value: &str) -> ValidationResult<()> {
		PhoneNumberValidator::validate(self, value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_us_phone_number() {
		let validator = PhoneNumberValidator::new();
		assert!(validator.validate("+12025551234").is_ok());
		assert!(validator.validate("+1 202 555 1234").is_ok());
		assert!(validator.validate("+1-202-555-1234").is_ok());
		assert!(validator.validate("+1 (202) 555-1234").is_ok());
	}

	#[test]
	fn test_valid_japan_phone_number() {
		let validator = PhoneNumberValidator::new();
		assert!(validator.validate("+819012345678").is_ok());
		assert!(validator.validate("+81-90-1234-5678").is_ok());
		assert!(validator.validate("+81 90 1234 5678").is_ok());
	}

	#[test]
	fn test_valid_uk_phone_number() {
		let validator = PhoneNumberValidator::new();
		assert!(validator.validate("+442012345678").is_ok());
		assert!(validator.validate("+44-20-1234-5678").is_ok());
		assert!(validator.validate("+44 20 1234 5678").is_ok());
	}

	#[test]
	fn test_invalid_missing_plus() {
		let validator = PhoneNumberValidator::new();
		let result = validator.validate("12025551234");
		assert!(result.is_err());
		match result {
			Err(ValidationError::InvalidPhoneNumber(msg)) => {
				assert!(msg.contains("E.164 format"));
			}
			_ => panic!("Expected InvalidPhoneNumber error"),
		}
	}

	#[test]
	fn test_invalid_too_short() {
		let validator = PhoneNumberValidator::new();
		let result = validator.validate("+123");
		assert!(result.is_err());
		match result {
			Err(ValidationError::InvalidPhoneNumber(msg)) => {
				assert!(msg.contains("5-15 digits"));
			}
			_ => panic!("Expected InvalidPhoneNumber error"),
		}
	}

	#[test]
	fn test_invalid_too_long() {
		let validator = PhoneNumberValidator::new();
		let result = validator.validate("+12345678901234567890");
		assert!(result.is_err());
		match result {
			Err(ValidationError::InvalidPhoneNumber(msg)) => {
				assert!(msg.contains("5-15 digits"));
			}
			_ => panic!("Expected InvalidPhoneNumber error"),
		}
	}

	#[test]
	fn test_invalid_empty_string() {
		let validator = PhoneNumberValidator::new();
		let result = validator.validate("");
		assert!(result.is_err());
		match result {
			Err(ValidationError::InvalidPhoneNumber(msg)) => {
				assert!(msg.contains("cannot be empty"));
			}
			_ => panic!("Expected InvalidPhoneNumber error"),
		}
	}

	#[test]
	fn test_invalid_characters() {
		let validator = PhoneNumberValidator::new();
		assert!(validator.validate("+1202abc5678").is_err());
		assert!(validator.validate("+1202#5678901").is_err());
	}

	#[test]
	fn test_country_code_whitelist() {
		let validator = PhoneNumberValidator::with_countries(vec![
			"1".to_string(),
			"81".to_string(),
			"44".to_string(),
		]);

		// Allowed countries
		assert!(validator.validate("+12025551234").is_ok());
		assert!(validator.validate("+819012345678").is_ok());
		assert!(validator.validate("+442012345678").is_ok());

		// Disallowed country (France +33)
		let result = validator.validate("+33123456789");
		assert!(result.is_err());
		match result {
			Err(ValidationError::CountryCodeNotAllowed {
				country_code,
				allowed_countries,
			}) => {
				assert_eq!(country_code, "33");
				assert!(allowed_countries.contains("1"));
				assert!(allowed_countries.contains("81"));
				assert!(allowed_countries.contains("44"));
			}
			_ => panic!("Expected CountryCodeNotAllowed error"),
		}
	}

	#[test]
	fn test_extensions_allowed() {
		let validator = PhoneNumberValidator::new().with_extensions(true);

		assert!(validator.validate("+12025551234 ext. 123").is_ok());
		assert!(validator.validate("+12025551234 x123").is_ok());
		assert!(validator.validate("+12025551234 extension 456").is_ok());
		assert!(validator.validate("+12025551234 ext 789").is_ok());
	}

	#[test]
	fn test_extensions_not_allowed() {
		let validator = PhoneNumberValidator::new();

		// Extensions should fail when not allowed
		let result = validator.validate("+12025551234 ext. 123");
		assert!(result.is_err());
		match result {
			Err(ValidationError::InvalidPhoneNumber(msg)) => {
				assert!(msg.contains("Extensions are not allowed"));
			}
			_ => panic!("Expected InvalidPhoneNumber error for extension"),
		}
	}

	#[test]
	fn test_various_formatting() {
		let validator = PhoneNumberValidator::new();

		// Different separators
		assert!(validator.validate("+1-202-555-1234").is_ok());
		assert!(validator.validate("+1.202.555.1234").is_ok());
		assert!(validator.validate("+1 202 555 1234").is_ok());
		assert!(validator.validate("+12025551234").is_ok());

		// With parentheses
		assert!(validator.validate("+1 (202) 555-1234").is_ok());
	}

	#[test]
	fn test_edge_cases() {
		let validator = PhoneNumberValidator::new();

		// Minimum valid length (5 digits)
		assert!(validator.validate("+112345").is_ok());

		// Maximum valid length (15 digits)
		assert!(validator.validate("+123456789012345").is_ok());

		// Country code starting with 0 is invalid in E.164
		assert!(validator.validate("+0123456789").is_err());
	}

	#[test]
	fn test_validator_trait_with_string() {
		let validator = PhoneNumberValidator::new();
		let number = String::from("+12025551234");

		// Test with String
		assert!(Validator::<String>::validate(&validator, &number).is_ok());

		// Test with &str
		assert!(Validator::<str>::validate(&validator, &number).is_ok());
	}

	#[test]
	fn test_country_code_extraction() {
		let validator = PhoneNumberValidator::new();

		// 1 digit country code
		assert_eq!(validator.extract_country_code("+12025551234").unwrap(), "1");

		// 2 digit country code
		assert_eq!(
			validator.extract_country_code("+819012345678").unwrap(),
			"81"
		);

		// 3 digit country code
		assert_eq!(
			validator.extract_country_code("+3531234567").unwrap(),
			"353"
		);
	}

	#[test]
	fn test_default_constructor() {
		let validator = PhoneNumberValidator::default();
		assert!(validator.validate("+12025551234").is_ok());
	}

	#[test]
	fn test_whitespace_handling() {
		let validator = PhoneNumberValidator::new();

		// Leading and trailing whitespace should be trimmed
		assert!(validator.validate("  +12025551234  ").is_ok());
		assert!(validator.validate("\t+12025551234\n").is_ok());
	}

	#[test]
	fn test_combined_country_codes_and_extensions() {
		let validator =
			PhoneNumberValidator::with_countries(vec!["1".to_string(), "81".to_string()])
				.with_extensions(true);

		// Allowed country with extension
		assert!(validator.validate("+12025551234 ext. 123").is_ok());
		assert!(validator.validate("+819012345678 x456").is_ok());

		// Disallowed country with extension
		let result = validator.validate("+442012345678 ext. 789");
		assert!(result.is_err());
		if let Err(ValidationError::CountryCodeNotAllowed { .. }) = result {
			// Expected
		} else {
			panic!("Expected CountryCodeNotAllowed error");
		}
	}
}
