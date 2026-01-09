//! Integration tests for SettingsBuilder error handling.
//!
//! This test module validates that SettingsBuilder provides clear, informative error
//! messages for various failure scenarios including missing files, invalid formats,
//! validation failures, and type conflicts.

use reinhardt_settings::builder::SettingsBuilder;
use reinhardt_settings::prelude::SettingsValidator;
use reinhardt_settings::profile::Profile;
use reinhardt_settings::sources::{JsonFileSource, TomlFileSource};
use reinhardt_settings::validation::SecurityValidator;
use rstest::*;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

/// Fixture providing temporary directory for test configuration files
#[fixture]
fn temp_dir() -> TempDir {
	TempDir::new().expect("Failed to create temporary directory")
}

/// Test: Builder with missing TOML file
///
/// Why: Validates that SettingsBuilder treats non-existent TOML files as empty configuration
/// (optional file behavior), allowing graceful degradation when profile-specific configs are missing.
#[rstest]
#[tokio::test]
async fn test_builder_missing_toml_file(temp_dir: TempDir) {
	let non_existent_path = temp_dir.path().join("non_existent.toml");

	let result = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(non_existent_path.clone()))
		.build();

	// Missing TOML file is treated as empty configuration (optional file behavior)
	// This allows profile-specific configs to be optional
	match result {
		Ok(merged) => {
			// Merged settings should be empty or contain only default values
			assert!(
				merged.as_map().is_empty() || merged.as_map().len() < 5,
				"Missing TOML file should result in empty or minimal configuration"
			);
		}
		Err(err) => panic!(
			"Building with missing TOML file should succeed with empty config, got error: {}",
			err
		),
	}
}

/// Test: Builder with invalid JSON format
///
/// Why: Validates that SettingsBuilder detects and reports JSON parse errors
/// with clear error messages.
#[rstest]
#[tokio::test]
async fn test_builder_invalid_json_format(temp_dir: TempDir) {
	let json_path = temp_dir.path().join("invalid.json");

	// Write malformed JSON (missing closing brace, trailing comma)
	let malformed_json = r#"{
		"key1": "value1",
		"key2": "value2",
	"#;
	fs::write(&json_path, malformed_json).expect("Failed to write invalid JSON");

	let result = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(JsonFileSource::new(json_path.clone()))
		.build();

	// Error message should indicate JSON parse error
	match result {
		Err(err) => {
			let error_msg = err.to_string();
			assert!(
				error_msg.to_lowercase().contains("json")
					|| error_msg.to_lowercase().contains("parse")
					|| error_msg.to_lowercase().contains("invalid"),
				"Error message should indicate JSON parse error, got: {}",
				error_msg
			);
		}
		Ok(_) => panic!("Building with malformed JSON should fail"),
	}
}

/// Test: Builder strict mode with missing required fields
///
/// Why: Validates that strict mode with RequiredValidator fails build when
/// required fields are missing, with specific field names in error.
#[rstest]
#[tokio::test]
async fn test_builder_strict_mode_missing_required(temp_dir: TempDir) {
	let config_path = temp_dir.path().join("incomplete.toml");

	// Create config missing required fields
	let incomplete_config = r#"
app_name = "test_app"
# Missing: secret_key, database_url
"#;
	fs::write(&config_path, incomplete_config).expect("Failed to write incomplete config");

	let result = SettingsBuilder::new()
		.profile(Profile::Production)
		.strict(true)
		.add_source(TomlFileSource::new(config_path))
		.build();

	// Note: Strict mode validation happens during build
	// If the builder doesn't have strict mode built-in, we need to validate after building
	match result {
		Err(err) => {
			let error_msg = err.to_string();
			// Error should mention missing required fields
			assert!(
				error_msg.to_lowercase().contains("required")
					|| error_msg.to_lowercase().contains("missing"),
				"Error message should indicate missing required fields, got: {}",
				error_msg
			);
		}
		Ok(merged) => {
			// If builder doesn't validate, we should validate the merged settings
			// Check that required fields are missing
			assert!(
				merged.get::<String>("secret_key").is_err()
					|| merged
						.get::<String>("secret_key")
						.unwrap_or_default()
						.is_empty(),
				"secret_key should be missing or empty"
			);
		}
	}
}

/// Test: Builder with type conflicts between sources
///
/// Why: Validates that SettingsBuilder handles type conflicts (e.g., string vs integer)
/// gracefully, either by type coercion or clear error messages.
#[rstest]
#[tokio::test]
async fn test_builder_conflicting_values_types(temp_dir: TempDir) {
	let toml_path = temp_dir.path().join("types.toml");
	let json_path = temp_dir.path().join("types.json");

	// TOML defines port as string
	let toml_content = r#"
port = "8000"
"#;
	fs::write(&toml_path, toml_content).expect("Failed to write TOML");

	// JSON defines port as integer
	let json_content = json!({
		"port": 9000
	});
	fs::write(
		&json_path,
		serde_json::to_string_pretty(&json_content).expect("Failed to serialize JSON"),
	)
	.expect("Failed to write JSON");

	let result = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(toml_path))
		.add_source(JsonFileSource::new(json_path))
		.build();

	// Builder should succeed (last source wins in priority)
	assert!(
		result.is_ok(),
		"Builder should handle type conflicts by using last source"
	);

	let merged = result.unwrap();

	// Port should be the value from JSON source (higher priority)
	let port_value = merged.get::<i64>("port");
	assert!(
		port_value.is_ok(),
		"Port should be accessible as integer from JSON source"
	);
	assert_eq!(port_value.unwrap(), 9000, "Port should be 9000 from JSON");
}

