//! Fuzz tests for Configuration Merging Logic.
//!
//! This test module uses quickcheck to generate random JSON/TOML configurations
//! and verify that merging logic never panics, produces valid output, and does not
//! lose data unexpectedly.
//!
//! ## Testing Strategy
//!
//! - Generate random JSON structures with varying depths, types, and null values
//! - Test merging with different priority combinations
//! - Verify no panics during merge operations
//! - Verify merged result is valid JSON
//! - Verify data integrity (no unexpected data loss)

use quickcheck::{Arbitrary, Gen};
use quickcheck_macros::quickcheck;
use reinhardt_settings::builder::SettingsBuilder;
use reinhardt_settings::sources::DefaultSource;
use rstest::*;
use serde_json::{Value, json};

/// Test: Configuration merging with random strings never panics
///
/// Why: Validates that merging arbitrary key-value pairs never causes panic.
#[quickcheck]
fn quickcheck_merge_random_strings(
	key1: String,
	value1: String,
	key2: String,
	value2: String,
) -> bool {
	// Skip if keys contain dots (nested keys not supported in this test)
	if key1.contains('.') || key2.contains('.') || key1.is_empty() || key2.is_empty() {
		return true;
	}

	let source1 = DefaultSource::default().with_value(&key1, json!(value1));
	let source2 = DefaultSource::default().with_value(&key2, json!(value2));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	// Should not panic, result is Ok or Err
	result.is_ok() || result.is_err()
}

/// Test: Merging with integers never panics
///
/// Why: Validates integer value merging robustness.
#[quickcheck]
fn quickcheck_merge_integers(key: String, value1: i32, value2: i32) -> bool {
	if key.contains('.') || key.is_empty() {
		return true;
	}

	let source1 = DefaultSource::default().with_value(&key, json!(value1));
	let source2 = DefaultSource::default().with_value(&key, json!(value2));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	if let Ok(merged) = result {
		// Last value should win
		if let Ok(result_value) = merged.get::<i32>(&key) {
			result_value == value2
		} else {
			false
		}
	} else {
		true // Graceful error is acceptable
	}
}

