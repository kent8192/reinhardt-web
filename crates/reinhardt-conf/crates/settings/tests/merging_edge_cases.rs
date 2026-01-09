//! Integration tests for Configuration Merging Edge Cases.
//!
//! This test module validates that SettingsBuilder correctly handles edge cases
//! when merging configuration from multiple sources, including empty sources,
//! null values, arrays, and deeply nested objects.

use reinhardt_settings::builder::SettingsBuilder;
use reinhardt_settings::sources::DefaultSource;
use rstest::*;
use serde_json::{Value, json};

/// Test: Merge with all empty sources
///
/// Why: Validates that SettingsBuilder correctly handles the case where all
/// configuration sources are empty.
#[rstest]
#[test]
fn test_merge_empty_sources() {
	let result = SettingsBuilder::new()
		.add_source(DefaultSource::new())
		.add_source(DefaultSource::new())
		.add_source(DefaultSource::new())
		.build();

	assert!(result.is_ok(), "Building with empty sources should succeed");

	let merged = result.unwrap();
	assert_eq!(
		merged.as_map().len(),
		0,
		"Merging empty sources should result in empty configuration"
	);
}

/// Test: Merge with single source
///
/// Why: Validates that a single configuration source produces identical output
/// without any merging complications.
#[rstest]
#[test]
fn test_merge_single_source() {
	let source = DefaultSource::new()
		.with_value("app_name", Value::String("test_app".to_string()))
		.with_value("port", Value::Number(8080.into()))
		.with_value("debug", Value::Bool(true));

	let result = SettingsBuilder::new().add_source(source).build();

	assert!(result.is_ok(), "Building with single source should succeed");

	let merged = result.unwrap();
	assert_eq!(merged.as_map().len(), 3, "Should have 3 keys");
	assert_eq!(
		merged.get::<String>("app_name").unwrap(),
		"test_app",
		"app_name should be preserved"
	);
	assert_eq!(
		merged.get::<i64>("port").unwrap(),
		8080,
		"port should be preserved"
	);
	assert_eq!(
		merged.get::<bool>("debug").unwrap(),
		true,
		"debug should be preserved"
	);
}

/// Test: Null value override behavior
///
/// Why: Validates that when Source2 has a null value and Source1 has a value,
/// the null from Source2 overwrites the value from Source1 (last wins).
#[rstest]
#[test]
fn test_merge_null_values_override() {
	let source1 = DefaultSource::new().with_value("timeout", Value::Number(30.into()));

	let source2 = DefaultSource::new().with_value("timeout", Value::Null);

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	// Null value from source2 should override value from source1
	let timeout_value = merged.as_map().get("timeout");
	assert!(timeout_value.is_some(), "timeout key should exist");
	assert_eq!(
		timeout_value.unwrap(),
		&Value::Null,
		"Null from source2 should override value from source1"
	);
}

/// Test: Array replacement (not append)
///
/// Why: Validates that when merging arrays, the array from the higher priority
/// source completely replaces the array from the lower priority source
/// (not appended).
#[rstest]
#[test]
fn test_merge_array_replacement() {
	let source1 = DefaultSource::new().with_value("tags", json!(["tag1", "tag2"]));

	let source2 = DefaultSource::new().with_value("tags", json!(["tag3", "tag4"]));

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	let tags = merged.get::<Vec<String>>("tags").unwrap();
	assert_eq!(
		tags.len(),
		2,
		"Array should be replaced, not appended (should have 2 elements, not 4)"
	);
	assert_eq!(
		tags,
		vec!["tag3", "tag4"],
		"Array from source2 should replace array from source1"
	);
}

/// Test: Deeply nested object merging
///
/// Why: Validates correct merging behavior for objects nested 5 levels deep.
/// The entire nested object should be replaced, not merged key-by-key.
#[rstest]
#[test]
fn test_merge_deeply_nested_objects() {
	let source1 = DefaultSource::new().with_value(
		"database",
		json!({
			"connection": {
				"pool": {
					"settings": {
						"size": {
							"min": 1,
							"max": 10
						}
					}
				}
			}
		}),
	);

	let source2 = DefaultSource::new().with_value(
		"database",
		json!({
			"connection": {
				"pool": {
					"settings": {
						"size": {
							"min": 5,
							"max": 50
						},
						"timeout": 30
					}
				}
			}
		}),
	);

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	// Get the nested value
	let database_value = merged.as_map().get("database");
	assert!(database_value.is_some(), "database key should exist");

	// The entire object from source2 should replace the object from source1
	let expected = json!({
		"connection": {
			"pool": {
				"settings": {
					"size": {
						"min": 5,
						"max": 50
					},
					"timeout": 30
				}
			}
		}
	});

	assert_eq!(
		database_value.unwrap(),
		&expected,
		"Entire nested object from source2 should replace object from source1"
	);
}

