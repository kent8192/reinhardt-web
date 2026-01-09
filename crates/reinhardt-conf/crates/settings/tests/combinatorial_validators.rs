//! Combinatorial Validator Tests for reinhardt-settings.
//!
//! This test module validates combinations of validators to ensure they work
//! correctly when composed together.
//!
//! ## Test Categories
//!
//! 1. **Single Validator Tests**: Baseline behavior of individual validators
//! 2. **Two Validator Combinations**: RequiredValidator + RangeValidator
//! 3. **Complex Combinations**: SecurityValidator + RequiredValidator
//! 4. **Conflicting Rules**: Multiple validators with overlapping or conflicting requirements

use reinhardt_settings::prelude::{SettingsValidator, Validator};
use reinhardt_settings::profile::Profile;
use reinhardt_settings::validation::{
	RangeValidator, RequiredValidator, SecurityValidator, ValidationError,
};
use rstest::*;
use serde_json::json;
use std::collections::HashMap;

/// Test: RequiredValidator alone
///
/// Why: Validates baseline behavior of RequiredValidator checking for field presence.
/// This establishes the foundation before testing combinations.
#[rstest]
#[case(vec!["secret_key"], json!({"secret_key": "value"}), true, "Required field present → PASS")]
#[case(vec!["secret_key"], json!({}), false, "Required field missing → FAIL")]
#[case(vec!["secret_key", "debug"], json!({"secret_key": "value", "debug": true}), true, "All required fields present → PASS")]
#[case(vec!["secret_key", "debug"], json!({"secret_key": "value"}), false, "One required field missing → FAIL")]
#[case(vec!["secret_key", "debug"], json!({}), false, "All required fields missing → FAIL")]
fn test_required_validator_alone(
	#[case] required_fields: Vec<&str>,
	#[case] settings_json: serde_json::Value,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	let validator = RequiredValidator::new(
		required_fields
			.iter()
			.map(|s| s.to_string())
			.collect::<Vec<String>>(),
	);

	let settings_map: HashMap<String, serde_json::Value> = settings_json
		.as_object()
		.expect("Should be object")
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let result = validator.validate_settings(&settings_map);

	assert_eq!(
		result.is_ok(),
		should_pass,
		"RequiredValidator test failed: {} - Result={:?}",
		description,
		result
	);
}

/// Test: RangeValidator alone (via Settings field validation)
///
/// Why: Validates baseline behavior of RangeValidator checking numeric value ranges.
/// Note: RangeValidator is typically used for individual field validation, not full settings.
#[rstest]
#[case(10.0, 0.0, 100.0, true, "Value within range → PASS")]
#[case(0.0, 0.0, 100.0, true, "Value at minimum → PASS")]
#[case(100.0, 0.0, 100.0, true, "Value at maximum → PASS")]
#[case(-1.0, 0.0, 100.0, false, "Value below minimum → FAIL")]
#[case(101.0, 0.0, 100.0, false, "Value above maximum → FAIL")]
fn test_range_validator_alone(
	#[case] value: f64,
	#[case] min: f64,
	#[case] max: f64,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	let validator = RangeValidator::between(min, max);
	let value_json = json!(value);

	let result = validator.validate("test_field", &value_json);

	assert_eq!(
		result.is_ok(),
		should_pass,
		"RangeValidator test failed: {} - Value={}, Range=[{}, {}], Result={:?}",
		description,
		value,
		min,
		max,
		result
	);
}

