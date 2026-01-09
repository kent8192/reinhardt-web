//! Decision Table Tests for reinhardt-settings.
//!
//! This test module uses decision tables to systematically validate complex
//! conditional logic in validators and configuration source priority resolution.
//!
//! ## Decision Tables
//!
//! ### Security Validator Decision Table
//!
//! | Profile      | Debug | Secret Key | Expected Result |
//! |--------------|-------|------------|-----------------|
//! | Development  | true  | ""         | PASS            |
//! | Development  | false | ""         | PASS            |
//! | Production   | true  | ""         | FAIL            |
//! | Production   | false | ""         | FAIL            |
//! | Production   | false | "valid"    | PASS            |
//! | Staging      | true  | "valid"    | PASS            |
//!
//! ### Source Priority Decision Table
//!
//! | Source 1 | Source 2 | Source 3 | Expected Winner |
//! |----------|----------|----------|-----------------|
//! | 100      | -        | -        | Source 1        |
//! | 100      | 50       | -        | Source 2        |
//! | 100      | 50       | 0        | Source 3        |
//! | -        | 50       | 0        | Source 3        |

use reinhardt_settings::Settings;
use reinhardt_settings::builder::SettingsBuilder;
use reinhardt_settings::prelude::SettingsValidator;
use reinhardt_settings::profile::Profile;
use reinhardt_settings::sources::DefaultSource;
use reinhardt_settings::validation::SecurityValidator as SecurityValidatorImpl;
use rstest::*;
use serde_json::json;
use std::collections::HashMap;

/// Test: Security Validator Decision Table
///
/// Why: Validates that SecurityValidator enforces production security rules correctly
/// based on combinations of profile, debug mode, and secret_key presence.
#[rstest]
#[case(
	Profile::Development,
	true,
	"",
	true,
	"Dev: debug=true, empty secret_key → PASS"
)]
#[case(
	Profile::Development,
	false,
	"",
	true,
	"Dev: debug=false, empty secret_key → PASS"
)]
#[case(
	Profile::Development,
	true,
	"secret",
	true,
	"Dev: debug=true, valid secret_key → PASS"
)]
#[case(
	Profile::Development,
	false,
	"secret",
	true,
	"Dev: debug=false, valid secret_key → PASS"
)]
#[case(
	Profile::Staging,
	true,
	"",
	true,
	"Staging: debug=true, empty secret_key → PASS (lenient)"
)]
#[case(
	Profile::Staging,
	false,
	"",
	true,
	"Staging: debug=false, empty secret_key → PASS (lenient)"
)]
#[case(
	Profile::Staging,
	true,
	"secret",
	true,
	"Staging: debug=true, valid secret_key → PASS"
)]
#[case(
	Profile::Staging,
	false,
	"secret",
	true,
	"Staging: debug=false, valid secret_key → PASS"
)]
#[case(
	Profile::Production,
	true,
	"",
	false,
	"Prod: debug=true, empty secret_key → FAIL"
)]
#[case(
	Profile::Production,
	false,
	"",
	false,
	"Prod: debug=false, empty secret_key → FAIL"
)]
#[case(
	Profile::Production,
	true,
	"production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
	false,
	"Prod: debug=true, valid secret_key → FAIL"
)]
#[case(
	Profile::Production,
	false,
	"production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
	true,
	"Prod: debug=false, valid secret_key → PASS"
)]
fn test_security_validator_decision_table(
	#[case] profile: Profile,
	#[case] debug: bool,
	#[case] secret_key: &str,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	let mut settings = Settings::default();
	settings.debug = debug;
	settings.secret_key = secret_key.to_string();

	// Production requires security settings
	if matches!(profile, Profile::Production) {
		settings.allowed_hosts = vec!["example.com".to_string()];
		settings.secure_ssl_redirect = true;
	}

	let validator = SecurityValidatorImpl::new(profile);
	let settings_value = serde_json::to_value(&settings).expect("Serialize should succeed");
	let settings_obj = settings_value.as_object().expect("Should be object");
	let settings_map: HashMap<String, serde_json::Value> = settings_obj
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let result = validator.validate_settings(&settings_map);

	assert_eq!(
		result.is_ok(),
		should_pass,
		"Decision table case failed: {} - Debug={}, SecretKey={:?}, Result={:?}",
		description,
		debug,
		secret_key,
		result
	);
}

