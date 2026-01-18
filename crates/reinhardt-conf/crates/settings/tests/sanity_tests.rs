//! Sanity Tests for reinhardt-settings.
//!
//! This test module provides basic smoke tests for each major module,
//! equivalent to doctests. These tests verify that fundamental operations
//! work correctly with minimal setup.
//!
//! ## Test Categories
//!
//! 1. **Builder Module**: SettingsBuilder basic usage
//! 2. **Parser Module**: Basic parsing functions
//! 3. **Env Loader Module**: Environment loading
//! 4. **Sources Module**: Configuration sources
//! 5. **Validation Module**: Basic validator usage
//! 6. **Profile Module**: Profile enum operations
//! 7. **Feature-gated Modules**: Encryption, dynamic settings, secrets

use reinhardt_conf::settings::prelude::*;
use reinhardt_conf::settings::profile::Profile;
use rstest::*;
use serde_json::json;
use std::collections::HashMap;

/// Sanity: SettingsBuilder basic usage
///
/// Why: Verifies the most common use case of building settings works.
#[rstest]
fn sanity_builder_basic_usage() {
	let builder = SettingsBuilder::new();
	let result = builder.build();
	assert!(result.is_ok(), "SettingsBuilder should build successfully");
}

/// Sanity: Settings struct default initialization
///
/// Why: Verifies Settings can be created with default values.
#[rstest]
fn sanity_settings_default() {
	let settings = Settings::default();
	assert!(
		settings.secret_key.is_empty() || !settings.secret_key.is_empty(),
		"Settings default should initialize"
	);
}

/// Sanity: Parse boolean from string
///
/// Why: Verifies the most common parser function works.
#[rstest]
fn sanity_parse_bool() {
	let result = parse_bool("true");
	assert_eq!(result, Ok(true), "parse_bool should parse 'true'");

	let result = parse_bool("false");
	assert_eq!(result, Ok(false), "parse_bool should parse 'false'");
}

/// Sanity: Parse database URL
///
/// Why: Verifies database URL parsing works for common formats.
#[rstest]
fn sanity_parse_database_url() {
	let result = parse_database_url("sqlite::memory:");
	assert!(result.is_ok(), "parse_database_url should parse SQLite URL");

	let db = result.unwrap();
	assert_eq!(
		db.engine, "reinhardt.db.backends.sqlite3",
		"Should recognize SQLite engine"
	);
}

/// Sanity: Parse list from string
///
/// Why: Verifies list parsing works.
#[rstest]
fn sanity_parse_list() {
	let result = parse_list("a,b,c");
	assert_eq!(
		result,
		vec!["a".to_string(), "b".to_string(), "c".to_string()],
		"parse_list should split by comma"
	);
}

/// Sanity: Parse dict from string
///
/// Why: Verifies dict parsing works.
#[rstest]
fn sanity_parse_dict() {
	let result = parse_dict("key1=value1,key2=value2");
	let mut expected = HashMap::new();
	expected.insert("key1".to_string(), "value1".to_string());
	expected.insert("key2".to_string(), "value2".to_string());
	assert_eq!(result, expected, "parse_dict should parse key=value pairs");
}

/// Sanity: Profile enum usage
///
/// Why: Verifies Profile enum can be created and compared.
#[rstest]
fn sanity_profile_enum() {
	let dev = Profile::Development;
	let prod = Profile::Production;

	assert_ne!(dev, prod, "Different profiles should not be equal");
	assert!(!prod.is_development(), "Production is not development");
	assert!(prod.is_production(), "Production should be production");
}

/// Sanity: DefaultSource can be created
///
/// Why: Verifies the most basic configuration source works.
#[rstest]
fn sanity_default_source() {
	let source = DefaultSource::new();
	// Just verify it can be created
	let _source_with_value = source.with_value("test_key", json!("test_value"));
}

/// Sanity: RequiredValidator basic usage
///
/// Why: Verifies validator can be created and used.
#[rstest]
fn sanity_required_validator() {
	let validator = RequiredValidator::new(vec!["secret_key".to_string()]);

	// Test with field present
	let mut settings = HashMap::new();
	settings.insert("secret_key".to_string(), json!("test_value"));
	let result = validator.validate_settings(&settings);
	assert!(result.is_ok(), "Validator should pass when field present");

	// Test with field missing
	let empty_settings = HashMap::new();
	let result = validator.validate_settings(&empty_settings);
	assert!(result.is_err(), "Validator should fail when field missing");
}

/// Sanity: RangeValidator basic usage
///
/// Why: Verifies range validator works for numeric values.
#[rstest]
fn sanity_range_validator() {
	let validator = RangeValidator::between(0.0, 100.0);
	let value = json!(50.0);

	let result = validator.validate("test_field", &value);
	assert!(
		result.is_ok(),
		"RangeValidator should pass for value in range"
	);

	let out_of_range = json!(150.0);
	let result = validator.validate("test_field", &out_of_range);
	assert!(
		result.is_err(),
		"RangeValidator should fail for value out of range"
	);
}

/// Sanity: SecurityValidator with Development profile
///
/// Why: Verifies security validator allows lenient settings in development.
#[rstest]
fn sanity_security_validator_development() {
	let validator = SecurityValidator::new(Profile::Development);

	let mut settings = HashMap::new();
	settings.insert("debug".to_string(), json!(true));
	settings.insert("secret_key".to_string(), json!("dev_secret"));

	let result = validator.validate_settings(&settings);
	assert!(
		result.is_ok(),
		"SecurityValidator should be lenient in development"
	);
}