/// Test: Mixed types for same key
///
/// Why: Validates that when different sources provide different types for the
/// same key, the last source wins (type replacement, not coercion).
#[rstest]
#[test]
fn test_merge_mixed_types() {
	let source1 = DefaultSource::new().with_value("value", Value::String("42".to_string()));

	let source2 = DefaultSource::new().with_value("value", Value::Number(42.into()));

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	// Type from source2 (Number) should replace type from source1 (String)
	let value = merged.as_map().get("value");
	assert!(value.is_some(), "value key should exist");
	assert!(
		value.unwrap().is_number(),
		"Value should be a number from source2, not string from source1"
	);
	assert_eq!(
		merged.get::<i64>("value").unwrap(),
		42,
		"Numeric value from source2 should be accessible"
	);
}

/// Test: Empty string vs null
///
/// Why: Validates that empty strings and null values are treated as distinct values.
#[rstest]
#[test]
fn test_merge_empty_string_vs_null() {
	let source1 = DefaultSource::new().with_value("description", Value::String("".to_string()));

	let source2 = DefaultSource::new().with_value("description", Value::Null);

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	let description = merged.as_map().get("description");
	assert!(description.is_some(), "description key should exist");
	assert_eq!(
		description.unwrap(),
		&Value::Null,
		"Null from source2 should override empty string from source1"
	);
}

/// Test: Partial overlap between sources
///
/// Why: Validates that when sources have partially overlapping keys,
/// all unique keys are preserved and overlapping keys use the last source.
#[rstest]
#[test]
fn test_merge_partial_overlap() {
	let source1 = DefaultSource::new()
		.with_value("key1", Value::String("value1".to_string()))
		.with_value("key2", Value::String("value2_source1".to_string()));

	let source2 = DefaultSource::new()
		.with_value("key2", Value::String("value2_source2".to_string()))
		.with_value("key3", Value::String("value3".to_string()));

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	assert_eq!(merged.as_map().len(), 3, "Should have 3 unique keys");

	// key1 from source1 (unique)
	assert_eq!(
		merged.get::<String>("key1").unwrap(),
		"value1",
		"key1 from source1 should be preserved"
	);

	// key2 from source2 (overlapping - last wins)
	assert_eq!(
		merged.get::<String>("key2").unwrap(),
		"value2_source2",
		"key2 from source2 should override source1"
	);

	// key3 from source2 (unique)
	assert_eq!(
		merged.get::<String>("key3").unwrap(),
		"value3",
		"key3 from source2 should be preserved"
	);
}

/// Test: Boolean value override
///
/// Why: Validates that boolean values are correctly overridden during merging.
#[rstest]
#[test]
fn test_merge_boolean_override() {
	let source1 = DefaultSource::new().with_value("debug", Value::Bool(true));

	let source2 = DefaultSource::new().with_value("debug", Value::Bool(false));

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	assert_eq!(
		merged.get::<bool>("debug").unwrap(),
		false,
		"Boolean false from source2 should override true from source1"
	);
}

/// Test: Object with array nested inside
///
/// Why: Validates merging behavior when objects contain arrays as nested values.
#[rstest]
#[test]
fn test_merge_object_with_nested_array() {
	let source1 = DefaultSource::new().with_value(
		"server",
		json!({
			"hosts": ["host1", "host2"],
			"port": 8080
		}),
	);

	let source2 = DefaultSource::new().with_value(
		"server",
		json!({
			"hosts": ["host3"],
			"timeout": 30
		}),
	);

	// Source2 has higher priority (added later)
	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Building should succeed");

	let merged = result.unwrap();

	// Entire object from source2 should replace object from source1
	let server = merged.as_map().get("server");
	assert!(server.is_some(), "server key should exist");

	let expected = json!({
		"hosts": ["host3"],
		"timeout": 30
	});

	assert_eq!(
		server.unwrap(),
		&expected,
		"Object from source2 should completely replace object from source1 (no key-by-key merge)"
	);
}
