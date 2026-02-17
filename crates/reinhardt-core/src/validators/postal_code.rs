//! Postal code validator for country-specific postal code formats
//!
//! This validator provides support for validating postal codes from multiple countries
//! with country-specific format rules.
//!
//! # Supported Countries
//!
//! - United States (US): ZIP and ZIP+4 format
//! - United Kingdom (UK): Complex UK postcode format
//! - Japan (JP): 7-digit format with hyphen
//! - Canada (CA): Alphanumeric format
//! - Germany (DE): 5-digit format
//!
//! # Examples
//!
//! ## Validate with country restriction
//!
//! ```
//! use reinhardt_core::validators::{PostalCodeValidator, Country, Validator};
//!
//! let validator = PostalCodeValidator::with_countries(vec![
//!     Country::US,
//!     Country::JP,
//! ]);
//!
//! assert!(validator.validate("12345").is_ok()); // US ZIP
//! assert!(validator.validate("123-4567").is_ok()); // Japan
//! assert!(validator.validate("SW1A 1AA").is_err()); // UK not allowed
//! ```
//!
//! ## Validate with country detection
//!
//! ```
//! use reinhardt_core::validators::{PostalCodeValidator, Country, Validator};
//!
//! let validator = PostalCodeValidator::new();
//!
//! let country = validator.validate_with_country("12345-6789").unwrap();
//! assert_eq!(country, Country::US);
//!
//! let country = validator.validate_with_country("123-4567").unwrap();
//! assert_eq!(country, Country::JP);
//! ```

use regex::Regex;
use std::collections::HashMap;

use super::{ValidationError, ValidationResult, Validator};

/// Supported countries for postal code validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Country {
	/// United States - ZIP code (5 digits) or ZIP+4 (5+4 digits with hyphen)
	US,
	/// United Kingdom - Complex alphanumeric format
	UK,
	/// Japan - 7 digits with hyphen (XXX-XXXX)
	JP,
	/// Canada - Alphanumeric format (A#A #A#)
	CA,
	/// Germany - 5 digits
	DE,
}

impl Country {
	/// Get the regex pattern for this country's postal code format
	fn pattern(&self) -> &'static str {
		match self {
			Country::US => r"^\d{5}(-\d{4})?$",
			Country::UK => r"^[A-Z]{1,2}\d{1,2}[A-Z]?\s?\d[A-Z]{2}$",
			Country::JP => r"^\d{3}-\d{4}$",
			Country::CA => r"^[A-Z]\d[A-Z]\s?\d[A-Z]\d$",
			Country::DE => r"^\d{5}$",
		}
	}

	/// Get the country code as a string
	fn code(&self) -> &'static str {
		match self {
			Country::US => "US",
			Country::UK => "UK",
			Country::JP => "JP",
			Country::CA => "CA",
			Country::DE => "DE",
		}
	}

	/// Get all supported countries
	fn all() -> Vec<Country> {
		vec![
			Country::US,
			Country::UK,
			Country::JP,
			Country::CA,
			Country::DE,
		]
	}
}

/// Postal code validator with country-specific format validation
///
/// Validates postal codes according to country-specific formats.
/// Can restrict validation to specific countries or auto-detect the country.
#[derive(Debug)]
pub struct PostalCodeValidator {
	allowed_countries: Option<Vec<Country>>,
	patterns: HashMap<Country, Regex>,
}

impl PostalCodeValidator {
	/// Create a new validator that accepts all supported countries
	pub fn new() -> Self {
		let mut patterns = HashMap::new();
		for country in Country::all() {
			patterns.insert(
				country,
				Regex::new(country.pattern()).expect("Invalid regex pattern"),
			);
		}

		Self {
			allowed_countries: None,
			patterns,
		}
	}

	/// Create a validator restricted to specific countries
	pub fn with_countries(countries: Vec<Country>) -> Self {
		let mut patterns = HashMap::new();
		for country in &countries {
			patterns.insert(
				*country,
				Regex::new(country.pattern()).expect("Invalid regex pattern"),
			);
		}

		Self {
			allowed_countries: Some(countries),
			patterns,
		}
	}

	/// Create a validator for a single country
	pub fn for_country(country: Country) -> Self {
		Self::with_countries(vec![country])
	}