/// Test: RequiredValidator + RangeValidator combination
///
/// Why: Validates that field presence validation (RequiredValidator) and value range
/// validation (RangeValidator) work together correctly. The field must exist AND
/// its value must be within the specified range.
///
/// Combination Logic:
/// 1. RequiredValidator checks field existence
/// 2. If field exists, RangeValidator checks its value
/// 3. Both must pass for overall validation to succeed
#[rstest]
#[case(
	vec!["timeout"],
	json!({"timeout": 50.0}),
	0.0,
	100.0,
	true,
	"Required field present + value in range → PASS"
)]
#[case(
	vec!["timeout"],
	json!({"timeout": 150.0}),
	0.0,
	100.0,
	false,
	"Required field present + value out of range → FAIL (range)"
)]
#[case(
	vec!["timeout"],
	json!({}),
	0.0,
	100.0,
	false,
	"Required field missing → FAIL (required)"
)]
#[case(
	vec!["timeout", "max_connections"],
	json!({"timeout": 50.0, "max_connections": 10.0}),
	0.0,
	100.0,
	true,
	"Multiple required fields + all values in range → PASS"
)]
#[case(
	vec!["timeout", "max_connections"],
	json!({"timeout": 50.0, "max_connections": 150.0}),
	0.0,
	100.0,
	false,
	"Multiple required fields + one value out of range → FAIL (range)"
)]
#[case(
	vec!["timeout", "max_connections"],
	json!({"timeout": 50.0}),
	0.0,
	100.0,
	false,
	"Multiple required fields + one missing → FAIL (required)"
)]
fn test_required_and_range_combination(
	#[case] required_fields: Vec<&str>,
	#[case] settings_json: serde_json::Value,
	#[case] range_min: f64,
	#[case] range_max: f64,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	// Step 1: Check required fields
	let required_validator = RequiredValidator::new(
		required_fields
			.iter()
			.map(|s| s.to_string())
			.collect::<Vec<String>>(),
	);

	let settings_map: HashMap<String, serde_json::Value> = settings_json
		.as_object()
		.expect("Should be object")
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let required_result = required_validator.validate_settings(&settings_map);

	// Step 2: If required validation passed, check ranges
	let combined_result = if required_result.is_ok() {
		let range_validator = RangeValidator::between(range_min, range_max);

		// Check all required fields have values in range
		let mut range_errors = Vec::new();
		for field in &required_fields {
			if let Some(value) = settings_map.get(*field) {
				if let Err(e) = range_validator.validate(field, value) {
					range_errors.push(e);
				}
			}
		}

		if range_errors.is_empty() {
			Ok(())
		} else if range_errors.len() == 1 {
			Err(range_errors.into_iter().next().unwrap())
		} else {
			Err(ValidationError::Multiple(range_errors))
		}
	} else {
		required_result
	};

	assert_eq!(
		combined_result.is_ok(),
		should_pass,
		"RequiredValidator + RangeValidator combination failed: {} - Result={:?}",
		description,
		combined_result
	);
}

/// Test: SecurityValidator + RequiredValidator combination
///
/// Why: Validates that security requirements (SecurityValidator) and required field
/// checks (RequiredValidator) work together in production environments.
///
/// Combination Logic (Production only):
/// 1. RequiredValidator checks all required fields exist
/// 2. SecurityValidator enforces production security rules (debug=false, strong secret_key, etc.)
/// 3. Both must pass for production deployment
///
/// Note: SecurityValidator only enforces rules in Production profile.
#[rstest]
#[case(
	Profile::Production,
	vec!["secret_key", "allowed_hosts"],
	json!({
		"debug": false,
		"secret_key": "production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"allowed_hosts": ["example.com"],
		"secure_ssl_redirect": true
	}),
	true,
	"Prod: All required fields + security rules met → PASS"
)]
#[case(
	Profile::Production,
	vec!["secret_key", "allowed_hosts"],
	json!({
		"debug": false,
		"secret_key": "production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"secure_ssl_redirect": true
	}),
	false,
	"Prod: Missing required field → FAIL (required)"
)]
#[case(
	Profile::Production,
	vec!["secret_key", "allowed_hosts"],
	json!({
		"debug": true,
		"secret_key": "production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"allowed_hosts": ["example.com"],
		"secure_ssl_redirect": true
	}),
	false,
	"Prod: All required fields but debug=true → FAIL (security)"
)]
#[case(
	Profile::Production,
	vec!["secret_key", "allowed_hosts"],
	json!({
		"debug": false,
		"secret_key": "weak",
		"allowed_hosts": ["example.com"],
		"secure_ssl_redirect": true
	}),
	false,
	"Prod: All required fields but weak secret_key → FAIL (security)"
)]
#[case(
	Profile::Development,
	vec!["secret_key"],
	json!({
		"debug": true,
		"secret_key": "dev_secret"
	}),
	true,
	"Dev: Required field present, security lenient → PASS"
)]
#[case(
	Profile::Development,
	vec!["secret_key"],
	json!({
		"debug": true
	}),
	false,
	"Dev: Missing required field → FAIL (required)"
)]
fn test_security_and_required_combination(
	#[case] profile: Profile,
	#[case] required_fields: Vec<&str>,
	#[case] settings_json: serde_json::Value,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	// Step 1: Check required fields
	let required_validator = RequiredValidator::new(
		required_fields
			.iter()
			.map(|s| s.to_string())
			.collect::<Vec<String>>(),
	);

	let settings_map: HashMap<String, serde_json::Value> = settings_json
		.as_object()
		.expect("Should be object")
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let required_result = required_validator.validate_settings(&settings_map);

	// Step 2: If required validation passed, check security
	let combined_result = if required_result.is_ok() {
		let security_validator = SecurityValidator::new(profile);
		security_validator.validate_settings(&settings_map)
	} else {
		required_result
	};

	assert_eq!(
		combined_result.is_ok(),
		should_pass,
		"SecurityValidator + RequiredValidator combination failed: {} - Profile={:?}, Result={:?}",
		description,
		profile,
		combined_result
	);
}