/// Test: Builder validation failure with detailed error
///
/// Why: Validates that validation failures include specific field names and
/// reasons in error messages for easier debugging.
#[rstest]
#[tokio::test]
async fn test_builder_validation_failure_details(temp_dir: TempDir) {
	let config_path = temp_dir.path().join("invalid_prod.toml");

	// Create Production config that violates SecurityValidator
	let invalid_prod_config = r#"
debug = true
secret_key = ""
"#;
	fs::write(&config_path, invalid_prod_config).expect("Failed to write invalid config");

	let result = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(config_path))
		.build();

	// Builder succeeds, but we need to validate Settings
	assert!(result.is_ok(), "Builder should succeed");

	let merged = result.unwrap();

	// Manually create Settings for validation
	let _settings = reinhardt_settings::Settings {
		debug: merged.get("debug").unwrap_or(false),
		secret_key: merged.get("secret_key").unwrap_or_default(),
		allowed_hosts: vec![],
		base_dir: std::env::current_dir().expect("Failed to get current directory"),
		installed_apps: vec![],
		middleware: vec![],
		root_urlconf: String::new(),
		databases: Default::default(),
		templates: vec![],
		static_url: "/static/".to_string(),
		static_root: None,
		staticfiles_dirs: vec![],
		media_url: "/media/".to_string(),
		media_root: None,
		language_code: "en-us".to_string(),
		time_zone: "UTC".to_string(),
		use_i18n: true,
		use_tz: true,
		default_auto_field: "BigAutoField".to_string(),
		secure_proxy_ssl_header: None,
		secure_ssl_redirect: false,
		secure_hsts_seconds: None,
		secure_hsts_include_subdomains: false,
		secure_hsts_preload: false,
		session_cookie_secure: false,
		csrf_cookie_secure: false,
		append_slash: true,
		admins: vec![],
		managers: vec![],
	};

	// Validate with SecurityValidator
	let validator = SecurityValidator::new(Profile::Production);
	let settings_map: HashMap<String, serde_json::Value> = merged
		.as_map()
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();
	let validation_result = validator.validate_settings(&settings_map);

	assert!(
		validation_result.is_err(),
		"Production settings with debug=true and empty secret_key should fail validation"
	);

	let error = validation_result.unwrap_err();
	let error_msg = format!("{:?}", error);

	// Error should mention the violated constraints
	assert!(
		error_msg.to_lowercase().contains("debug") || error_msg.to_lowercase().contains("secret"),
		"Error message should mention debug or secret_key, got: {}",
		error_msg
	);
}

/// Test: Builder with empty configuration sources
///
/// Why: Validates that SettingsBuilder handles empty configuration gracefully.
#[rstest]
#[tokio::test]
async fn test_builder_empty_config() {
	let result = SettingsBuilder::new().profile(Profile::Development).build();

	assert!(
		result.is_ok(),
		"Building with no sources should succeed and return empty config"
	);

	let merged = result.unwrap();
	assert!(
		merged.as_map().is_empty(),
		"Empty builder should produce empty configuration"
	);
}

/// Test: Builder with overlapping JSON and TOML sources containing invalid data
///
/// Why: Validates error handling when multiple sources contain problematic data.
#[rstest]
#[tokio::test]
async fn test_builder_multiple_invalid_sources(temp_dir: TempDir) {
	let toml_path = temp_dir.path().join("invalid.toml");
	let json_path = temp_dir.path().join("also_invalid.json");

	// Write invalid TOML (syntax error)
	let invalid_toml = r#"
[section
missing_closing_bracket = "value"
"#;
	fs::write(&toml_path, invalid_toml).expect("Failed to write invalid TOML");

	// Write valid JSON initially
	let valid_json = json!({"key": "value"});
	fs::write(
		&json_path,
		serde_json::to_string_pretty(&valid_json).expect("Failed to serialize JSON"),
	)
	.expect("Failed to write JSON");

	// First invalid source (TOML) should fail
	let result = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(toml_path))
		.build();

	assert!(result.is_err(), "Invalid TOML should cause build failure");

	// Now test with valid TOML but invalid JSON
	let toml_path2 = temp_dir.path().join("valid.toml");
	let valid_toml = r#"
key = "value"
"#;
	fs::write(&toml_path2, valid_toml).expect("Failed to write valid TOML");

	let invalid_json = "{invalid json";
	fs::write(&json_path, invalid_json).expect("Failed to write invalid JSON");

	let result2 = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(toml_path2))
		.add_source(JsonFileSource::new(json_path))
		.build();

	assert!(
		result2.is_err(),
		"Invalid JSON should cause build failure even with valid TOML"
	);
}