	/// Validate a postal code and return the detected country
	///
	/// # Returns
	///
	/// Returns the country if validation succeeds, or an error if:
	/// - The postal code doesn't match any country format
	/// - The detected country is not in the allowed list
	pub fn validate_with_country(&self, value: &str) -> Result<Country, ValidationError> {
		let value = value.trim().to_uppercase();

		// Define priority order - more specific patterns first
		let priority_order = vec![
			Country::UK, // Most specific (with space and complex pattern)
			Country::CA, // Alphanumeric pattern
			Country::JP, // Has hyphen
			Country::US, // Can match simple 5 digits, but check ZIP+4 first
			Country::DE, // Simple 5 digits
		];

		// Determine which countries to check
		let countries_to_check: Vec<Country> = if let Some(ref allowed) = self.allowed_countries {
			// Only check allowed countries, but maintain priority order
			priority_order
				.into_iter()
				.filter(|c| allowed.contains(c))
				.collect()
		} else {
			// Check all countries in priority order
			priority_order
		};

		// Try patterns in priority order
		for country in countries_to_check {
			if let Some(pattern) = self.patterns.get(&country)
				&& pattern.is_match(&value)
			{
				return Ok(country);
			}
		}

		// No pattern matched - check if it matches a non-allowed country
		if let Some(ref allowed) = self.allowed_countries {
			// Check if value matches any country pattern that's not allowed
			for country in Country::all() {
				if !allowed.contains(&country)
					&& let Ok(pattern) = Regex::new(country.pattern())
					&& pattern.is_match(&value)
				{
					return Err(ValidationError::PostalCodeCountryNotAllowed {
						country: country.code().to_string(),
						allowed_countries: allowed
							.iter()
							.map(|c| c.code())
							.collect::<Vec<_>>()
							.join(", "),
					});
				}
			}
		}

		// No pattern matched at all
		Err(ValidationError::PostalCodeCountryNotRecognized { postal_code: value })
	}
}

impl Default for PostalCodeValidator {
	fn default() -> Self {
		Self::new()
	}
}

impl Validator<str> for PostalCodeValidator {
	fn validate(&self, value: &str) -> ValidationResult<()> {
		self.validate_with_country(value).map(|_| ())
	}
}