/// Test: Triple combination - SecurityValidator + RequiredValidator + RangeValidator
///
/// Why: Validates that all three validator types can work together for comprehensive
/// configuration validation. This represents a realistic production scenario where
/// we need to ensure fields exist, have valid values, and meet security requirements.
///
/// Combination Logic:
/// 1. RequiredValidator checks field existence
/// 2. RangeValidator checks numeric value ranges
/// 3. SecurityValidator enforces production security rules
/// 4. All three must pass for complete validation
#[rstest]
#[case(
	Profile::Production,
	vec!["timeout", "secret_key", "allowed_hosts"],
	json!({
		"debug": false,
		"secret_key": "production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"allowed_hosts": ["example.com"],
		"secure_ssl_redirect": true,
		"timeout": 50.0
	}),
	0.0,
	100.0,
	true,
	"Prod: All validators pass → PASS"
)]
#[case(
	Profile::Production,
	vec!["timeout", "secret_key", "allowed_hosts"],
	json!({
		"debug": false,
		"secret_key": "production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"allowed_hosts": ["example.com"],
		"secure_ssl_redirect": true,
		"timeout": 150.0
	}),
	0.0,
	100.0,
	false,
	"Prod: Value out of range → FAIL (range)"
)]
#[case(
	Profile::Production,
	vec!["timeout", "secret_key", "allowed_hosts"],
	json!({
		"debug": false,
		"secret_key": "production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"allowed_hosts": ["example.com"],
		"secure_ssl_redirect": true
	}),
	0.0,
	100.0,
	false,
	"Prod: Missing required field → FAIL (required)"
)]
#[case(
	Profile::Production,
	vec!["timeout", "secret_key", "allowed_hosts"],
	json!({
		"debug": true,
		"secret_key": "production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"allowed_hosts": ["example.com"],
		"secure_ssl_redirect": true,
		"timeout": 50.0
	}),
	0.0,
	100.0,
	false,
	"Prod: Security rule violated → FAIL (security)"
)]
fn test_triple_validator_combination(
	#[case] profile: Profile,
	#[case] required_fields: Vec<&str>,
	#[case] settings_json: serde_json::Value,
	#[case] range_min: f64,
	#[case] range_max: f64,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	let settings_map: HashMap<String, serde_json::Value> = settings_json
		.as_object()
		.expect("Should be object")
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	// Step 1: Check required fields
	let required_validator = RequiredValidator::new(
		required_fields
			.iter()
			.map(|s| s.to_string())
			.collect::<Vec<String>>(),
	);
	let required_result = required_validator.validate_settings(&settings_map);

	// Step 2: If required validation passed, check ranges
	let range_result = if required_result.is_ok() {
		let range_validator = RangeValidator::between(range_min, range_max);

		// Check numeric fields have values in range
		let mut range_errors = Vec::new();
		for field in &required_fields {
			if let Some(value) = settings_map.get(*field) {
				// Only validate numeric fields
				if value.is_f64() || value.is_i64() || value.is_u64() {
					if let Err(e) = range_validator.validate(field, value) {
						range_errors.push(e);
					}
				}
			}
		}

		if range_errors.is_empty() {
			Ok(())
		} else if range_errors.len() == 1 {
			Err(range_errors.into_iter().next().unwrap())
		} else {
			Err(ValidationError::Multiple(range_errors))
		}
	} else {
		required_result
	};

	// Step 3: If both required and range validation passed, check security
	let combined_result = if range_result.is_ok() {
		let security_validator = SecurityValidator::new(profile);
		security_validator.validate_settings(&settings_map)
	} else {
		range_result
	};

	assert_eq!(
		combined_result.is_ok(),
		should_pass,
		"Triple validator combination failed: {} - Profile={:?}, Result={:?}",
		description,
		profile,
		combined_result
	);
}
