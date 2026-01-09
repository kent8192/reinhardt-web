//! Property-Based Tests for Configuration Source Priority Invariant.
//!
//! This test module validates that for sources S1 (priority P1) and S2 (priority P2)
//! where P2 > P1, if both define key K, merged result contains S2[K].
//!
//! ## Testing Strategy
//!
//! - Generate random sources with random priorities
//! - Verify priority always respected in merging
//! - Test with different data types (strings, integers, booleans)
//! - Verify "last source wins" invariant

use quickcheck_macros::quickcheck;
use reinhardt_settings::builder::SettingsBuilder;
use reinhardt_settings::sources::DefaultSource;
use rstest::*;
use serde_json::json;

/// Test: Last source wins for same key
///
/// Why: Validates that when multiple sources define the same key,
/// the value from the last-added source is used (priority-based merging).
#[quickcheck]
fn quickcheck_last_source_wins_string(key: String, value1: String, value2: String) -> bool {
	// Skip invalid keys
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
		if let Ok(result_value) = merged.get::<String>(&key) {
			// Last source (source2) should win
			result_value == value2
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Last source wins for integers
///
/// Why: Validates priority-based merging for integer values.
#[quickcheck]
fn quickcheck_last_source_wins_integer(key: String, value1: i32, value2: i32) -> bool {
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
			result_value == value2
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Last source wins for booleans
///
/// Why: Validates priority-based merging for boolean values.
#[quickcheck]
fn quickcheck_last_source_wins_boolean(key: String, value1: bool, value2: bool) -> bool {
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
		if let Ok(result_value) = merged.get::<bool>(&key) {
			result_value == value2
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Three sources - last wins
///
/// Why: Validates that with three sources, the last one always wins.
#[quickcheck]
fn quickcheck_three_sources_last_wins(key: String, value1: i32, value2: i32, value3: i32) -> bool {
	if key.is_empty() || key.contains('.') {
		return true;
	}

	let source1 = DefaultSource::default().with_value(&key, json!(value1));
	let source2 = DefaultSource::default().with_value(&key, json!(value2));
	let source3 = DefaultSource::default().with_value(&key, json!(value3));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.add_source(source3)
		.build();

	if let Ok(merged) = result {
		if let Ok(result_value) = merged.get::<i32>(&key) {
			// Third source should win
			result_value == value3
		} else {
			false
		}
	} else {
		true
	}
}

/// Test: Source order matters
///
/// Why: Validates that reversing source order changes the result
/// (order-dependent merging).
#[rstest]
#[test]
fn test_source_order_matters() {
	let key = "test_key";
	let value1 = "first_value";
	let value2 = "second_value";

	// Order 1: source1 then source2
	let source1_v1 = DefaultSource::default().with_value(key, json!(value1));
	let source2_v1 = DefaultSource::default().with_value(key, json!(value2));

	let result1 = SettingsBuilder::new()
		.add_source(source1_v1)
		.add_source(source2_v1)
		.build()
		.expect("Build should succeed");

	let result_value1 = result1.get::<String>(key).expect("Get should succeed");

	// Order 2: source2 then source1 (reversed)
	let source1_v2 = DefaultSource::default().with_value(key, json!(value1));
	let source2_v2 = DefaultSource::default().with_value(key, json!(value2));

	let result2 = SettingsBuilder::new()
		.add_source(source2_v2)
		.add_source(source1_v2)
		.build()
		.expect("Build should succeed");

	let result_value2 = result2.get::<String>(key).expect("Get should succeed");

	// Verify different results due to order
	assert_eq!(result_value1, value2, "First order should result in value2");
	assert_eq!(
		result_value2, value1,
		"Reversed order should result in value1"
	);
	assert_ne!(result_value1, result_value2, "Order should affect result");
}

/// Test: Empty source does not override non-empty
///
/// Why: Validates that empty sources do not affect values set by previous sources.
#[rstest]
#[test]
fn test_empty_source_no_override() {
	let key = "test_key";
	let value = "test_value";

	let source1 = DefaultSource::default().with_value(key, json!(value));
	let source2 = DefaultSource::default(); // Empty source

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build()
		.expect("Build should succeed");

	let result_value = result.get::<String>(key).expect("Get should succeed");

	assert_eq!(
		result_value, value,
		"Empty source should not override existing value"
	);
}

/// Test: Non-empty source overrides empty
///
/// Why: Validates that a non-empty source added after an empty source
/// correctly sets the value.
#[rstest]
#[test]
fn test_non_empty_overrides_empty() {
	let key = "test_key";
	let value = "test_value";

	let source1 = DefaultSource::default(); // Empty source
	let source2 = DefaultSource::default().with_value(key, json!(value));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build()
		.expect("Build should succeed");

	let result_value = result.get::<String>(key).expect("Get should succeed");

	assert_eq!(
		result_value, value,
		"Non-empty source should set value after empty source"
	);
}

/// Test: Multiple keys with different priorities
///
/// Why: Validates that different keys can have independent priority resolution
/// without interfering with each other.
#[rstest]
#[test]
fn test_multiple_keys_independent_priority() {
	let key1 = "key1";
	let key2 = "key2";

	// Source1: key1=A, key2=X
	let source1 = DefaultSource::default()
		.with_value(key1, json!("A"))
		.with_value(key2, json!("X"));

	// Source2: key1=B (no key2)
	let source2 = DefaultSource::default().with_value(key1, json!("B"));

	// Source3: key2=Y (no key1)
	let source3 = DefaultSource::default().with_value(key2, json!("Y"));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.add_source(source3)
		.build()
		.expect("Build should succeed");

	let value1 = result.get::<String>(key1).expect("Get key1 should succeed");
	let value2 = result.get::<String>(key2).expect("Get key2 should succeed");

	// key1: source2 (B) should win
	assert_eq!(value1, "B", "key1 should be from source2");

	// key2: source3 (Y) should win
	assert_eq!(value2, "Y", "key2 should be from source3");
}

/// Test: Type change via priority
///
/// Why: Validates that last source wins even when changing value type
/// (e.g., string to integer).
#[rstest]
#[case(json!("string_value"), json!(42))] // String to integer
#[case(json!(100), json!("replacement"))] // Integer to string
#[case(json!(true), json!(123))] // Boolean to integer
#[case(json!([1, 2, 3]), json!("array_replaced"))] // Array to string
fn test_type_change_priority(#[case] value1: serde_json::Value, #[case] value2: serde_json::Value) {
	let key = "test_key";

	let source1 = DefaultSource::default().with_value(key, value1);
	let source2 = DefaultSource::default().with_value(key, value2.clone());

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build()
		.expect("Build should succeed");

	let result_value = result
		.get::<serde_json::Value>(key)
		.expect("Get should succeed");

	assert_eq!(
		result_value, value2,
		"Last source should win even with type change"
	);
}

/// Test: Many sources - last always wins
///
/// Why: Validates that with many sources (100+), the last source always wins.
#[rstest]
#[test]
fn test_many_sources_last_wins() {
	let key = "test_key";
	let mut builder = SettingsBuilder::new();

	// Add 100 sources with values 0 to 99
	for i in 0..100 {
		let source = DefaultSource::default().with_value(key, json!(i));
		builder = builder.add_source(source);
	}

	let result = builder.build().expect("Build should succeed");
	let value = result.get::<i32>(key).expect("Get should succeed");

	assert_eq!(value, 99, "Last source (value 99) should win");
}

/// Test: Same source added twice
///
/// Why: Validates that adding the same source twice (with different values)
/// results in the second addition winning.
#[rstest]
#[test]
fn test_same_source_twice() {
	let key = "test_key";

	let source1 = DefaultSource::default().with_value(key, json!("first"));
	let source2 = DefaultSource::default().with_value(key, json!("second"));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build()
		.expect("Build should succeed");

	let value = result.get::<String>(key).expect("Get should succeed");

	assert_eq!(value, "second", "Second source should win");
}

/// Test: Priority with null values
///
/// Why: Validates that null values from later sources correctly override
/// non-null values from earlier sources.
#[rstest]
#[test]
fn test_priority_with_null_values() {
	let key = "test_key";

	let source1 = DefaultSource::default().with_value(key, json!("value"));
	let source2 = DefaultSource::default().with_value(key, json!(null));

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build()
		.expect("Build should succeed");

	let value = result
		.get::<serde_json::Value>(key)
		.expect("Get should succeed");

	assert_eq!(
		value,
		json!(null),
		"Null from last source should override non-null"
	);
}

/// Test: Priority invariant holds under concurrent operations
///
/// Why: Validates that priority invariant holds even when sources
/// are built concurrently.
#[rstest]
#[test]
fn test_priority_concurrent() {
	use std::thread;

	let key = "test_key";
	let mut handles = vec![];

	for i in 0..10 {
		let handle = thread::spawn(move || {
			let source1 = DefaultSource::default().with_value(key, json!(i * 2));
			let source2 = DefaultSource::default().with_value(key, json!(i * 2 + 1));

			let result = SettingsBuilder::new()
				.add_source(source1)
				.add_source(source2)
				.build()
				.unwrap();

			let value = result.get::<i32>(key).unwrap();

			// Last source should always win
			assert_eq!(value, i * 2 + 1);
		});
		handles.push(handle);
	}

	for handle in handles {
		handle.join().expect("Thread should not panic");
	}
}

/// Test: Priority with nested objects
///
/// Why: Validates that priority works correctly with nested object structures
/// (complete replacement, not deep merging).
#[rstest]
#[test]
fn test_priority_nested_objects() {
	let key = "config";

	let source1 = DefaultSource::default().with_value(
		key,
		json!({
			"database": {
				"host": "localhost",
				"port": 5432
			}
		}),
	);

	let source2 = DefaultSource::default().with_value(
		key,
		json!({
			"database": {
				"host": "production.example.com"
			}
		}),
	);

	let result = SettingsBuilder::new()
		.add_source(source1)
		.add_source(source2)
		.build()
		.expect("Build should succeed");

	let value = result
		.get::<serde_json::Value>(key)
		.expect("Get should succeed");

	// Last source should win with complete replacement
	assert_eq!(
		value["database"]["host"].as_str().unwrap(),
		"production.example.com"
	);

	// Note: port is NOT preserved (complete replacement, not deep merge)
	assert!(
		value["database"]["port"].is_null(),
		"Port should not be preserved (complete replacement)"
	);
}
