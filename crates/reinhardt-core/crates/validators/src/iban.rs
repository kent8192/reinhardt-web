//! IBAN (International Bank Account Number) validator
//!
//! This module provides validation for IBAN according to ISO 13616 standard.
//! IBAN validation includes:
//! - Format validation (country code, check digits, BBAN)
//! - MOD-97 checksum validation
//! - Country-specific length validation
//! - Optional country code filtering

use crate::{ValidationError, ValidationResult, Validator};
use std::collections::HashMap;

/// IBAN validator implementing ISO 13616 standard
///
/// The validator checks:
/// 1. Basic format (alphanumeric, uppercase)
/// 2. Country code validity and IBAN length for that country
/// 3. MOD-97 checksum algorithm
/// 4. Optional country filtering
///
/// # Examples
///
/// ```
/// use reinhardt_validators::{IBANValidator, Validator};
///
/// // Basic validation
/// let validator = IBANValidator::new();
/// assert!(validator.validate("DE89370400440532013000").is_ok());
///
/// // With country filtering
/// let validator = IBANValidator::with_countries(vec!["DE".to_string(), "FR".to_string()]);
/// assert!(validator.validate("DE89370400440532013000").is_ok());
/// assert!(validator.validate("GB82WEST12345698765432").is_err());
/// ```
pub struct IBANValidator {
	/// Optional list of allowed country codes (ISO 3166-1 alpha-2)
	pub country_codes: Option<Vec<String>>,
	country_lengths: HashMap<String, usize>,
}

impl IBANValidator {
	/// Creates a new IBAN validator with default settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::IBANValidator;
	///
	/// let validator = IBANValidator::new();
	/// ```
	pub fn new() -> Self {
		Self {
			country_codes: None,
			country_lengths: Self::init_country_lengths(),
		}
	}

	/// Creates a new IBAN validator that only accepts specified country codes
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::IBANValidator;
	///
	/// let validator = IBANValidator::with_countries(vec!["DE".to_string(), "FR".to_string()]);
	/// ```
	pub fn with_countries(codes: Vec<String>) -> Self {
		Self {
			country_codes: Some(codes.iter().map(|c| c.to_uppercase()).collect()),
			country_lengths: Self::init_country_lengths(),
		}
	}

	/// Initializes the mapping of country codes to IBAN lengths
	///
	/// Based on ISO 13616 standard as of 2024
	fn init_country_lengths() -> HashMap<String, usize> {
		let mut map = HashMap::new();

		// European countries
		map.insert("AD".to_string(), 24); // Andorra
		map.insert("AT".to_string(), 20); // Austria
		map.insert("BE".to_string(), 16); // Belgium
		map.insert("BG".to_string(), 22); // Bulgaria
		map.insert("CH".to_string(), 21); // Switzerland
		map.insert("CY".to_string(), 28); // Cyprus
		map.insert("CZ".to_string(), 24); // Czech Republic
		map.insert("DE".to_string(), 22); // Germany
		map.insert("DK".to_string(), 18); // Denmark
		map.insert("EE".to_string(), 20); // Estonia
		map.insert("ES".to_string(), 24); // Spain
		map.insert("FI".to_string(), 18); // Finland
		map.insert("FR".to_string(), 27); // France
		map.insert("GB".to_string(), 22); // United Kingdom
		map.insert("GI".to_string(), 23); // Gibraltar
		map.insert("GR".to_string(), 27); // Greece
		map.insert("HR".to_string(), 21); // Croatia
		map.insert("HU".to_string(), 28); // Hungary
		map.insert("IE".to_string(), 22); // Ireland
		map.insert("IS".to_string(), 26); // Iceland
		map.insert("IT".to_string(), 27); // Italy
		map.insert("LI".to_string(), 21); // Liechtenstein
		map.insert("LT".to_string(), 20); // Lithuania
		map.insert("LU".to_string(), 20); // Luxembourg
		map.insert("LV".to_string(), 21); // Latvia
		map.insert("MC".to_string(), 27); // Monaco
		map.insert("MT".to_string(), 31); // Malta
		map.insert("NL".to_string(), 18); // Netherlands
		map.insert("NO".to_string(), 15); // Norway
		map.insert("PL".to_string(), 28); // Poland
		map.insert("PT".to_string(), 25); // Portugal
		map.insert("RO".to_string(), 24); // Romania
		map.insert("SE".to_string(), 24); // Sweden
		map.insert("SI".to_string(), 19); // Slovenia
		map.insert("SK".to_string(), 24); // Slovakia
		map.insert("SM".to_string(), 27); // San Marino
		map.insert("VA".to_string(), 22); // Vatican City

		// Middle East and North Africa
		map.insert("AE".to_string(), 23); // United Arab Emirates
		map.insert("BH".to_string(), 22); // Bahrain
		map.insert("IL".to_string(), 23); // Israel
		map.insert("JO".to_string(), 30); // Jordan
		map.insert("KW".to_string(), 30); // Kuwait
		map.insert("LB".to_string(), 28); // Lebanon
		map.insert("PS".to_string(), 29); // Palestine
		map.insert("QA".to_string(), 29); // Qatar
		map.insert("SA".to_string(), 24); // Saudi Arabia
		map.insert("TR".to_string(), 26); // Turkey

		// Other countries
		map.insert("AZ".to_string(), 28); // Azerbaijan
		map.insert("BR".to_string(), 29); // Brazil
		map.insert("CR".to_string(), 22); // Costa Rica
		map.insert("DO".to_string(), 28); // Dominican Republic
		map.insert("GE".to_string(), 22); // Georgia
		map.insert("GT".to_string(), 28); // Guatemala
		map.insert("KZ".to_string(), 20); // Kazakhstan
		map.insert("MD".to_string(), 24); // Moldova
		map.insert("MK".to_string(), 19); // North Macedonia
		map.insert("MR".to_string(), 27); // Mauritania
		map.insert("MU".to_string(), 30); // Mauritius
		map.insert("PK".to_string(), 24); // Pakistan
		map.insert("RS".to_string(), 22); // Serbia
		map.insert("TN".to_string(), 24); // Tunisia
		map.insert("UA".to_string(), 29); // Ukraine
		map.insert("XK".to_string(), 20); // Kosovo

		map
	}

