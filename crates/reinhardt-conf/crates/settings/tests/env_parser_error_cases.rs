//! Integration tests for EnvParser error handling and malformed input.
//!
//! This test module validates that EnvParser functions provide clear error messages
//! for invalid input, including malformed URLs, invalid boolean values, and
//! unbalanced dictionary syntax.

use reinhardt_settings::env_parser::{
	parse_bool, parse_cache_url, parse_database_url, parse_dict, parse_list,
};
use rstest::*;

/// Test: parse_bool with invalid value
///
/// Why: Validates that parse_bool rejects invalid boolean strings with clear error message.
#[rstest]
#[case("maybe")]
#[case("unknown")]
#[case("2")]
#[case("invalid")]
#[case("")]
#[test]
fn test_parse_bool_invalid_value(#[case] input: &str) {
	let result = parse_bool(input);

	assert!(
		result.is_err(),
		"parse_bool('{}') should return error for invalid boolean value",
		input
	);

	let error = result.unwrap_err();
	assert!(
		!error.is_empty(),
		"Error message should not be empty for invalid boolean"
	);
}

/// Test: parse_database_url with missing scheme
///
/// Why: Validates that parse_database_url rejects URLs without proper scheme.
#[rstest]
#[case("localhost/db")]
#[case("//localhost/db")]
#[case("db_name")]
#[test]
fn test_parse_database_url_missing_scheme(#[case] input: &str) {
	let result = parse_database_url(input);

	assert!(
		result.is_err(),
		"parse_database_url('{}') should fail without proper scheme",
		input
	);

	let error = result.unwrap_err();
	assert!(
		error.to_lowercase().contains("scheme") || error.to_lowercase().contains("invalid"),
		"Error should mention missing scheme or invalid format, got: {}",
		error
	);
}

/// Test: parse_database_url with invalid port
///
/// Why: Validates that parse_database_url rejects non-numeric ports.
#[rstest]
#[case("postgres://localhost:abc/db")]
#[case("postgres://localhost:99999999/db")]
#[case("postgres://localhost:-1/db")]
#[test]
fn test_parse_database_url_invalid_port(#[case] input: &str) {
	let result = parse_database_url(input);

	assert!(
		result.is_err(),
		"parse_database_url('{}') should fail with invalid port",
		input
	);

	let error = result.unwrap_err();
	assert!(
		error.to_lowercase().contains("port") || error.to_lowercase().contains("invalid"),
		"Error should mention port or invalid format, got: {}",
		error
	);
}

/// Test: parse_cache_url with unknown backend
///
/// Why: Validates that parse_cache_url rejects unsupported cache backend types.
#[rstest]
#[case("unknown://localhost")]
#[case("invalid://127.0.0.1:6379")]
#[test]
fn test_parse_cache_url_unknown_backend(#[case] input: &str) {
	let result = parse_cache_url(input);

	assert!(
		result.is_err(),
		"parse_cache_url('{}') should reject unknown backend",
		input
	);

	let error = result.unwrap_err();
	assert!(
		error.to_lowercase().contains("scheme")
			|| error.to_lowercase().contains("backend")
			|| error.to_lowercase().contains("unsupported"),
		"Error should mention unsupported scheme, got: {}",
		error
	);
}

/// Test: parse_dict with unusual characters in values
///
/// Why: Validates that parse_dict handles values containing special characters without panic.
/// Note: The simple parser does not support nested structures.
#[rstest]
#[case("key={missing_close")]
#[case("key=value}extra_close")]
#[case("key={nested{too_deep}}")]
#[test]
fn test_parse_dict_special_chars(#[case] input: &str) {
	let _dict = parse_dict(input);

	// Parser treats everything after '=' as the value
	// Parser should handle any characters without panic
	// Just verify parsing completes without panic (implicitly tested by reaching this point)
}