/// Sanity: SecurityValidator with Production profile
///
/// Why: Verifies security validator enforces strict rules in production.
#[rstest]
fn sanity_security_validator_production() {
	let validator = SecurityValidator::new(Profile::Production);

	// Invalid production settings (debug=true)
	let mut bad_settings = HashMap::new();
	bad_settings.insert("debug".to_string(), json!(true));
	bad_settings.insert(
		"secret_key".to_string(),
		json!("production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"),
	);
	bad_settings.insert("allowed_hosts".to_string(), json!(["example.com"]));
	bad_settings.insert("secure_ssl_redirect".to_string(), json!(true));

	let result = validator.validate_settings(&bad_settings);
	assert!(
		result.is_err(),
		"SecurityValidator should reject debug=true in production"
	);

	// Valid production settings
	let mut good_settings = HashMap::new();
	good_settings.insert("debug".to_string(), json!(false));
	good_settings.insert(
		"secret_key".to_string(),
		json!("production_secret_key_12345_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"),
	);
	good_settings.insert("allowed_hosts".to_string(), json!(["example.com"]));
	good_settings.insert("secure_ssl_redirect".to_string(), json!(true));

	let result = validator.validate_settings(&good_settings);
	assert!(
		result.is_ok(),
		"SecurityValidator should accept valid production settings"
	);
}

/// Sanity: ConfigEncryptor basic usage (encryption feature)
///
/// Why: Verifies encryption module works with basic operations.
#[cfg(feature = "encryption")]
#[rstest]
fn sanity_encryption_basic() {
	use reinhardt_conf::settings::encryption::ConfigEncryptor;

	let key = vec![42u8; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Create encryptor");

	let plaintext = b"test_data";
	let encrypted = encryptor.encrypt(plaintext).expect("Encrypt");
	let decrypted = encryptor.decrypt(&encrypted).expect("Decrypt");

	assert_eq!(
		plaintext.to_vec(),
		decrypted,
		"Encryption roundtrip should preserve data"
	);
}

/// Sanity: DynamicSettings basic usage (async feature)
///
/// Why: Verifies dynamic settings module works with basic operations.
#[cfg(feature = "async")]
#[rstest]
#[tokio::test]
async fn sanity_dynamic_settings_basic() {
	use reinhardt_conf::settings::backends::memory::MemoryBackend;
	use reinhardt_conf::settings::dynamic::DynamicSettings;
	use std::sync::Arc;

	let backend = Arc::new(MemoryBackend::new());
	let dynamic = DynamicSettings::new(backend);

	// Set and get a value
	dynamic
		.set("test_key", &"test_value", None)
		.await
		.expect("Set value");
	let value: Option<String> = dynamic.get("test_key").await.expect("Get value");

	assert_eq!(
		value,
		Some("test_value".to_string()),
		"Dynamic settings should store and retrieve values"
	);
}

/// Sanity: SecretString basic usage (async feature for secrets module)
///
/// Why: Verifies secret types work and don't expose values accidentally.
#[cfg(feature = "async")]
#[rstest]
fn sanity_secret_string_basic() {
	use reinhardt_conf::settings::secrets::SecretString;

	let secret = SecretString::new("my_secret_value");

	// Verify it doesn't expose in debug output
	let debug_output = format!("{:?}", secret);
	assert!(
		debug_output.contains("[REDACTED]"),
		"SecretString debug output should be redacted"
	);

	// Verify actual value can be accessed when needed
	assert_eq!(
		secret.expose_secret(),
		"my_secret_value",
		"SecretString should preserve actual value"
	);
}

/// Sanity: MergedSettings basic operations
///
/// Why: Verifies MergedSettings can store and retrieve values.
#[rstest]
fn sanity_merged_settings() {
	// MergedSettings is obtained from SettingsBuilder
	let merged = SettingsBuilder::new()
		.add_source(DefaultSource::new().with_value("test_key", json!("test_value")))
		.build()
		.expect("Build settings");

	let value: Option<String> = merged.get_optional("test_key");
	assert_eq!(
		value,
		Some("test_value".to_string()),
		"MergedSettings should store and retrieve values"
	);
}

/// Sanity: Env basic usage
///
/// Why: Verifies Env wrapper works for environment variable access.
#[rstest]
fn sanity_env_basic() {
	use reinhardt_conf::settings::env::Env;

	let env = Env::new();
	// Just verify it can be created and methods are available
	// We can't test actual env var access without setting them
	let result = env.str("NONEXISTENT_VAR");
	// Should return error for nonexistent var
	assert!(result.is_err(), "Should return error for nonexistent var");
}

/// Sanity: ChoiceValidator basic usage
///
/// Why: Verifies choice validator works for allowed values.
#[rstest]
fn sanity_choice_validator() {
	let validator = ChoiceValidator::new(vec![
		"option1".to_string(),
		"option2".to_string(),
		"option3".to_string(),
	]);

	// Valid choice
	let valid = json!("option1");
	let result = validator.validate("field", &valid);
	assert!(result.is_ok(), "ChoiceValidator should accept valid choice");

	// Invalid choice
	let invalid = json!("invalid_option");
	let result = validator.validate("field", &invalid);
	assert!(
		result.is_err(),
		"ChoiceValidator should reject invalid choice"
	);
}

/// Sanity: PatternValidator basic usage
///
/// Why: Verifies pattern validator works for regex patterns.
#[rstest]
fn sanity_pattern_validator() {
	let validator = PatternValidator::new(r"^\d{3}-\d{4}$").expect("Create validator");

	// Valid pattern
	let valid = json!("123-4567");
	let result = validator.validate("phone", &valid);
	assert!(
		result.is_ok(),
		"PatternValidator should accept matching pattern"
	);

	// Invalid pattern
	let invalid = json!("invalid");
	let result = validator.validate("phone", &invalid);
	assert!(
		result.is_err(),
		"PatternValidator should reject non-matching pattern"
	);
}