	/// Validates IBAN format and structure
	///
	/// Returns Ok if valid, Err with ValidationError if invalid
	fn validate_format(&self, iban: &str) -> ValidationResult<String> {
		// Remove spaces and convert to uppercase
		let iban = iban.replace(' ', "").to_uppercase();

		// Check minimum length (15 is the shortest IBAN - Norway)
		if iban.len() < 15 {
			return Err(ValidationError::InvalidIBAN(
				"IBAN is too short".to_string(),
			));
		}

		// Check maximum length (31 is the longest IBAN - Malta)
		if iban.len() > 31 {
			return Err(ValidationError::InvalidIBAN("IBAN is too long".to_string()));
		}

		// Check if all characters are alphanumeric
		if !iban.chars().all(|c| c.is_ascii_alphanumeric()) {
			return Err(ValidationError::InvalidIBAN(
				"IBAN contains invalid characters".to_string(),
			));
		}

		// Extract country code (first 2 characters)
		let country_code = &iban[0..2];

		// Check if country code consists of letters only
		if !country_code.chars().all(|c| c.is_ascii_alphabetic()) {
			return Err(ValidationError::InvalidIBAN(
				"Invalid country code".to_string(),
			));
		}

		// Check if country code is in allowed list (if specified)
		if let Some(allowed) = &self.country_codes
			&& !allowed.contains(&country_code.to_string()) {
				return Err(ValidationError::IBANCountryNotAllowed {
					country_code: country_code.to_string(),
					allowed_codes: allowed.join(", "),
				});
			}

		// Check country-specific length
		if let Some(&expected_length) = self.country_lengths.get(country_code) {
			if iban.len() != expected_length {
				return Err(ValidationError::InvalidIBAN(format!(
					"Invalid IBAN length for country '{}': expected {}, got {}",
					country_code,
					expected_length,
					iban.len()
				)));
			}
		} else {
			return Err(ValidationError::InvalidIBAN(format!(
				"Unknown country code: {}",
				country_code
			)));
		}

		// Extract check digits (characters 3-4)
		let check_digits = &iban[2..4];

		// Check if check digits are numeric
		if !check_digits.chars().all(|c| c.is_ascii_digit()) {
			return Err(ValidationError::InvalidIBAN(
				"Check digits must be numeric".to_string(),
			));
		}

		Ok(iban)
	}