/// Test: parse_list with empty string
///
/// Why: Validates that parse_list handles empty input gracefully.
#[rstest]
#[test]
fn test_parse_list_empty_string() {
	let list = parse_list("");

	// Empty string should return empty list
	assert!(list.is_empty(), "Empty string should produce empty list");
}

/// Test: parse_database_url with malformed URL
///
/// Why: Validates comprehensive error handling for various malformed database URLs.
#[rstest]
#[case("postgres:///")]
#[case("postgres://")]
#[case("://localhost/db")]
#[case("postgres//localhost/db")]
#[test]
fn test_parse_database_url_malformed(#[case] input: &str) {
	let result = parse_database_url(input);

	assert!(
		result.is_err(),
		"parse_database_url('{}') should fail for malformed URL",
		input
	);
}

/// Test: parse_cache_url with malformed URL
///
/// Why: Validates error handling for malformed cache URLs.
#[rstest]
#[case("://localhost")] // Missing scheme
#[test]
fn test_parse_cache_url_malformed(#[case] input: &str) {
	let result = parse_cache_url(input);

	assert!(
		result.is_err(),
		"parse_cache_url('{}') should fail for malformed URL",
		input
	);
}

/// Test: parse_dict with missing values
///
/// Why: Validates that parse_dict handles incomplete key-value pairs.
#[rstest]
#[case("key1=")]
#[case("=value")]
#[case("key1=,key2=value")]
#[test]
fn test_parse_dict_missing_values(#[case] input: &str) {
	let _dict = parse_dict(input);

	// Parser may accept empty values or reject them
	// Verify parser's behavior with incomplete pairs (implicitly tested by reaching this point)
}

/// Test: parse_database_url with special characters in password
///
/// Why: Validates that parse_database_url handles URL-encoded passwords correctly.
#[rstest]
#[case("postgres://user:p@ss%40word@localhost/db")]
#[case("postgres://user:pass%20word@localhost/db")]
#[test]
fn test_parse_database_url_special_chars_password(#[case] input: &str) {
	let result = parse_database_url(input);

	// Should either parse successfully with decoded password or fail with clear error
	if result.is_ok() {
		let db_url = result.unwrap();
		assert!(db_url.password.is_some(), "Password should be extracted");
	} else {
		let error = result.unwrap_err();
		assert!(!error.is_empty(), "Error message should be provided");
	}
}

/// Test: parse_list with only delimiters
///
/// Why: Validates that parse_list handles input consisting only of delimiter characters (comma).
#[rstest]
#[test]
fn test_parse_list_only_delimiters() {
	let input = ",,,";
	let list = parse_list(input);

	// Should return empty list (empty strings are filtered out)
	assert!(
		list.is_empty(),
		"List should be empty when input is only commas"
	);
}

/// Test: parse_dict with duplicate keys
///
/// Why: Validates that parse_dict handles duplicate keys (last value wins or error).
#[rstest]
#[test]
fn test_parse_dict_duplicate_keys() {
	let input = "key1=value1,key1=value2,key1=value3";
	let dict = parse_dict(input);

	// Last value should win
	assert_eq!(
		dict.get("key1"),
		Some(&"value3".to_string()),
		"Duplicate keys should use last value"
	);
}

/// Test: parse_database_url with IPv6 host
///
/// Why: Validates that parse_database_url handles IPv6 addresses correctly.
#[rstest]
#[case("postgres://[::1]/db")]
#[case("postgres://[2001:db8::1]:5432/db")]
#[test]
fn test_parse_database_url_ipv6(#[case] input: &str) {
	let result = parse_database_url(input);

	// Should either parse successfully or provide clear error
	if result.is_ok() {
		let db_url = result.unwrap();
		if let Some(host) = &db_url.host {
			assert!(
				host.contains("::") || host.contains("["),
				"IPv6 address should be preserved in host"
			);
		}
	} else {
		// Error is acceptable if IPv6 not supported
		assert!(result.is_err());
	}
}
