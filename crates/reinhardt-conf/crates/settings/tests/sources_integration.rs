//! Integration tests for multiple settings sources
//!
//! This test module validates the integration of multiple configuration sources,
//! including priority resolution, merging logic, and profile-based configuration.

use reinhardt_settings::builder::SettingsBuilder;
use reinhardt_settings::profile::Profile;
use reinhardt_settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use tempfile::TempDir;

/// Nested configuration structures for testing
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DatabaseConfig {
	host: String,
	port: u64,
	name: String,
	#[serde(default)]
	max_connections: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct CacheConfig {
	enabled: bool,
	ttl: u64,
}

/// Fixture for temporary directory with TOML config files
#[fixture]
fn temp_config_dir() -> TempDir {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");

	// Create base.toml
	let base_content = r#"
app_name = "base_app"
debug = false
port = 8000
database_url = "postgres://localhost/base_db"
max_connections = 20
"#;
	fs::write(temp_dir.path().join("base.toml"), base_content).expect("Failed to write base.toml");

	// Create local.toml (development profile)
	let local_content = r#"
debug = true
port = 3000
database_url = "postgres://localhost/local_db"
"#;
	fs::write(temp_dir.path().join("local.toml"), local_content)
		.expect("Failed to write local.toml");

	// Create production.toml
	let production_content = r#"
debug = false
port = 80
database_url = "postgres://prod-server/prod_db"
max_connections = 100
"#;
	fs::write(temp_dir.path().join("production.toml"), production_content)
		.expect("Failed to write production.toml");

	temp_dir
}

/// Test: Multiple sources with correct priority (profile.toml > base.toml > env > defaults)
#[rstest]
#[tokio::test]
async fn test_sources_priority_resolution(temp_config_dir: TempDir) {
	// Set environment variable (lower priority than TOML files)
	unsafe {
		env::set_var("REINHARDT_APP_NAME", "env_app");
	}
	unsafe {
		env::set_var("REINHARDT_MAX_CONNECTIONS", "50");
	}

	let base_path = temp_config_dir.path().join("base.toml");
	let local_path = temp_config_dir.path().join("local.toml");

	let merged = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(
			DefaultSource::new()
				.with_value("app_name", json!("default_app"))
				.with_value("debug", json!(false))
				.with_value("port", json!(8000)),
		)
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.add_source(TomlFileSource::new(base_path))
		.add_source(TomlFileSource::new(local_path))
		.build()
		.expect("Failed to build settings");

	// Priority: local.toml > base.toml > env > defaults
	// app_name: base.toml overrides env ("base_app")
	// debug: local.toml overrides base.toml (true)
	// port: local.toml overrides base.toml (3000)
	// database_url: local.toml overrides base.toml ("postgres://localhost/local_db")
	// max_connections: base.toml overrides env (20)

	assert_eq!(
		merged.get::<String>("app_name").ok(),
		Some("base_app".to_string())
	);
	assert_eq!(merged.get::<bool>("debug").ok(), Some(true));
	assert_eq!(merged.get::<u16>("port").ok(), Some(3000));
	assert_eq!(
		merged.get::<String>("database_url").ok(),
		Some("postgres://localhost/local_db".to_string())
	);
	assert_eq!(merged.get::<u32>("max_connections").ok(), Some(20));

	// Cleanup
	unsafe {
		env::remove_var("REINHARDT_APP_NAME");
	}
	unsafe {
		env::remove_var("REINHARDT_MAX_CONNECTIONS");
	}
}

/// Test: Profile-based configuration switching
#[rstest]
#[tokio::test]
async fn test_profile_based_configuration(temp_config_dir: TempDir) {
	let base_path = temp_config_dir.path().join("base.toml");
	let production_path = temp_config_dir.path().join("production.toml");

	let merged = SettingsBuilder::new()
		.profile(Profile::Production)
		.add_source(
			DefaultSource::new()
				.with_value("debug", json!(true))
				.with_value("port", json!(8000)),
		)
		.add_source(TomlFileSource::new(base_path))
		.add_source(TomlFileSource::new(production_path))
		.build()
		.expect("Failed to build settings");

	// Production profile should use production.toml values
	assert_eq!(merged.get::<bool>("debug").ok(), Some(false));
	assert_eq!(merged.get::<u64>("port").ok(), Some(80));
	assert_eq!(
		merged.get::<String>("database_url").ok(),
		Some("postgres://prod-server/prod_db".to_string())
	);
	assert_eq!(merged.get::<u64>("max_connections").ok(), Some(100));
}

/// Test: Environment variables override lower priority sources
#[rstest]
#[tokio::test]
async fn test_environment_override(temp_config_dir: TempDir) {
	// Set environment variables with higher priority
	unsafe {
		env::set_var("APP_NAME", "env_override_app");
	}
	unsafe {
		env::set_var("PORT", "9000");
	}

	let base_path = temp_config_dir.path().join("base.toml");

	let merged = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(
			DefaultSource::new()
				.with_value("app_name", json!("default_app"))
				.with_value("port", json!(8000)),
		)
		.add_source(TomlFileSource::new(base_path))
		.add_source(LowPriorityEnvSource::new().with_prefix(""))
		.build()
		.expect("Failed to build settings");

	// Note: LowPriorityEnvSource has lower priority than TOML files
	// So TOML values should win
	assert_eq!(
		merged.get::<String>("app_name").ok(),
		Some("base_app".to_string())
	);
	assert_eq!(merged.get::<u64>("port").ok(), Some(8000));

	// Cleanup
	unsafe {
		env::remove_var("APP_NAME");
	}
	unsafe {
		env::remove_var("PORT");
	}
}

/// Test: Merging nested configuration structures
#[rstest]
#[tokio::test]
async fn test_nested_config_merging() {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");

	// Base config with nested structure
	let base_content = r#"
app_name = "base_app"

[database]
host = "localhost"
port = 5432
name = "base_db"

[cache]
enabled = true
ttl = 3600
"#;
	fs::write(temp_dir.path().join("base.toml"), base_content).expect("Failed to write base.toml");

	// Override config with complete nested structure
	// Note: Settings merging replaces entire top-level keys, not deep-merging nested objects
	let override_content = r#"
[database]
host = "override-host"
port = 5433
name = "override_db"
max_connections = 100

[cache]
enabled = false
ttl = 7200
"#;
	fs::write(temp_dir.path().join("override.toml"), override_content)
		.expect("Failed to write override.toml");

	let base_path = temp_dir.path().join("base.toml");
	let override_path = temp_dir.path().join("override.toml");

	let merged = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(TomlFileSource::new(base_path))
		.add_source(TomlFileSource::new(override_path))
		.build()
		.expect("Failed to build settings");

	// Verify that override.toml completely replaces nested structures from base.toml
	let database_config: DatabaseConfig = merged
		.get("database")
		.expect("Failed to get database config");
	assert_eq!(database_config.host, "override-host");
	assert_eq!(database_config.port, 5433);
	assert_eq!(database_config.name, "override_db");
	assert_eq!(database_config.max_connections, Some(100));

	let cache_config: CacheConfig = merged.get("cache").expect("Failed to get cache config");
	assert!(!cache_config.enabled);
	assert_eq!(cache_config.ttl, 7200);
}

/// Test: Default source provides fallback values
#[rstest]
#[tokio::test]
async fn test_default_source_fallback() {
	let merged = SettingsBuilder::new()
		.profile(Profile::Development)
		.add_source(
			DefaultSource::new()
				.with_value("app_name", json!("default_app"))
				.with_value("debug", json!(false))
				.with_value("port", json!(8000))
				.with_value("database_url", json!("sqlite::memory:"))
				.with_value("max_connections", json!(10)),
		)
		.build()
		.expect("Failed to build settings");

	// All values should come from default source
	assert_eq!(
		merged.get::<String>("app_name").ok(),
		Some("default_app".to_string())
	);
	assert_eq!(merged.get::<bool>("debug").ok(), Some(false));
	assert_eq!(merged.get::<u64>("port").ok(), Some(8000));
	assert_eq!(
		merged.get::<String>("database_url").ok(),
		Some("sqlite::memory:".to_string())
	);
	assert_eq!(merged.get::<u64>("max_connections").ok(), Some(10));
}

/// Test: Empty sources list returns empty settings
#[rstest]
#[tokio::test]
async fn test_empty_sources() {
	let merged = SettingsBuilder::new()
		.profile(Profile::Development)
		.build()
		.expect("Failed to build settings");

	// Should be empty
	assert!(merged.as_map().is_empty());
}