	/// Performs MOD-97 checksum validation according to ISO 13616
	///
	/// Algorithm:
	/// 1. Move the first 4 characters to the end
	/// 2. Replace each letter with its numeric value (A=10, B=11, ..., Z=35)
	/// 3. Calculate the remainder when divided by 97
	/// 4. Valid IBAN has remainder of 1
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_validators::IBANValidator;
	///
	/// let validator = IBANValidator::new();
	/// // This is tested internally by the validate method
	/// ```
	fn mod97_check(iban: &str) -> bool {
		let iban = iban.replace(' ', "").to_uppercase();

		// Move first 4 characters to the end
		let rearranged = format!("{}{}", &iban[4..], &iban[0..4]);

		// Convert letters to numbers (A=10, B=11, ..., Z=35)
		let numeric_string: String = rearranged
			.chars()
			.map(|c| {
				if c.is_ascii_digit() {
					c.to_string()
				} else {
					// A=10, B=11, ..., Z=35
					(c as u32 - 'A' as u32 + 10).to_string()
				}
			})
			.collect();

		// Calculate MOD-97
		// We need to handle very large numbers, so we process in chunks
		let mut remainder: u32 = 0;

		for chunk in numeric_string.chars() {
			let digit = chunk.to_digit(10).unwrap();
			remainder = (remainder * 10 + digit) % 97;
		}

		remainder == 1
	}
}

impl Default for IBANValidator {
	fn default() -> Self {
		Self::new()
	}
}

impl Validator<String> for IBANValidator {
	fn validate(&self, value: &String) -> ValidationResult<()> {
		// Validate format and get normalized IBAN
		let iban = self.validate_format(value)?;

		// Perform MOD-97 check
		if !Self::mod97_check(&iban) {
			return Err(ValidationError::InvalidIBAN(
				"Invalid IBAN checksum".to_string(),
			));
		}

		Ok(())
	}
}