/// Test: Merging with booleans never panics
///
/// Why: Validates boolean value merging robustness.
#[quickcheck]
fn quickcheck_merge_booleans(key: String, value1: bool, value2: bool) -> bool {
	if key.contains('.') || key.is_empty() {
		return true;
	}

	let source1 = DefaultSource::default().with_value(&key, json!(value1));
	let source2 = DefaultSource::default().with_value(&key, json!(value2));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<bool>(&key) {
			result_value == value2
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Empty source merging
///
/// Why: Validates edge case where empty sources are merged.
#[rstest]
#[test]
fn test_merge_empty_sources() {
	let source1 = DefaultSource::default();
	let source2 = DefaultSource::default();
	let source3 = DefaultSource::default();

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.add_source(source3)
		.build();

	assert!(result.is_ok(), "Merging empty sources should succeed");

	if let Ok(merged) = result {
		assert!(
			merged.as_map().is_empty(),
			"Merging empty sources should result in empty settings"
		);
	}
}

/// Test: Large number of sources
///
/// Why: Validates scalability of merging logic with many sources.
#[rstest]
#[test]
fn test_merge_many_sources() {
	let mut builder = SettingsBuilder::new();

	// Add 100 sources with different keys
	for i in 0..100 {
		let key = format!("key_{}", i);
		let source = DefaultSource::default().with_value(&key, json!(i));
		builder = builder.add_source(source);
	}

	let result = builder.build();

	assert!(result.is_ok(), "Merging many sources should succeed");

	if let Ok(merged) = result {
		// Verify all keys are present
		for i in 0..100 {
			let key = format!("key_{}", i);
			assert!(merged.get::<i32>(&key).is_ok(), "All keys should be merged");
		}
	}
}

/// Test: Type conflicts during merging
///
/// Why: Validates behavior when same key has different types across sources.
#[rstest]
#[case(json!("string"), json!(123))]
#[case(json!(true), json!({"nested": "object"}))]
#[case(json!([1, 2, 3]), json!(false))]
#[case(json!({"key": "value"}), json!("replaced_with_string"))]
fn test_type_conflicts_during_merge(#[case] value1: Value, #[case] value2: Value) {
	let key = "conflict_key";

	let source1 = DefaultSource::default().with_value(key, value1);
	let source2 = DefaultSource::default().with_value(key, value2.clone());

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(
		result.is_ok(),
		"Merging with type conflicts should not panic"
	);

	// Last source should win
	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<Value>(key) {
			assert_eq!(
				result_value, value2,
				"Last source value should override type conflict"
			);
		}
	}
}

/// Test: Array replacement behavior
///
/// Why: Validates that arrays are replaced completely, not merged element-wise.
#[quickcheck]
fn quickcheck_array_replacement(key: String) -> bool {
	if key.is_empty() || key.contains('.') {
		return true;
	}

	let array1 = json!([1, 2, 3]);
	let array2 = json!([4, 5, 6]);

	let source1 = DefaultSource::default().with_value(&key, array1);
	let source2 = DefaultSource::default().with_value(&key, array2.clone());

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<Value>(&key) {
			result_value == array2
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Merging with null values
///
/// Why: Validates that null values in later sources correctly override
/// non-null values from earlier sources.
#[quickcheck]
fn quickcheck_merge_with_null_values(key: String, non_null_value: i32) -> bool {
	if key.is_empty() || key.contains('.') {
		return true;
	}

	let source1 = DefaultSource::default().with_value(&key, json!(non_null_value));
	let source2 = DefaultSource::default().with_value(&key, Value::Null);

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<Value>(&key) {
			// Null should override non-null
			result_value == Value::Null
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Deeply nested object merging
///
/// Why: Validates merging behavior with deeply nested JSON structures.
#[rstest]
#[test]
fn test_deeply_nested_merging() {
	// Create 5-level nested structure
	let nested_value = json!({
		"level_0": {
			"level_1": {
				"level_2": {
					"level_3": {
						"level_4": "leaf_value"
					}
				}
			}
		}
	});

	let source = DefaultSource::default().with_value("root", nested_value.clone());

	let result = SettingsBuilder::new().add_source(source).build();

	assert!(result.is_ok(), "Deeply nested merging should succeed");

	if let Ok(merged) = result {
		let result_value = merged.get::<Value>("root");
		assert!(result_value.is_ok(), "Should retrieve nested value");
		assert_eq!(
			result_value.unwrap(),
			nested_value,
			"Nested structure should be preserved"
		);
	}
}

/// Test: Priority-based merging
///
/// Why: Validates that last source wins when keys conflict.
#[quickcheck]
fn quickcheck_priority_based_merging(key: String, value1: i32, value2: i32) -> bool {
	if key.is_empty() || key.contains('.') {
		return true;
	}

	let source1 = DefaultSource::default().with_value(&key, json!(value1));
	let source2 = DefaultSource::default().with_value(&key, json!(value2));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<i32>(&key) {
			// Last source should win
			result_value == value2
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Merged configuration is serializable to JSON
///
/// Why: Validates that merge result maintains valid JSON structure.
#[rstest]
#[test]
fn test_merged_config_serializable() {
	let source1 = DefaultSource::default()
		.with_value("key1", json!("value1"))
		.with_value("key2", json!(42));

	let source2 = DefaultSource::default()
		.with_value("key2", json!(99))
		.with_value("key3", json!(true));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	assert!(result.is_ok(), "Build should succeed");

	if let Ok(merged) = result {
		let map = merged.as_map();
		let json_result = serde_json::to_string(&map);
		assert!(
			json_result.is_ok(),
			"Merged config should be serializable to JSON"
		);

		// Verify serialized JSON is valid
		if let Ok(json_str) = json_result {
			let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
			assert!(parsed.is_ok(), "Serialized JSON should be parseable");
		}
	}
}

/// Custom generator for safe keys (no dots, not empty)
#[derive(Clone, Debug)]
struct SafeKey(String);

impl Arbitrary for SafeKey {
	fn arbitrary(g: &mut Gen) -> Self {
		loop {
			let s = String::arbitrary(g);
			if !s.is_empty() && !s.contains('.') && s.len() < 100 {
				return SafeKey(s);
			}
		}
	}
}

/// Test: Merging with generated safe keys
///
/// Why: Tests merging with randomly generated valid keys.
#[quickcheck]
fn quickcheck_merge_safe_keys(key1: SafeKey, key2: SafeKey, value1: i32, value2: i32) -> bool {
	let source1 = DefaultSource::default().with_value(&key1.0, json!(value1));
	let source2 = DefaultSource::default().with_value(&key2.0, json!(value2));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build();

	result.is_ok() || result.is_err() // Should not panic
}

/// Test: Concurrent merging safety
///
/// Why: Validates that SettingsBuilder is safe to use from multiple threads.
#[rstest]
#[test]
fn test_concurrent_merging() {
	use std::sync::Arc;
	use std::thread;

	let test_data = Arc::new(vec![
		("key1", json!("value1")),
		("key2", json!(42)),
		("key3", json!(true)),
		("key4", json!({"nested": "value"})),
	]);

	let mut handles = vec![];

	for _ in 0..10 {
		let data = test_data.clone();
		let handle = thread::spawn(move || {
			for (key, value) in data.iter() {
				let source = DefaultSource::default().with_value(*key, value.clone());
				let _merged = SettingsBuilder::new().add_source(source).build();
			}
		});
		handles.push(handle);
	}

	for handle in handles {
		handle.join().expect("Thread should not panic");
	}
}

/// Test: Very long keys and values
///
/// Why: Validates handling of extremely long configuration data.
#[rstest]
#[test]
fn test_very_long_keys_and_values() {
	let long_key = "k".repeat(1000);
	let long_value = "v".repeat(10000);

	let source = DefaultSource::default().with_value(&long_key, json!(long_value.clone()));

	let result = SettingsBuilder::new().add_source(source).build();

	assert!(result.is_ok(), "Long keys and values should be handled");

	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<String>(&long_key) {
			assert_eq!(result_value, long_value, "Long value should be preserved");
		}
	}
}

/// Test: Special characters in keys
///
/// Why: Validates handling of Unicode and special characters in configuration keys.
#[rstest]
#[case("emoji😀")]
#[case("日本語")]
#[case("кириллица")]
#[case("key_with_underscore")]
#[case("key-with-dash")]
#[case("key123")]
fn test_special_characters_in_keys(#[case] key: &str) {
	let source = DefaultSource::default().with_value(key, json!("value"));

	let result = SettingsBuilder::new().add_source(source).build();

	assert!(
		result.is_ok(),
		"Special characters in keys should be handled"
	);

	if let Ok(merged) = result {
		let result_value = merged.get::<String>(key);
		assert!(result_value.is_ok(), "Should retrieve value by special key");
	}
}

/// Test: Merging identical keys with identical values
///
/// Why: Validates idempotency - merging same key-value multiple times.
#[quickcheck]
fn quickcheck_idempotent_merging(key: SafeKey, value: i32) -> bool {
	let source1 = DefaultSource::default().with_value(&key.0, json!(value));
	let source2 = DefaultSource::default().with_value(&key.0, json!(value));
	let source3 = DefaultSource::default().with_value(&key.0, json!(value));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.add_source(source3)
		.build();

	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<i32>(&key.0) {
			result_value == value
		} else {
			false
		}
	} else {
		true
	}
}
