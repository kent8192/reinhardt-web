// Integration tests for TomlFileSource.
// Covers: real file reading, type parsing, missing/invalid files.

use reinhardt_conf::settings::builder::SettingsBuilder;
use reinhardt_conf::settings::sources::{ConfigSource, TomlFileSource};
use rstest::rstest;
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_toml_file(content: &str) -> (TempDir, PathBuf) {
	let dir = TempDir::new().unwrap();
	let path = dir.path().join("config.toml");
	let mut file = std::fs::File::create(&path).unwrap();
	file.write_all(content.as_bytes()).unwrap();
	(dir, path)
}

// ===========================================================================
// TomlFileSource – real file loading via builder
// ===========================================================================

#[rstest]
fn toml_source_loads_string_and_bool_via_builder() {
	// Arrange
	let (_dir, path) = write_toml_file(
		r#"
debug = true
secret_key = "abc"
"#,
	);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path))
		.build()
		.unwrap();

	// Assert
	let debug: bool = settings.get("debug").unwrap();
	assert!(debug);
	let key: String = settings.get("secret_key").unwrap();
	assert_eq!(key, "abc");
}

#[rstest]
fn toml_source_loads_integer_value() {
	// Arrange
	let (_dir, path) = write_toml_file("port = 8080\n");

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path))
		.build()
		.unwrap();

	// Assert
	let port: u16 = settings.get("port").unwrap();
	assert_eq!(port, 8080);
}

#[rstest]
fn toml_source_loads_array_value() {
	// Arrange
	let (_dir, path) = write_toml_file(r#"hosts = ["a.com", "b.com"]"#);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path))
		.build()
		.unwrap();

	// Assert
	let raw = settings.get_raw("hosts").unwrap();
	assert!(raw.is_array());
	let arr = raw.as_array().unwrap();
	assert_eq!(arr.len(), 2);
	assert_eq!(arr[0], Value::String("a.com".to_string()));
	assert_eq!(arr[1], Value::String("b.com".to_string()));
}

#[rstest]
fn toml_source_loads_nested_object() {
	// Arrange
	let (_dir, path) = write_toml_file(
		r#"
[database]
engine = "postgresql"
name = "mydb"
"#,
	);

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path))
		.build()
		.unwrap();

	// Assert
	let raw = settings.get_raw("database").unwrap();
	assert!(raw.is_object());
	let obj = raw.as_object().unwrap();
	assert_eq!(
		obj.get("engine").unwrap(),
		&Value::String("postgresql".to_string())
	);
	assert_eq!(obj.get("name").unwrap(), &Value::String("mydb".to_string()));
}

#[rstest]
fn toml_source_missing_file_returns_empty() {
	// Arrange
	let path = PathBuf::from("/tmp/nonexistent_reinhardt_config.toml");

	// Act
	let settings = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path))
		.build()
		.unwrap();

	// Assert
	assert_eq!(settings.keys().count(), 0);
}

#[rstest]
fn toml_source_invalid_content_returns_error() {
	// Arrange
	let (_dir, path) = write_toml_file("this is [[ not valid toml");

	// Act
	let result = SettingsBuilder::new()
		.add_source(TomlFileSource::new(&path))
		.build();

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn toml_source_priority_is_50() {
	// Assert
	assert_eq!(TomlFileSource::new("any.toml").priority(), 50);
}

#[rstest]
fn source_error_interpolation_variant_is_constructible() {
	// Arrange
	use reinhardt_conf::settings::interpolation::InterpolationError;
	use reinhardt_conf::settings::sources::SourceError;
	let inner = InterpolationError::Required {
		var: "X".into(),
		path: std::path::PathBuf::from("a.toml"),
		key_path: "x".into(),
	};

	// Act
	let err: SourceError = inner.into();

	// Assert — Display chains both messages
	let msg = err.to_string();
	assert!(msg.contains("Interpolation error"));
	assert!(msg.contains("X"));
}
