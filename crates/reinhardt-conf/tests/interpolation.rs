// Integration tests for TOML interpolation via TomlFileSource.
//
// These tests mutate `std::env`, so they MUST run under
// `#[serial(env)]` (matching `tests/source_priority.rs`). The `EnvGuard`
// ensures cleanup even on panic.

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
#[serial(env)]
fn with_interpolation_resolves_env_var() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_DB_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::set_var("IT_DB_HOST", "production-db") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_DB_HOST}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation())
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "production-db");
}

#[rstest]
#[serial(env)]
fn without_interpolation_preserves_literal_pattern() {
	// Arrange — explicit opt-out keeps `${...}` strings verbatim
	let (_dir, path) = write_toml_file(r#"host = "${SHOULD_NOT_EXPAND}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).without_interpolation())
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "${SHOULD_NOT_EXPAND}");
}

#[rstest]
#[serial(env)]
fn default_constructor_interpolates_var() {
	// Arrange — new default in 0.1.0-rc.27: interpolation is ON without
	// any builder method. Issue #4224.
	let _guard = EnvGuard(vec!["IT_DEFAULT_ON_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::set_var("IT_DEFAULT_ON_HOST", "default-on-host") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_DEFAULT_ON_HOST}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path)) // no interpolation method
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "default-on-host");
}

#[rstest]
#[serial(env)]
#[allow(deprecated)] // exercising the deprecated set_interpolation path on purpose
fn deprecated_set_interpolation_still_works() {
	// Arrange — set_interpolation(bool) is deprecated since 0.1.0-rc.27
	// but must remain functional until removal in 0.2.0 (issue #4224).
	let _guard = EnvGuard(vec!["IT_DEPRECATED_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::set_var("IT_DEPRECATED_HOST", "legacy-host") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_DEPRECATED_HOST}""#);
	let (_dir2, path2) = write_toml_file(r#"host = "${IT_DEPRECATED_HOST}""#);

	// Act — true keeps interpolation on; false opts out and preserves literal
	let on: String = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).set_interpolation(true))
		.build()
		.unwrap()
		.get("host")
		.unwrap();
	let off: String = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path2).set_interpolation(false))
		.build()
		.unwrap()
		.get("host")
		.unwrap();

	// Assert
	assert_eq!(on, "legacy-host");
	assert_eq!(off, "${IT_DEPRECATED_HOST}");
}

#[rstest]
#[serial(env)]
fn default_value_used_when_var_unset() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_UNSET_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::remove_var("IT_UNSET_HOST") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_UNSET_HOST:-fallback-host}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation())
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "fallback-host");
}

#[rstest]
#[serial(env)]
fn default_value_used_when_var_empty() {
	// Arrange — strict-empty contract
	let _guard = EnvGuard(vec!["IT_EMPTY_HOST"]);
	// SAFETY: serial-protected.
	unsafe { env::set_var("IT_EMPTY_HOST", "") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_EMPTY_HOST:-fallback}""#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation())
		.build()
		.unwrap();

	// Assert
	let host: String = settings.get("host").unwrap();
	assert_eq!(host, "fallback");
}

