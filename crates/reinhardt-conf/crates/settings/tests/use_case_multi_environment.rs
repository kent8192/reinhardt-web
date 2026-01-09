//! Integration tests for Multi-Environment Deployment Use Case.
//!
//! This test module validates real-world multi-environment deployment scenarios
//! where applications are deployed across Development, Staging, and Production
//! environments with profile-specific configuration.
//!
//! ## Scenario
//!
//! - Base configuration shared across all environments
//! - Profile-specific overrides (development.toml, staging.toml, production.toml)
//! - SecurityValidator enforces production safety requirements
//! - Database configurations vary by environment

use reinhardt_settings::builder::SettingsBuilder;
use reinhardt_settings::profile::Profile;
use reinhardt_settings::sources::TomlFileSource;
use reinhardt_settings::validation::{SecurityValidator, SettingsValidator};
use rstest::*;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Test: Multi-environment deployment workflow
///
/// Why: Validates that applications can correctly load profile-specific
/// configuration across Development, Staging, and Production environments
/// with appropriate security validations.
#[rstest]
#[test]
fn test_multi_environment_deployment() {
	let temp_dir = TempDir::new().unwrap();

	// Create base configuration (shared across all environments)
	let base_config = temp_dir.path().join("base.toml");
	let mut file = fs::File::create(&base_config).unwrap();
	writeln!(
		file,
		r#"
base_dir = "/app"
static_url = "/static/"
media_url = "/media/"
language_code = "en-us"
time_zone = "UTC"
use_i18n = true
use_tz = true
default_auto_field = "BigAutoField"
installed_apps = ["core", "auth", "sessions"]
middleware = ["SecurityMiddleware", "SessionMiddleware"]
root_urlconf = "app.urls"
"#
	)
	.unwrap();

	// Create development.toml
	let dev_config = temp_dir.path().join("development.toml");
	let mut file = fs::File::create(&dev_config).unwrap();
	writeln!(
		file,
		r#"
secret_key = "development-secret-key-for-local-testing-only"
debug = true
allowed_hosts = ["localhost", "127.0.0.1"]

[databases.default]
ENGINE = "sqlite"
NAME = ":memory:"
"#
	)
	.unwrap();

	// Create staging.toml
	let staging_config = temp_dir.path().join("staging.toml");
	let mut file = fs::File::create(&staging_config).unwrap();
	writeln!(
		file,
		r#"
secret_key = "staging-secret-key-minimum-32-characters-long-for-security"
debug = false
allowed_hosts = ["staging.example.com"]

[databases.default]
ENGINE = "postgresql"
NAME = "staging_db"
USER = "staging_user"
PASSWORD = "staging_pass"
HOST = "staging-db.internal"
PORT = "5432"
"#
	)
	.unwrap();

	// Create production.toml
	let prod_config = temp_dir.path().join("production.toml");
	let mut file = fs::File::create(&prod_config).unwrap();
	writeln!(
		file,
		r#"
secret_key = "production-secret-key-must-be-very-secure-and-random-generated"
debug = false
allowed_hosts = ["example.com", "www.example.com"]

[databases.default]
ENGINE = "postgresql"
NAME = "production_db"
USER = "prod_user"
PASSWORD = "prod_pass"
HOST = "prod-db.internal"
PORT = "5432"
"#
	)
	.unwrap();

	// Test 1: Development environment
	let dev_settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&base_config))
		.add_source(TomlFileSource::new(&dev_config))
		.build()
		.expect("Development settings should build successfully");

	assert_eq!(
		dev_settings.get::<bool>("debug").unwrap(),
		true,
		"Development should have debug=true"
	);

	let dev_allowed_hosts = dev_settings.get::<Vec<String>>("allowed_hosts").unwrap();
	assert!(
		dev_allowed_hosts.contains(&"localhost".to_string()),
		"Development should allow localhost"
	);

	// Verify development database is SQLite
	let dev_db_engine = dev_settings
		.as_map()
		.get("databases")
		.and_then(|v| v.as_object())
		.and_then(|dbs| dbs.get("default"))
		.and_then(|db| db.as_object())
		.and_then(|db| db.get("ENGINE"))
		.and_then(|e| e.as_str())
		.expect("Development database ENGINE should exist");
	assert_eq!(
		dev_db_engine, "sqlite",
		"Development should use SQLite database"
	);

	// Test 2: Staging environment
	let staging_settings = SettingsBuilder::new()
		.profile(Profile::Staging)
		.add_source(TomlFileSource::new(&base_config))
		.add_source(TomlFileSource::new(&staging_config))
		.build()
		.expect("Staging settings should build successfully");

	assert_eq!(
		staging_settings.get::<bool>("debug").unwrap(),
		false,
		"Staging should have debug=false"
	);

	let staging_allowed_hosts = staging_settings
		.get::<Vec<String>>("allowed_hosts")
		.unwrap();
	assert!(
		staging_allowed_hosts.contains(&"staging.example.com".to_string()),
		"Staging should allow staging.example.com"
	);

	// Verify staging database is PostgreSQL
	let staging_db_engine = staging_settings
		.as_map()
		.get("databases")
		.and_then(|v| v.as_object())
		.and_then(|dbs| dbs.get("default"))
		.and_then(|db| db.as_object())
		.and_then(|db| db.get("ENGINE"))
		.and_then(|e| e.as_str())
		.expect("Staging database ENGINE should exist");
	assert_eq!(
		staging_db_engine, "postgresql",
		"Staging should use PostgreSQL database"
	);

	// Test 3: Production environment with SecurityValidator
	let prod_settings = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(&base_config))
		.add_source(TomlFileSource::new(&prod_config))
		.build()
		.expect("Production settings should build successfully");

	// Manually validate with SecurityValidator
	let validator = SecurityValidator::new(Profile::Production);
	let settings_map: HashMap<String, serde_json::Value> = prod_settings
		.as_map()
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();
	validator
		.validate_settings(&settings_map)
		.expect("Production settings should pass security validation");

	assert_eq!(
		prod_settings.get::<bool>("debug").unwrap(),
		false,
		"Production should have debug=false"
	);

	let prod_secret_key = prod_settings.get::<String>("secret_key").unwrap();
	assert!(
		prod_secret_key.len() >= 32,
		"Production secret_key should be at least 32 characters"
	);
	assert!(
		!prod_secret_key.contains("insecure"),
		"Production secret_key should not contain 'insecure'"
	);

	let prod_allowed_hosts = prod_settings.get::<Vec<String>>("allowed_hosts").unwrap();
	assert!(
		prod_allowed_hosts.contains(&"example.com".to_string()),
		"Production should allow example.com"
	);
	assert!(
		!prod_allowed_hosts.contains(&"*".to_string()),
		"Production should not allow wildcard hosts"
	);

	// Verify production database is PostgreSQL
	let prod_db_engine = prod_settings
		.as_map()
		.get("databases")
		.and_then(|v| v.as_object())
		.and_then(|dbs| dbs.get("default"))
		.and_then(|db| db.as_object())
		.and_then(|db| db.get("ENGINE"))
		.and_then(|e| e.as_str())
		.expect("Production database ENGINE should exist");
	assert_eq!(
		prod_db_engine, "postgresql",
		"Production should use PostgreSQL database"
	);

	// Verify production database credentials
	let prod_db_name = prod_settings
		.as_map()
		.get("databases")
		.and_then(|v| v.as_object())
		.and_then(|dbs| dbs.get("default"))
		.and_then(|db| db.as_object())
		.and_then(|db| db.get("NAME"))
		.and_then(|n| n.as_str())
		.expect("Production database NAME should exist");
	assert_eq!(
		prod_db_name, "production_db",
		"Production database name should be production_db"
	);
}