/// Test: Source Priority Decision Table (Last Source Wins)
///
/// Why: Validates that the "last source wins" priority rule is applied correctly
/// across different combinations of source presence.
#[rstest]
#[case(Some(100), None, None, 100, "Source 1 only")]
#[case(Some(100), Some(50), None, 50, "Source 1 + Source 2 → Source 2 wins")]
#[case(Some(100), Some(50), Some(0), 0, "All three sources → Source 3 wins")]
#[case(None, Some(50), Some(0), 0, "Source 2 + Source 3 → Source 3 wins")]
#[case(None, None, Some(0), 0, "Source 3 only")]
fn test_source_priority_decision_table(
	#[case] source1_value: Option<i32>,
	#[case] source2_value: Option<i32>,
	#[case] source3_value: Option<i32>,
	#[case] expected_value: i32,
	#[case] description: &str,
) {
	let key = "test_key";
	let mut builder = SettingsBuilder::new();

	if let Some(val) = source1_value {
		let source = DefaultSource::default().with_value(key, json!(val));
		builder = builder.add_source(source);
	}

	if let Some(val) = source2_value {
		let source = DefaultSource::default().with_value(key, json!(val));
		builder = builder.add_source(source);
	}

	if let Some(val) = source3_value {
		let source = DefaultSource::default().with_value(key, json!(val));
		builder = builder.add_source(source);
	}

	let merged = builder.build().expect("Build should succeed");
	let result_value = merged.get::<i32>(key).expect("Get should succeed");

	assert_eq!(
		result_value, expected_value,
		"Decision table case failed: {}",
		description
	);
}

/// Test: Profile + Secret Key Decision Table (Comprehensive)
///
/// Why: Validates all combinations of profile detection and secret key validation.
#[rstest]
#[case(Profile::Development, "", true, "Dev with empty secret → PASS")]
#[case(Profile::Development, "dev_secret", true, "Dev with secret → PASS")]
#[case(Profile::Staging, "", true, "Staging with empty secret → PASS")]
#[case(Profile::Staging, "staging_secret", true, "Staging with secret → PASS")]
#[case(Profile::Production, "", false, "Prod with empty secret → FAIL")]
#[case(
	Profile::Production,
	"production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
	true,
	"Prod with secret → PASS"
)]
fn test_profile_secret_key_decision_table(
	#[case] profile: Profile,
	#[case] secret_key: &str,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	let mut settings = Settings::default();
	settings.secret_key = secret_key.to_string();
	settings.debug = false; // Ensure debug is off for production checks

	// Production requires security settings
	if matches!(profile, Profile::Production) {
		settings.allowed_hosts = vec!["example.com".to_string()];
		settings.secure_ssl_redirect = true;
	}

	let validator = SecurityValidatorImpl::new(profile);
	let settings_value = serde_json::to_value(&settings).expect("Serialize should succeed");
	let settings_obj = settings_value.as_object().expect("Should be object");
	let settings_map: HashMap<String, serde_json::Value> = settings_obj
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let result = validator.validate_settings(&settings_map);

	assert_eq!(
		result.is_ok(),
		should_pass,
		"Decision table case failed: {} - SecretKey={:?}",
		description,
		secret_key
	);
}

/// Test: Debug Mode + Profile Decision Table
///
/// Why: Validates debug mode rules across different profiles.
#[rstest]
#[case(Profile::Development, true, true, "Dev + debug=true → PASS")]
#[case(Profile::Development, false, true, "Dev + debug=false → PASS")]
#[case(Profile::Staging, true, true, "Staging + debug=true → PASS")]
#[case(Profile::Staging, false, true, "Staging + debug=false → PASS")]
#[case(Profile::Production, true, false, "Prod + debug=true → FAIL")]
#[case(
	Profile::Production,
	false,
	true,
	"Prod + debug=false → PASS (with valid secret)"
)]
fn test_debug_mode_profile_decision_table(
	#[case] profile: Profile,
	#[case] debug: bool,
	#[case] should_pass: bool,
	#[case] description: &str,
) {
	let mut settings = Settings::default();
	settings.debug = debug;
	settings.secret_key =
		"production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_string(); // Provide valid secret for prod

	// Production requires security settings
	if matches!(profile, Profile::Production) {
		settings.allowed_hosts = vec!["example.com".to_string()];
		settings.secure_ssl_redirect = true;
	}

	let validator = SecurityValidatorImpl::new(profile);
	let settings_value = serde_json::to_value(&settings).expect("Serialize should succeed");
	let settings_obj = settings_value.as_object().expect("Should be object");
	let settings_map: HashMap<String, serde_json::Value> = settings_obj
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let result = validator.validate_settings(&settings_map);

	assert_eq!(
		result.is_ok(),
		should_pass,
		"Decision table case failed: {} - Debug={}",
		description,
		debug
	);
}

/// Test: Source Merging with Null Values Decision Table
///
/// Why: Validates null value handling in source merging.
#[rstest]
#[case(json!("value1"), json!(null), json!(null), "Non-null overridden by null")]
#[case(json!(null), json!("value2"), json!("value2"), "Null overridden by non-null")]
#[case(json!("value1"), json!("value2"), json!("value2"), "Non-null overridden by non-null")]
#[case(json!(null), json!(null), json!(null), "Null remains null")]
fn test_source_merging_null_values_decision_table(
	#[case] source1_value: serde_json::Value,
	#[case] source2_value: serde_json::Value,
	#[case] expected_value: serde_json::Value,
	#[case] description: &str,
) {
	let key = "test_key";

	let source1 = DefaultSource::default().with_value(key, source1_value);
	let source2 = DefaultSource::default().with_value(key, source2_value.clone());

	let merged = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build()
		.expect("Build should succeed");

	let result: serde_json::Value = merged.get(key).expect("Get should succeed");

	assert_eq!(
		result, expected_value,
		"Decision table case failed: {}",
		description
	);
}