#[rstest]
#[serial(env)]
fn required_var_unset_propagates_source_error() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_REQUIRED_VAR"]);
	// SAFETY: serial-protected.
	unsafe { env::remove_var("IT_REQUIRED_VAR") };
	let (_dir, path) = write_toml_file(r#"host = "${IT_REQUIRED_VAR}""#);

	// Act
	let result = TomlFileSource::new(&path).with_interpolation().load();

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
#[serial(env)]
fn required_with_message_surface_user_message() {
	// Arrange
	let _guard = EnvGuard(vec!["IT_NEEDS_MESSAGE"]);
	// SAFETY: serial-protected.
	unsafe { env::remove_var("IT_NEEDS_MESSAGE") };
	let (_dir, path) =
		write_toml_file(r#"password = "${IT_NEEDS_MESSAGE:?Set via direnv or 1Password CLI}""#);

	// Act
	let result = TomlFileSource::new(&path).with_interpolation().load();

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
#[serial(env)]
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
	let source = TomlFileSource::new(&path).with_interpolation();
	let config = source.load().unwrap();

	// Assert — top-level "database" object holds resolved children
	let db = config.get("database").unwrap();
	assert_eq!(db["host"], serde_json::json!("postgres"));
	assert_eq!(db["port"], serde_json::json!("5433"));
	assert_eq!(db["engine"], serde_json::json!("postgresql"));
}

// Regression coverage for issue #4235: end-to-end pipeline
// `[database].host = "${VAR:-default}"` → SettingsBuilder merge
// → DatabaseConfig deserialize → DatabaseConfig::to_url(). The
// pre-existing `nested_table_interpolation_resolves_keys` covers
// only the TomlFileSource::load() return value, but the actual
// `manage migrate` path goes through SettingsBuilder.build() and
// then constructs the DATABASE_URL from a typed DatabaseConfig.
// A regression here would let a literal `${...}` reach DATABASE_URL
// and trip the URL parser with a misleading "invalid port number".

#[rstest]
#[serial(env)]
fn nested_db_config_through_settings_builder_resolves_db_host() {
	// Arrange — env override takes effect; literal `${...}` must not
	// reach the constructed DATABASE_URL.
	let _guard = EnvGuard(vec!["IT_DB_HOST_E2E"]);
	// SAFETY: serial-protected.
	unsafe { env::set_var("IT_DB_HOST_E2E", "postgres") };
	let (_dir, path) = write_toml_file(
		r#"
		[database]
		engine = "postgresql"
		name = "appdb"
		user = "app"
		password = "secret"
		host = "${IT_DB_HOST_E2E:-localhost}"
		port = 5432
		"#,
	);

	// Act
	let merged = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation())
		.build()
		.unwrap();
	let db_value = merged
		.get_raw("database")
		.expect("database key present after merge");
	let db_config: reinhardt_conf::settings::DatabaseConfig =
		serde_json::from_value(db_value.clone())
			.expect("database deserializes into DatabaseConfig");
	let url = db_config.to_url();

	// Assert — `${IT_DB_HOST_E2E:-localhost}` is fully resolved before
	// reaching DATABASE_URL. The env value (`postgres`) wins over the
	// `:-localhost` fallback.
	assert!(
		!url.contains("${"),
		"DATABASE_URL still contains a literal interpolation token: {url}",
	);
	assert!(
		url.contains("@postgres:5432/"),
		"DATABASE_URL host segment is not the env-resolved value: {url}",
	);
}

#[rstest]
#[serial(env)]
fn nested_db_config_falls_back_to_default_when_var_unset() {
	// Companion to the env-set case: with the variable cleared, the
	// `:-fallback` default must reach the URL — confirming that the
	// fallback branch is never bypassed by the SettingsBuilder merge.

	// Arrange
	let _guard = EnvGuard(vec!["IT_DB_HOST_E2E_UNSET"]);
	// SAFETY: serial-protected.
	unsafe { env::remove_var("IT_DB_HOST_E2E_UNSET") };
	let (_dir, path) = write_toml_file(
		r#"
		[database]
		engine = "postgresql"
		name = "appdb"
		user = "app"
		password = "secret"
		host = "${IT_DB_HOST_E2E_UNSET:-fallback-host}"
		port = 5432
		"#,
	);

	// Act
	let merged = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path).with_interpolation())
		.build()
		.unwrap();
	let db_value = merged.get_raw("database").expect("database key present");
	let db_config: reinhardt_conf::settings::DatabaseConfig =
		serde_json::from_value(db_value.clone()).expect("deserializes into DatabaseConfig");
	let url = db_config.to_url();

	// Assert
	assert!(
		!url.contains("${"),
		"DATABASE_URL still contains a literal interpolation token: {url}",
	);
	assert!(
		url.contains("@fallback-host:5432/"),
		"DATABASE_URL host did not resolve to `:-default` fallback: {url}",
	);
}