/// Test: Production rejects insecure configuration
///
/// Why: Validates that SecurityValidator correctly rejects production
/// configurations that violate security requirements (debug=true).
#[rstest]
#[test]
fn test_production_rejects_debug_true() {
	let temp_dir = TempDir::new().unwrap();

	// Create insecure production configuration with debug=true
	let insecure_config = temp_dir.path().join("insecure_production.toml");
	let mut file = fs::File::create(&insecure_config).unwrap();
	writeln!(
		file,
		r#"
base_dir = "/app"
secret_key = "secure-production-key-with-enough-length-for-security-requirements"
debug = true
allowed_hosts = ["example.com"]
installed_apps = ["core"]
middleware = ["SecurityMiddleware"]
root_urlconf = "app.urls"
static_url = "/static/"
media_url = "/media/"
language_code = "en-us"
time_zone = "UTC"
use_i18n = true
use_tz = true
default_auto_field = "BigAutoField"

[databases.default]
ENGINE = "postgresql"
NAME = "db"
"#
	)
	.unwrap();

	// Attempt to build production settings with debug=true
	let settings = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(&insecure_config))
		.build()
		.expect("Build should succeed");

	// Validate with SecurityValidator
	let validator = SecurityValidator::new(Profile::Production);
	let settings_map: HashMap<String, serde_json::Value> = settings
		.as_map()
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();
	let result = validator.validate_settings(&settings_map);

	// Should fail due to debug=true in production
	assert!(
		result.is_err(),
		"Production settings with debug=true should fail validation"
	);

	let error_message = result.unwrap_err().to_string();
	assert!(
		error_message.contains("DEBUG") || error_message.to_lowercase().contains("debug"),
		"Error message should mention debug setting"
	);
}