impl Validator<String> for PostalCodeValidator {
	fn validate(&self, value: &String) -> ValidationResult<()> {
		self.validate(value.as_str())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	// US postal code tests
	#[rstest]
	fn test_us_zip_code() {
		let validator = PostalCodeValidator::for_country(Country::US);
		assert!(validator.validate("12345").is_ok());
		assert!(validator.validate("90210").is_ok());
	}

	#[rstest]
	fn test_us_zip_plus_4() {
		let validator = PostalCodeValidator::for_country(Country::US);
		assert!(validator.validate("12345-6789").is_ok());
		assert!(validator.validate("90210-1234").is_ok());
	}

	#[rstest]
	fn test_us_invalid_format() {
		let validator = PostalCodeValidator::for_country(Country::US);
		assert!(validator.validate("1234").is_err()); // Too short
		assert!(validator.validate("123456").is_err()); // Too long
		assert!(validator.validate("ABCDE").is_err()); // Not numeric
	}

	// Japan postal code tests
	#[rstest]
	fn test_jp_postal_code() {
		let validator = PostalCodeValidator::for_country(Country::JP);
		assert!(validator.validate("123-4567").is_ok());
		assert!(validator.validate("100-0001").is_ok()); // Tokyo
	}

	#[rstest]
	fn test_jp_invalid_format() {
		let validator = PostalCodeValidator::for_country(Country::JP);
		assert!(validator.validate("1234567").is_err()); // Missing hyphen
		assert!(validator.validate("12-34567").is_err()); // Wrong hyphen position
		assert!(validator.validate("ABC-DEFG").is_err()); // Not numeric
	}

	// UK postal code tests
	#[rstest]
	fn test_uk_postal_code() {
		let validator = PostalCodeValidator::for_country(Country::UK);
		assert!(validator.validate("SW1A 1AA").is_ok());
		assert!(validator.validate("M1 1AE").is_ok());
		assert!(validator.validate("B33 8TH").is_ok());
		assert!(validator.validate("CR2 6XH").is_ok());
		assert!(validator.validate("DN55 1PT").is_ok());
	}

	#[rstest]
	fn test_uk_postal_code_without_space() {
		let validator = PostalCodeValidator::for_country(Country::UK);
		// UK postcodes can be written without space
		assert!(validator.validate("SW1A1AA").is_ok());
		assert!(validator.validate("M11AE").is_ok());
	}

	// Canada postal code tests
	#[rstest]
	fn test_ca_postal_code() {
		let validator = PostalCodeValidator::for_country(Country::CA);
		assert!(validator.validate("K1A 0B1").is_ok());
		assert!(validator.validate("M5W 1E6").is_ok()); // Toronto
	}

	#[rstest]
	fn test_ca_postal_code_without_space() {
		let validator = PostalCodeValidator::for_country(Country::CA);
		assert!(validator.validate("K1A0B1").is_ok());
	}

	#[rstest]
	fn test_ca_invalid_format() {
		let validator = PostalCodeValidator::for_country(Country::CA);
		assert!(validator.validate("K1A 0B").is_err()); // Too short
		assert!(validator.validate("111 111").is_err()); // All digits
	}

	// Germany postal code tests
	#[rstest]
	fn test_de_postal_code() {
		let validator = PostalCodeValidator::for_country(Country::DE);
		assert!(validator.validate("12345").is_ok());
		assert!(validator.validate("10115").is_ok()); // Berlin
		assert!(validator.validate("80331").is_ok()); // Munich
	}

	#[rstest]
	fn test_de_invalid_format() {
		let validator = PostalCodeValidator::for_country(Country::DE);
		assert!(validator.validate("1234").is_err()); // Too short
		assert!(validator.validate("123456").is_err()); // Too long
		assert!(validator.validate("ABCDE").is_err()); // Not numeric
	}

	// Multi-country validation tests
	#[rstest]
	fn test_multiple_countries() {
		let validator = PostalCodeValidator::with_countries(vec![Country::US, Country::JP]);

		assert!(validator.validate("12345").is_ok()); // US
		assert!(validator.validate("123-4567").is_ok()); // JP
		assert!(validator.validate("SW1A 1AA").is_err()); // UK not allowed
	}

	#[rstest]
	fn test_all_countries() {
		let validator = PostalCodeValidator::new();

		assert!(validator.validate("12345").is_ok()); // US
		assert!(validator.validate("123-4567").is_ok()); // JP
		assert!(validator.validate("SW1A 1AA").is_ok()); // UK
		assert!(validator.validate("K1A 0B1").is_ok()); // CA
		assert!(validator.validate("10115").is_ok()); // DE
	}

	// validate_with_country tests
	#[rstest]
	fn test_validate_with_country_detection() {
		let validator = PostalCodeValidator::new();

		// US ZIP+4 format (unambiguous)
		assert_eq!(
			validator.validate_with_country("12345-6789").unwrap(),
			Country::US
		);
		// Japan format (unambiguous - has hyphen)
		assert_eq!(
			validator.validate_with_country("123-4567").unwrap(),
			Country::JP
		);
		// UK format (unambiguous - alphanumeric with space)
		assert_eq!(
			validator.validate_with_country("SW1A 1AA").unwrap(),
			Country::UK
		);
		// Canada format (unambiguous - alphanumeric)
		assert_eq!(
			validator.validate_with_country("K1A 0B1").unwrap(),
			Country::CA
		);
		// 5-digit numbers are ambiguous (could be US or DE)
		// When using with_countries(), users should restrict to avoid ambiguity
		// Here we test that it matches one of them
		let result = validator.validate_with_country("12345").unwrap();
		assert!(result == Country::US || result == Country::DE);
	}

	#[rstest]
	fn test_validate_with_country_restriction() {
		let validator = PostalCodeValidator::with_countries(vec![Country::US, Country::JP]);

		// Allowed countries
		assert!(validator.validate_with_country("12345").is_ok());
		assert!(validator.validate_with_country("123-4567").is_ok());

		// Not allowed country
		match validator.validate_with_country("SW1A 1AA") {
			Err(ValidationError::PostalCodeCountryNotAllowed { country, .. }) => {
				assert_eq!(country, "UK");
			}
			_ => panic!("Expected PostalCodeCountryNotAllowed error"),
		}
	}

	#[rstest]
	fn test_invalid_postal_code() {
		let validator = PostalCodeValidator::new();

		match validator.validate_with_country("invalid") {
			Err(ValidationError::PostalCodeCountryNotRecognized { postal_code }) => {
				assert_eq!(postal_code, "INVALID");
			}
			_ => panic!("Expected PostalCodeCountryNotRecognized error"),
		}
	}

	// Case insensitivity tests
	#[rstest]
	fn test_case_insensitive() {
		let validator = PostalCodeValidator::new();

		// Lowercase should work
		assert!(validator.validate("sw1a 1aa").is_ok()); // UK
		assert!(validator.validate("k1a 0b1").is_ok()); // CA
	}

	// Whitespace trimming tests
	#[rstest]
	fn test_whitespace_trimming() {
		let validator = PostalCodeValidator::new();

		assert!(validator.validate("  12345  ").is_ok()); // US with spaces
		assert!(validator.validate(" SW1A 1AA ").is_ok()); // UK with spaces
	}
}
