//! Integration tests for environment variable override and environment-specific settings.
//!
//! This test suite verifies:
//! - Environment variable override behavior
//! - Environment-specific settings (dev, staging, prod)
//! - Settings validation with environment variables
//! - Secret management from environment
//! - Settings reload on environment change
//! - Environment variable type conversion

use reinhardt_conf::{
	settings::prelude::*,
	sources::{EnvSource, LowPriorityEnvSource, TomlFileSource},
};
use reinhardt_test::resource::{TeardownGuard, TestResource};
use rstest::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use std::{collections::HashMap, env, path::PathBuf, sync::Arc};
use tempfile::TempDir;

/// Guard for environment variable cleanup
struct EnvGuard {
	vars: Vec<String>,
}

impl TestResource for EnvGuard {
	fn setup() -> Self {
		// Clear any existing Reinhardt environment variables
		for (key, _) in env::vars() {
			if key.starts_with("REINHARDT_") || key.starts_with("TEST_") {
				env::remove_var(&key);
			}
		}
		Self { vars: Vec::new() }
	}

	fn teardown(&mut self) {
		// Remove all tracked environment variables
		for key in &self.vars {
			env::remove_var(key);
		}
		// Extra cleanup for common prefixes
		for (key, _) in env::vars() {
			if key.starts_with("REINHARDT_") || key.starts_with("TEST_") {
				env::remove_var(&key);
			}
		}
	}
}

impl EnvGuard {
	fn set(&mut self, key: impl Into<String>, value: impl AsRef<str>) {
		let key_string = key.into();
		env::set_var(&key_string, value.as_ref());
		self.vars.push(key_string);
	}
}

#[fixture]
fn env_guard() -> TeardownGuard<EnvGuard> {
	TeardownGuard::new()
}

/// Test configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestConfig {
	debug: bool,
	host: String,
	port: u16,
	database_url: Option<String>,
	secret_key: Option<String>,
	max_connections: Option<usize>,
	timeout_seconds: Option<u64>,
}

/// Database configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseConfig {
	engine: String,
	host: String,
	port: u16,
	name: String,
	user: String,
	password: String,
}

/// Application configuration with nested structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
	debug: bool,
	secret_key: String,
	database: DatabaseConfig,
	cache_enabled: bool,
	workers: usize,
}

#[rstest]
#[serial(env)]
#[test]
fn test_env_var_override_basic(mut env_guard: TeardownGuard<EnvGuard>) {
	// Create temporary directory for config files
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	// Create base config file
	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Set environment variable to override port
	env_guard.set("REINHARDT_PORT", "9000");

	// Build settings with both file and env sources
	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Environment variable should override file setting
	assert_eq!(config.port, 9000);
	assert_eq!(config.host, "localhost");
	assert!(!config.debug);
}