impl Validator<str> for IBANValidator {
	fn validate(&self, value: &str) -> ValidationResult<()> {
		// Validate format and get normalized IBAN
		let iban = self.validate_format(value)?;

		// Perform MOD-97 check
		if !Self::mod97_check(&iban) {
			return Err(ValidationError::InvalidIBAN(
				"Invalid IBAN checksum".to_string(),
			));
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// Valid IBAN test cases from various countries
	#[test]
	fn test_valid_ibans() {
		let validator = IBANValidator::new();

		let valid_ibans = vec![
			"DE89370400440532013000",      // Germany
			"GB82WEST12345698765432",      // United Kingdom
			"FR1420041010050500013M02606", // France
			"IT60X0542811101000000123456", // Italy
			"ES9121000418450200051332",    // Spain
			"NL91ABNA0417164300",          // Netherlands
			"BE68539007547034",            // Belgium
			"CH9300762011623852957",       // Switzerland
			"AT611904300234573201",        // Austria
			"NO9386011117947",             // Norway (shortest IBAN)
		];

		for iban in valid_ibans {
			assert!(
				validator.validate(iban).is_ok(),
				"Expected {} to be valid",
				iban
			);
		}
	}

	// Test IBANs with spaces (should be handled correctly)
	#[test]
	fn test_ibans_with_spaces() {
		let validator = IBANValidator::new();

		// IBANs are often written with spaces for readability
		assert!(validator.validate("DE89 3704 0044 0532 0130 00").is_ok());
		assert!(validator.validate("GB82 WEST 1234 5698 7654 32").is_ok());
		assert!(
			validator
				.validate("FR14 2004 1010 0505 0001 3M02 606")
				.is_ok()
		);
	}

	#[test]
	fn test_invalid_ibans() {
		let validator = IBANValidator::new();

		let invalid_ibans = vec![
			"DE89370400440532013001",            // Wrong check digit
			"GB82WEST12345698765433",            // Wrong check digit
			"XX1234567890",                      // Unknown country code
			"DE8937040044053201300",             // Too short for Germany
			"DE893704004405320130000",           // Too long for Germany
			"12345678901234567890",              // No country code
			"DEAA370400440532013000",            // Invalid check digits (should be numeric)
			"",                                  // Empty string
			"DE",                                // Too short
			"DE89 3704 0044 0532 0130 00 EXTRA", // Too long with extra characters
		];

		for iban in invalid_ibans {
			assert!(
				validator.validate(iban).is_err(),
				"Expected {} to be invalid",
				iban
			);
		}
	}

	#[test]
	fn test_mod97_algorithm() {
		// Test the MOD-97 algorithm directly
		assert!(IBANValidator::mod97_check("DE89370400440532013000"));
		assert!(IBANValidator::mod97_check("GB82WEST12345698765432"));
		assert!(IBANValidator::mod97_check("NO9386011117947"));

		// Invalid checksums
		assert!(!IBANValidator::mod97_check("DE89370400440532013001"));
		assert!(!IBANValidator::mod97_check("GB82WEST12345698765433"));
	}

	#[test]
	fn test_allowed_countries() {
		let validator = IBANValidator::with_countries(vec!["DE".to_string(), "FR".to_string()]);

		// German IBAN should be accepted
		assert!(validator.validate("DE89370400440532013000").is_ok());

		// French IBAN should be accepted
		assert!(validator.validate("FR1420041010050500013M02606").is_ok());

		// British IBAN should be rejected (not in allowed list)
		let result = validator.validate("GB82WEST12345698765432");
		assert!(result.is_err());
		match result {
			Err(ValidationError::IBANCountryNotAllowed {
				country_code,
				allowed_codes,
			}) => {
				assert_eq!(country_code, "GB");
				assert!(allowed_codes.contains("DE"));
				assert!(allowed_codes.contains("FR"));
			}
			_ => panic!("Expected IBANCountryNotAllowed error"),
		}
	}

	#[test]
	fn test_country_code_validation() {
		let validator = IBANValidator::new();

		// Invalid country codes
		assert!(validator.validate("12345678901234567890123456").is_err()); // No letters
		assert!(validator.validate("A1234567890123456789012345").is_err()); // Single letter country code
		assert!(validator.validate("1A234567890123456789012345").is_err()); // Number in country code
	}

	#[test]
	fn test_check_digit_validation() {
		let validator = IBANValidator::new();

		// Check digits must be numeric
		assert!(validator.validate("DEAA370400440532013000").is_err());
		assert!(validator.validate("DE8A370400440532013000").is_err());
		assert!(validator.validate("DEA9370400440532013000").is_err());
	}

	#[test]
	fn test_case_insensitivity() {
		let validator = IBANValidator::new();

		// Lowercase should work (will be converted to uppercase internally)
		assert!(validator.validate("de89370400440532013000").is_ok());
		assert!(validator.validate("gb82west12345698765432").is_ok());

		// Mixed case
		assert!(validator.validate("De89370400440532013000").is_ok());
		assert!(validator.validate("GB82west12345698765432").is_ok());
	}

	#[test]
	fn test_specific_country_lengths() {
		let validator = IBANValidator::new();

		// Norway has the shortest IBAN (15 characters)
		assert!(validator.validate("NO9386011117947").is_ok());

		// Malta has the longest IBAN (31 characters)
		// Note: This is a fabricated example for testing length
		// Real Maltese IBANs would need proper check digits
		let maltese_length_test = "MT84MALT011000012345MTLCAST001S";
		// We expect this to fail on checksum, not length
		assert!(validator.validate_format(maltese_length_test).is_ok());
	}

	#[test]
	fn test_invalid_characters() {
		let validator = IBANValidator::new();

		assert!(validator.validate("DE89-3704-0044-0532-0130-00").is_err()); // Hyphens
		assert!(validator.validate("DE89.3704.0044.0532.0130.00").is_err()); // Dots
		assert!(validator.validate("DE89_3704_0044_0532_0130_00").is_err()); // Underscores
		assert!(validator.validate("DE89/3704/0044/0532/0130/00").is_err()); // Slashes
	}

	#[test]
	fn test_string_type_validation() {
		let validator = IBANValidator::new();

		let iban = String::from("DE89370400440532013000");
		assert!(validator.validate(&iban).is_ok());

		let invalid = String::from("INVALID");
		assert!(validator.validate(&invalid).is_err());
	}

	#[test]
	fn test_with_countries_builder() {
		let validator = IBANValidator::with_countries(vec!["DE".to_string()]);

		assert!(validator.validate("DE89370400440532013000").is_ok());
		assert!(validator.validate("GB82WEST12345698765432").is_err());
	}

	// Additional edge cases
	#[test]
	fn test_numeric_string_conversion_in_mod97() {
		// This tests that the MOD-97 algorithm correctly handles
		// the conversion of letters to numbers
		// For DE89370400440532013000:
		// Rearranged: 370400440532013000DE89
		// Converted: 3704004405320130001314 (D=13, E=14)
		assert!(IBANValidator::mod97_check("DE89370400440532013000"));
	}

	#[test]
	fn test_all_supported_countries() {
		let validator = IBANValidator::new();

		// Verify that all countries in the map are accessible
		assert_eq!(validator.country_lengths.len(), 63); // Total number of supported countries

		// Spot check a few countries
		assert_eq!(validator.country_lengths.get("DE"), Some(&22));
		assert_eq!(validator.country_lengths.get("GB"), Some(&22));
		assert_eq!(validator.country_lengths.get("FR"), Some(&27));
		assert_eq!(validator.country_lengths.get("NO"), Some(&15));
		assert_eq!(validator.country_lengths.get("MT"), Some(&31));
	}
}