/// Test: Production rejects weak secret key
///
/// Why: Validates that SecurityValidator rejects production configurations
/// with insecure secret keys (too short, contains 'insecure', etc.).
#[rstest]
#[test]
fn test_production_rejects_weak_secret_key() {
	let temp_dir = TempDir::new().unwrap();

	// Create configuration with weak secret key
	let weak_key_config = temp_dir.path().join("weak_key_production.toml");
	let mut file = fs::File::create(&weak_key_config).unwrap();
	writeln!(
		file,
		r#"
base_dir = "/app"
secret_key = "insecure"
debug = false
allowed_hosts = ["example.com"]
installed_apps = ["core"]
middleware = ["SecurityMiddleware"]
root_urlconf = "app.urls"
static_url = "/static/"
media_url = "/media/"
language_code = "en-us"
time_zone = "UTC"
use_i18n = true
use_tz = true
default_auto_field = "BigAutoField"

[databases.default]
ENGINE = "postgresql"
NAME = "db"
"#
	)
	.unwrap();

	// Attempt to build with weak secret key
	let settings = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(&weak_key_config))
		.build()
		.expect("Build should succeed");

	// Validate with SecurityValidator
	let validator = SecurityValidator::new(Profile::Production);
	let settings_map: HashMap<String, serde_json::Value> = settings
		.as_map()
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();
	let result = validator.validate_settings(&settings_map);

	// Should fail due to weak secret_key
	assert!(
		result.is_err(),
		"Production settings with weak secret_key should fail validation"
	);

	let error_message = result.unwrap_err().to_string();
	assert!(
		error_message.contains("SECRET_KEY") || error_message.contains("secret_key"),
		"Error message should mention secret_key"
	);
}

/// Test: Development environment allows debug mode
///
/// Why: Validates that SecurityValidator does NOT enforce production
/// security requirements in Development environment.
#[rstest]
#[test]
fn test_development_allows_debug_mode() {
	let temp_dir = TempDir::new().unwrap();

	// Create development configuration with debug=true and weak key
	let dev_config = temp_dir.path().join("development.toml");
	let mut file = fs::File::create(&dev_config).unwrap();
	writeln!(
		file,
		r#"
base_dir = "/app"
secret_key = "dev-key"
debug = true
allowed_hosts = ["*"]
installed_apps = ["core"]
middleware = ["SecurityMiddleware"]
root_urlconf = "app.urls"
static_url = "/static/"
media_url = "/media/"
language_code = "en-us"
time_zone = "UTC"
use_i18n = true
use_tz = true
default_auto_field = "BigAutoField"

[databases.default]
ENGINE = "sqlite"
NAME = ":memory:"
"#
	)
	.unwrap();

	// Build with Development profile (SecurityValidator should pass)
	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&dev_config))
		.build()
		.expect("Build should succeed");

	// Validate with SecurityValidator
	let validator = SecurityValidator::new(Profile::Development);
	let settings_map: HashMap<String, serde_json::Value> = settings
		.as_map()
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();
	let result = validator.validate_settings(&settings_map);

	// Should succeed in development
	assert!(
		result.is_ok(),
		"Development settings with debug=true and weak key should pass validation"
	);

	assert_eq!(
		settings.get::<bool>("debug").unwrap(),
		true,
		"Development can have debug=true"
	);
}

/// Test: Profile-specific configuration cascading
///
/// Why: Validates that base configuration is correctly inherited and
/// overridden by profile-specific configuration.
#[rstest]
#[test]
fn test_profile_configuration_cascading() {
	let temp_dir = TempDir::new().unwrap();

	// Create base configuration
	let base_config = temp_dir.path().join("base.toml");
	let mut file = fs::File::create(&base_config).unwrap();
	writeln!(
		file,
		r#"
base_dir = "/app"
language_code = "en-us"
time_zone = "UTC"
use_i18n = true
use_tz = true
static_url = "/static/"
"#
	)
	.unwrap();

	// Create environment-specific config that overrides base
	let env_config = temp_dir.path().join("environment.toml");
	let mut file = fs::File::create(&env_config).unwrap();
	writeln!(
		file,
		r#"
time_zone = "Asia/Tokyo"
language_code = "ja"
"#
	)
	.unwrap();

	// Load with cascading
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&base_config))
		.add_source(TomlFileSource::new(&env_config))
		.build()
		.expect("Settings should build successfully");

	// Verify cascading: env_config overrides base_config
	assert_eq!(
		settings.get::<String>("time_zone").unwrap(),
		"Asia/Tokyo",
		"Environment-specific time_zone should override base"
	);
	assert_eq!(
		settings.get::<String>("language_code").unwrap(),
		"ja",
		"Environment-specific language_code should override base"
	);

	// Verify inheritance: values from base_config are preserved
	assert_eq!(
		settings.get::<String>("static_url").unwrap(),
		"/static/",
		"Base configuration values should be inherited"
	);
}