#[rstest]
#[serial(env)]
#[test]
fn test_env_var_override_nested_with_underscore(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	// Create base config with database section
	std::fs::write(
		&config_path,
		r#"
debug = true
secret_key = "file-secret"

[database]
engine = "postgresql"
host = "localhost"
port = 5432
name = "testdb"
user = "user"
password = "password"
"#,
	)
	.expect("Failed to write config file");

	// Set environment variables with nested path (using __)
	env_guard.set("REINHARDT_DATABASE__HOST", "db.example.com");
	env_guard.set("REINHARDT_DATABASE__PORT", "5433");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: AppConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Environment variables should override nested settings
	assert_eq!(config.database.host, "db.example.com");
	assert_eq!(config.database.port, 5433);
	assert_eq!(config.database.engine, "postgresql");
	assert_eq!(config.database.name, "testdb");
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_specific_settings_development(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let base_path = temp_dir.path().join("base.toml");
	let dev_path = temp_dir.path().join("development.toml");

	// Base configuration
	std::fs::write(
		&base_path,
		r#"
debug = false
host = "0.0.0.0"
port = 8000
"#,
	)
	.expect("Failed to write base config");

	// Development-specific configuration
	std::fs::write(
		&dev_path,
		r#"
debug = true
host = "localhost"
database_url = "postgres://localhost/dev"
"#,
	)
	.expect("Failed to write dev config");

	// Set environment to development
	env_guard.set("REINHARDT_ENV", "development");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&base_path))
		.add_source(TomlFileSource::new(&dev_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Development settings should override base
	assert!(config.debug);
	assert_eq!(config.host, "localhost");
	assert_eq!(config.port, 8000); // From base
	assert_eq!(config.database_url.as_deref(), Some("postgres://localhost/dev"));
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_specific_settings_production(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let base_path = temp_dir.path().join("base.toml");
	let prod_path = temp_dir.path().join("production.toml");

	// Base configuration
	std::fs::write(
		&base_path,
		r#"
debug = false
host = "0.0.0.0"
port = 8000
"#,
	)
	.expect("Failed to write base config");

	// Production-specific configuration
	std::fs::write(
		&prod_path,
		r#"
debug = false
host = "0.0.0.0"
port = 443
database_url = "postgres://prod-db.example.com/prod"
max_connections = 100
"#,
	)
	.expect("Failed to write prod config");

	// Set environment to production
	env_guard.set("REINHARDT_ENV", "production");

	let settings = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(&base_path))
		.add_source(TomlFileSource::new(&prod_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Production settings should override base
	assert!(!config.debug);
	assert_eq!(config.host, "0.0.0.0");
	assert_eq!(config.port, 443);
	assert_eq!(
		config.database_url.as_deref(),
		Some("postgres://prod-db.example.com/prod")
	);
	assert_eq!(config.max_connections, Some(100));
}

#[rstest]
#[serial(env)]
#[test]
fn test_secret_management_from_environment(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	// Config file without secrets (secrets should come from env)
	std::fs::write(
		&config_path,
		r#"
debug = true

[database]
engine = "postgresql"
host = "localhost"
port = 5432
name = "testdb"
user = "user"
password = "placeholder"
"#,
	)
	.expect("Failed to write config file");

	// Set secrets via environment variables
	env_guard.set("REINHARDT_SECRET_KEY", "super-secret-key-from-env");
	env_guard.set("REINHARDT_DATABASE__PASSWORD", "secure-db-password");

	let settings = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: AppConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Secrets from environment should be used
	assert_eq!(config.secret_key, "super-secret-key-from-env");
	assert_eq!(config.database.password, "secure-db-password");
}

#[rstest]
#[serial(env)]
#[test]
fn test_settings_reload_on_environment_change(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Initial environment setup
	env_guard.set("REINHARDT_PORT", "8080");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");
	assert_eq!(config.port, 8080);

	// Change environment variable
	env_guard.set("REINHARDT_PORT", "9090");

	// Rebuild settings (simulating reload)
	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// New environment value should be reflected
	assert_eq!(config.port, 9090);
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_variable_type_conversion_integers(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = true
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Set integer values as strings in environment
	env_guard.set("REINHARDT_PORT", "3000");
	env_guard.set("REINHARDT_MAX_CONNECTIONS", "50");
	env_guard.set("REINHARDT_TIMEOUT_SECONDS", "30");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Type conversion should work correctly
	assert_eq!(config.port, 3000);
	assert_eq!(config.max_connections, Some(50));
	assert_eq!(config.timeout_seconds, Some(30));
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_variable_type_conversion_booleans(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Set boolean values as strings in environment
	env_guard.set("REINHARDT_DEBUG", "true");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Boolean conversion should work
	assert!(config.debug);
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_variable_priority_over_file(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let base_path = temp_dir.path().join("base.toml");
	let dev_path = temp_dir.path().join("development.toml");

	std::fs::write(
		&base_path,
		r#"
debug = false
host = "base-host"
port = 8000
"#,
	)
	.expect("Failed to write base config");

	std::fs::write(
		&dev_path,
		r#"
debug = true
host = "dev-host"
port = 8080
"#,
	)
	.expect("Failed to write dev config");

	// Environment variable should have highest priority
	env_guard.set("REINHARDT_HOST", "env-host");
	env_guard.set("REINHARDT_PORT", "9000");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&base_path))
		.add_source(TomlFileSource::new(&dev_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Environment variables should override everything
	assert_eq!(config.host, "env-host");
	assert_eq!(config.port, 9000);
	assert!(config.debug); // From dev file (not overridden)
}

#[rstest]
#[serial(env)]
#[test]
fn test_missing_required_environment_variable() {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	// Config file without required secret_key
	std::fs::write(
		&config_path,
		r#"
debug = true

[database]
engine = "postgresql"
host = "localhost"
port = 5432
name = "testdb"
user = "user"
password = "password"
"#,
	)
	.expect("Failed to write config file");

	// Don't set REINHARDT_SECRET_KEY in environment
	// Note: secret_key is required in AppConfig

	let settings = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	// Conversion should fail due to missing required field
	let result = settings.into_typed::<AppConfig>();
	assert!(result.is_err());
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_variable_with_default_values(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Don't set optional environment variables
	// max_connections and timeout_seconds should remain None

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Optional fields should be None
	assert!(config.max_connections.is_none());
	assert!(config.timeout_seconds.is_none());
	assert!(config.database_url.is_none());

	// Now set one optional field
	env_guard.set("REINHARDT_MAX_CONNECTIONS", "100");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Now max_connections should be set
	assert_eq!(config.max_connections, Some(100));
	assert!(config.timeout_seconds.is_none());
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_variable_case_sensitivity(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Set with correct case
	env_guard.set("REINHARDT_PORT", "9000");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Should work with uppercase
	assert_eq!(config.port, 9000);
}

#[rstest]
#[serial(env)]
#[test]
fn test_multiple_environment_prefixes() {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Create new guard for TEST_ prefix
	let mut test_env_guard = TeardownGuard::<EnvGuard>::new();

	// Set with different prefix
	test_env_guard.set("TEST_PORT", "7000");
	test_env_guard.set("TEST_DEBUG", "true");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("TEST_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Should work with TEST_ prefix
	assert_eq!(config.port, 7000);
	assert!(config.debug);
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_variable_empty_string(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
database_url = "postgres://localhost/db"
"#,
	)
	.expect("Failed to write config file");

	// Set empty string (should be treated as absence for optional fields)
	env_guard.set("REINHARDT_DATABASE_URL", "");

	let settings = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Empty string should override to empty, not None
	assert_eq!(config.database_url.as_deref(), Some(""));
}

#[rstest]
#[serial(env)]
#[test]
fn test_complex_nested_environment_override(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
secret_key = "base-secret"
cache_enabled = true
workers = 4

[database]
engine = "postgresql"
host = "localhost"
port = 5432
name = "testdb"
user = "user"
password = "password"
"#,
	)
	.expect("Failed to write config file");

	// Override multiple nested and top-level settings
	env_guard.set("REINHARDT_DEBUG", "true");
	env_guard.set("REINHARDT_SECRET_KEY", "env-secret");
	env_guard.set("REINHARDT_WORKERS", "8");
	env_guard.set("REINHARDT_DATABASE__HOST", "db.example.com");
	env_guard.set("REINHARDT_DATABASE__PORT", "5433");
	env_guard.set("REINHARDT_DATABASE__PASSWORD", "secure-password");

	let settings = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: AppConfig = settings.into_typed().expect("Failed to convert to typed config");

	// All environment overrides should be applied
	assert!(config.debug);
	assert_eq!(config.secret_key, "env-secret");
	assert_eq!(config.workers, 8);
	assert_eq!(config.database.host, "db.example.com");
	assert_eq!(config.database.port, 5433);
	assert_eq!(config.database.password, "secure-password");

	// Non-overridden values should remain from file
	assert!(config.cache_enabled);
	assert_eq!(config.database.engine, "postgresql");
	assert_eq!(config.database.name, "testdb");
	assert_eq!(config.database.user, "user");
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_validation_with_invalid_type(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let config_path = temp_dir.path().join("base.toml");

	std::fs::write(
		&config_path,
		r#"
debug = false
host = "localhost"
port = 8000
"#,
	)
	.expect("Failed to write config file");

	// Set invalid integer value
	env_guard.set("REINHARDT_PORT", "not-a-number");

	let result = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(&config_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build();

	// Should fail to build settings due to type conversion error
	assert!(result.is_err());
}

#[rstest]
#[serial(env)]
#[test]
fn test_environment_staging_configuration(mut env_guard: TeardownGuard<EnvGuard>) {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let base_path = temp_dir.path().join("base.toml");
	let staging_path = temp_dir.path().join("staging.toml");

	std::fs::write(
		&base_path,
		r#"
debug = false
host = "0.0.0.0"
port = 8000
"#,
	)
	.expect("Failed to write base config");

	std::fs::write(
		&staging_path,
		r#"
debug = true
host = "staging.example.com"
port = 8080
database_url = "postgres://staging-db.example.com/staging"
max_connections = 50
"#,
	)
	.expect("Failed to write staging config");

	env_guard.set("REINHARDT_ENV", "staging");

	let settings = SettingsBuilder::new()
		.profile(Profile::Staging)
		.add_source(TomlFileSource::new(&base_path))
		.add_source(TomlFileSource::new(&staging_path))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.build()
		.expect("Failed to build settings");

	let config: TestConfig = settings.into_typed().expect("Failed to convert to typed config");

	// Staging configuration should be applied
	assert!(config.debug);
	assert_eq!(config.host, "staging.example.com");
	assert_eq!(config.port, 8080);
	assert_eq!(
		config.database_url.as_deref(),
		Some("postgres://staging-db.example.com/staging")
	);
	assert_eq!(config.max_connections, Some(50));
}
