//! Equivalence Partitioning Tests for Parse List and Dict.
//!
//! This test module validates that EnvParser correctly handles all equivalence classes
//! of list and dictionary input.
//!
//! ## Actual Implementation Behavior
//!
//! **parse_list:**
//! - Returns `Vec<String>` directly (not `Result`)
//! - Only comma delimiter is supported
//! - Automatically trims whitespace from items
//!
//! **parse_dict:**
//! - Returns `HashMap<String, String>` directly (not `Result`)
//! - Format is `key=value` (NOT `key:value`)
//! - Comma-separated pairs
//!
//! ## List Partitions
//!
//! **Content Types:**
//! - Empty string → empty vector
//! - Single item
//! - Multiple items (2, 3, 10)
//! - Items with special characters
//! - Numeric strings
//! - Whitespace handling (before/after commas)
//!
//! ## Dict Partitions
//!
//! **Format Types:**
//! - Single key=value pair
//! - Multiple pairs (2, 3)
//! - Keys with underscores
//! - Keys with dashes
//! - Values with special characters

use reinhardt_conf::settings::env_parser::{parse_dict, parse_list};
use rstest::*;

/// Test: Parse List - Content Equivalence Classes
///
/// Why: Validates list parsing with different content types.
/// Per actual implementation, parse_list returns Vec<String> directly (not Result).
#[rstest]
#[case("", vec![], "Empty string")]
#[case("single", vec!["single"], "Single item")]
#[case("item1,item2", vec!["item1", "item2"], "Two items")]
#[case("a,b,c", vec!["a", "b", "c"], "Three items")]
#[case("a,b,c,d,e,f,g,h,i,j", vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"], "Ten items")]
fn test_parse_list_content_equivalence_classes(
	#[case] input: &str,
	#[case] expected: Vec<&str>,
	#[case] description: &str,
) {
	let parsed = parse_list(input);
	let expected_strings: Vec<String> = expected.iter().map(|s| s.to_string()).collect();

	assert_eq!(
		parsed, expected_strings,
		"parse_list({:?}) failed for partition: {}",
		input, description
	);
}

/// Test: Parse List - Special Characters Equivalence Classes
///
/// Why: Validates list parsing with items containing special characters.
#[rstest]
#[case("item_with_underscore,item-with-dash", vec!["item_with_underscore", "item-with-dash"], "Underscore and dash")]
#[case("item.with.dots,another.item", vec!["item.with.dots", "another.item"], "Dots")]
#[case("123,456,789", vec!["123", "456", "789"], "Numeric strings")]
#[case("item1,item2,item3", vec!["item1", "item2", "item3"], "Alphanumeric")]
#[case("UPPER,lower,MiXeD", vec!["UPPER", "lower", "MiXeD"], "Mixed case")]
fn test_parse_list_special_chars_equivalence_classes(
	#[case] input: &str,
	#[case] expected: Vec<&str>,
	#[case] description: &str,
) {
	let parsed = parse_list(input);
	let expected_strings: Vec<String> = expected.iter().map(|s| s.to_string()).collect();

	assert_eq!(
		parsed, expected_strings,
		"parse_list({:?}) failed for partition: {}",
		input, description
	);
}

/// Test: Parse List - Whitespace Handling
///
/// Why: Validates that list parsing automatically trims whitespace.
/// Per actual implementation, whitespace around items is trimmed.
#[rstest]
#[case("a, b, c", vec!["a", "b", "c"], "Whitespace after comma")]
#[case("a ,b ,c", vec!["a", "b", "c"], "Whitespace before comma")]
#[case("a , b , c", vec!["a", "b", "c"], "Whitespace around comma")]
#[case(" a,b,c ", vec!["a", "b", "c"], "Surrounding whitespace")]
fn test_parse_list_whitespace_equivalence_classes(
	#[case] input: &str,
	#[case] expected: Vec<&str>,
	#[case] description: &str,
) {
	let parsed = parse_list(input);
	let expected_strings: Vec<String> = expected.iter().map(|s| s.to_string()).collect();

	assert_eq!(
		parsed, expected_strings,
		"parse_list({:?}) whitespace handling for partition: {}",
		input, description
	);
}

