// Integration tests for TOML interpolation via TomlFileSource.
//
// These tests mutate `std::env`, so they MUST run under
// `#[serial(env_vars)]`. The `EnvGuard` ensures cleanup even on panic.

use reinhardt_conf::settings::builder::SettingsBuilder;
use reinhardt_conf::settings::interpolation::InterpolationError;
use reinhardt_conf::settings::sources::{ConfigSource, SourceError, TomlFileSource};
use rstest::rstest;
use serial_test::serial;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Drop-based env-var cleanup. Removes named keys when the guard is
/// dropped, even on panic.
struct EnvGuard(Vec<&'static str>);

impl Drop for EnvGuard {
	fn drop(&mut self) {
		for key in &self.0 {
			// SAFETY: env mutation in tests is protected by #[serial].
			unsafe { env::remove_var(key) };
		}
	}
}

fn write_toml_file(content: &str) -> (TempDir, PathBuf) {
	let dir = TempDir::new().unwrap();
	let path = dir.path().join("config.toml");
	let mut file = std::fs::File::create(&path).unwrap();
	file.write_all(content.as_bytes()).unwrap();
	(dir, path)
}

#[rstest]
#[serial(env_vars)]
fn with_interpolation_resolves_env_var() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_DB_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::set_var("IT_DB_HOST", "production-db") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_DB_HOST}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation(true))
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "production-db");
}

#[rstest]
#[serial(env_vars)]
fn without_interpolation_preserves_literal_pattern() {
	// Arrange — back-compat regression: default constructor must NOT expand
	let (_dir, path) = write_toml_file(r#"host = "${SHOULD_NOT_EXPAND}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path))
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "${SHOULD_NOT_EXPAND}");
}

#[rstest]
#[serial(env_vars)]
fn default_value_used_when_var_unset() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_UNSET_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::remove_var("IT_UNSET_HOST") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_UNSET_HOST:-fallback-host}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation(true))
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "fallback-host");
}

#[rstest]
#[serial(env_vars)]
fn default_value_used_when_var_empty() {
	// Arrange — strict-empty contract
	let _guard = EnvGuard(vec!["IT_EMPTY_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::set_var("IT_EMPTY_HOST", "") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_EMPTY_HOST:-fallback}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation(true))
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "fallback");
}

#[rstest]
#[serial(env_vars)]
fn required_var_unset_propagates_source_error() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_REQUIRED_VAR"]);
	// SAFETY: serial-protected.
	unsafe { env::remove_var("IT_REQUIRED_VAR") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_REQUIRED_VAR}""#);

	// Act
	let result = TomlFileSource::new(&path).with_interpolation(true).load();

	// Assert
	let err = result.unwrap_err();
	match &err {
		SourceError::Interpolation(boxed) => {
			assert!(
				matches!(
					&**boxed,
					InterpolationError::Required { var, .. } if var == "IT_REQUIRED_VAR"
				),
				"expected Required {{var: \"IT_REQUIRED_VAR\"}}, got {:?}",
				**boxed
			);
		}
		other => panic!("expected Interpolation variant, got {:?}", other),
	}
	assert!(err.to_string().contains("IT_REQUIRED_VAR"));
}

#[rstest]
#[serial(env_vars)]
fn required_with_message_surface_user_message() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_NEEDS_MESSAGE"]);
	// SAFETY: serial-protected.
	unsafe { env::remove_var("IT_NEEDS_MESSAGE") };
	let (_dir, path) = write_toml_file(
		r#"password = "${IT_NEEDS_MESSAGE:?Set via direnv or 1Password CLI}""#,
	);

	// Act
	let result = TomlFileSource::new(&path).with_interpolation(true).load();

	// Assert
	let err = result.unwrap_err();
	match &err {
		SourceError::Interpolation(boxed) => {
			assert!(matches!(
				&**boxed,
				InterpolationError::RequiredWithMessage { message, .. }
					if message == "Set via direnv or 1Password CLI"
			));
		}
		other => panic!("expected Interpolation variant, got {:?}", other),
	}
}

#[rstest]
#[serial(env_vars)]
fn nested_table_interpolation_resolves_keys() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_DB_HOST_NESTED", "IT_DB_PORT_NESTED"]);
	// SAFETY: serial-protected.
	unsafe {
		env::set_var("IT_DB_HOST_NESTED", "postgres");
		env::set_var("IT_DB_PORT_NESTED", "5433");
	}
	let (_dir, path) = write_toml_file(
		r#"
		[database]
		host = "${IT_DB_HOST_NESTED:-localhost}"
		port = "${IT_DB_PORT_NESTED:-5432}"
		engine = "postgresql"
		"#,
	);

	// Act
	let source = TomlFileSource::new(&path).with_interpolation(true);
	let config = source.load().unwrap();

	// Assert — top-level "database" object holds resolved children
	let db = config.get("database").unwrap();
	assert_eq!(db["host"], serde_json::json!("postgres"));
	assert_eq!(db["port"], serde_json::json!("5433"));
	assert_eq!(db["engine"], serde_json::json!("postgresql"));
}