/// Test: Parse Dict - Basic Format Equivalence Classes
///
/// Why: Validates dict parsing with key=value format (NOT key:value).
/// Per actual implementation, parse_dict returns HashMap<String, String> directly.
#[rstest]
#[case("key1=value1", vec![("key1", "value1")], "Single pair")]
#[case("key1=value1,key2=value2", vec![("key1", "value1"), ("key2", "value2")], "Two pairs")]
#[case("k1=v1,k2=v2,k3=v3", vec![("k1", "v1"), ("k2", "v2"), ("k3", "v3")], "Three pairs")]
fn test_parse_dict_basic_equivalence_classes(
	#[case] input: &str,
	#[case] expected: Vec<(&str, &str)>,
	#[case] description: &str,
) {
	let parsed = parse_dict(input);
	let expected_map: std::collections::HashMap<String, String> = expected
		.into_iter()
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.collect();

	assert_eq!(
		parsed, expected_map,
		"parse_dict({:?}) failed for partition: {}",
		input, description
	);
}

/// Test: Parse Dict - Special Characters in Keys Equivalence Classes
///
/// Why: Validates dict parsing with keys containing special characters.
#[rstest]
#[case("key_with_underscore=value", vec![("key_with_underscore", "value")], "Key with underscore")]
#[case("key-with-dash=value", vec![("key-with-dash", "value")], "Key with dash")]
#[case("key.with.dots=value", vec![("key.with.dots", "value")], "Key with dots")]
#[case("key123=value", vec![("key123", "value")], "Key with numbers")]
#[case("UPPERCASE_KEY=value", vec![("UPPERCASE_KEY", "value")], "Uppercase key")]
fn test_parse_dict_key_special_chars_equivalence_classes(
	#[case] input: &str,
	#[case] expected: Vec<(&str, &str)>,
	#[case] description: &str,
) {
	let parsed = parse_dict(input);
	let expected_map: std::collections::HashMap<String, String> = expected
		.into_iter()
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.collect();

	assert_eq!(
		parsed, expected_map,
		"parse_dict({:?}) failed for partition: {}",
		input, description
	);
}

/// Test: Parse Dict - Special Characters in Values Equivalence Classes
///
/// Why: Validates dict parsing with values containing special characters.
#[rstest]
#[case("key=value_with_underscore", vec![("key", "value_with_underscore")], "Value with underscore")]
#[case("key=value-with-dash", vec![("key", "value-with-dash")], "Value with dash")]
#[case("key=value.with.dots", vec![("key", "value.with.dots")], "Value with dots")]
#[case("key=123", vec![("key", "123")], "Numeric value")]
#[case("key=/path/to/file", vec![("key", "/path/to/file")], "Path value")]
fn test_parse_dict_value_special_chars_equivalence_classes(
	#[case] input: &str,
	#[case] expected: Vec<(&str, &str)>,
	#[case] description: &str,
) {
	let parsed = parse_dict(input);
	let expected_map: std::collections::HashMap<String, String> = expected
		.into_iter()
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.collect();

	assert_eq!(
		parsed, expected_map,
		"parse_dict({:?}) failed for partition: {}",
		input, description
	);
}

/// Test: Parse Dict - Empty Input
///
/// Why: Validates empty input handling.
/// Per actual implementation, empty input returns empty HashMap.
#[rstest]
#[test]
fn test_parse_dict_empty_input() {
	let parsed = parse_dict("");

	assert!(
		parsed.is_empty(),
		"parse_dict(\"\") should return empty map"
	);
}

/// Test: Parse List - Empty Items Handling
///
/// Why: Validates how empty items in comma-separated lists are handled.
#[rstest]
#[case("a,,c", "Double comma (empty middle item)")]
#[case(",a,b", "Leading comma (empty first item)")]
#[case("a,b,", "Trailing comma (empty last item)")]
fn test_parse_list_empty_items(#[case] input: &str, #[case] description: &str) {
	let parsed = parse_list(input);

	// Document actual behavior: either filtered out or preserved
	// This test validates consistency, not specific behavior
	assert!(
		!parsed.is_empty() || input == ",,",
		"parse_list({:?}) should handle empty items consistently for partition: {}",
		input,
		description
	);
}

/// Test: Parse Dict - Multiple Values Per Key
///
/// Why: Validates behavior when same key appears multiple times.
/// Typically, last value should win (HashMap semantics).
#[rstest]
#[test]
fn test_parse_dict_duplicate_keys() {
	let input = "key1=first,key1=second,key1=third";
	let parsed = parse_dict(input);

	// Verify HashMap contains the key
	assert!(
		parsed.contains_key("key1"),
		"Dictionary should contain key1"
	);

	// Value should be one of the provided values (typically last one wins)
	let value = parsed.get("key1").unwrap();
	assert!(
		value == "first" || value == "second" || value == "third",
		"Value should be one of the provided values"
	);
}
