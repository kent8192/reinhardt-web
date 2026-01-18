//! Fuzz tests for EnvParser parsing functions.
//!
//! This test module uses quickcheck to generate random inputs and verify that
//! parsers never panic, even with malformed or adversarial input.
//!
//! ## Testing Strategy
//!
//! - Generate random strings of varying lengths and character sets
//! - Test parser functions with edge cases (empty, whitespace, special characters)
//! - Verify that all parsers return Result/Vec instead of panicking
//! - Focus on robustness over correctness (parsers should handle invalid input gracefully)

use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use quickcheck_macros::quickcheck;
use reinhardt_conf::settings::env_parser::{parse_bool, parse_database_url, parse_dict, parse_list};
use rstest::*;

/// Test: parse_bool never panics on random input
///
/// Why: Validates that parse_bool gracefully handles any string input
/// without crashing, returning Result::Err for invalid values.
#[rstest]
#[test]
fn test_fuzz_parse_bool() {
	fn property(input: String) -> TestResult {
		// parse_bool should never panic
		let result = std::panic::catch_unwind(|| parse_bool(&input));

		if result.is_err() {
			return TestResult::failed();
		}

		// Result should always be Ok(bool) or Err(String)
		let parse_result = result.unwrap();
		match parse_result {
			Ok(b) => {
				// Valid boolean values
				assert!(b || !b); // Tautology, but verifies bool type
				TestResult::passed()
			}
			Err(e) => {
				// Invalid input should return error, not panic
				assert!(e.contains("Invalid boolean value"));
				TestResult::passed()
			}
		}
	}

	QuickCheck::new()
		.tests(1000) // Run 1000 random tests
		.quickcheck(property as fn(String) -> TestResult);
}

/// Test: parse_list never panics on random input
///
/// Why: Validates that parse_list handles any input gracefully,
/// including empty strings, special characters, and malformed delimiters.
#[rstest]
#[test]
fn test_fuzz_parse_list() {
	fn property(input: String) -> TestResult {
		// parse_list should never panic
		let result = std::panic::catch_unwind(|| parse_list(&input));

		if result.is_err() {
			return TestResult::failed();
		}

		// Result should always be Vec<String>
		let list = result.unwrap();

		// Verify result is valid Vec
		assert!(list.len() <= input.len() + 1); // Sanity check: can't have more items than characters

		TestResult::passed()
	}

	QuickCheck::new()
		.tests(1000)
		.quickcheck(property as fn(String) -> TestResult);
}

/// Test: parse_dict never panics on random input
///
/// Why: Validates that parse_dict handles any input gracefully,
/// including malformed key=value pairs and special characters.
#[rstest]
#[test]
fn test_fuzz_parse_dict() {
	fn property(input: String) -> TestResult {
		// parse_dict should never panic
		let result = std::panic::catch_unwind(|| parse_dict(&input));

		if result.is_err() {
			return TestResult::failed();
		}

		// Result should always be HashMap
		let dict = result.unwrap();

		// Verify result is valid HashMap
		for (key, value) in &dict {
			assert!(!key.is_empty() || !value.is_empty()); // At least one should have content
		}

		TestResult::passed()
	}

	QuickCheck::new()
		.tests(1000)
		.quickcheck(property as fn(String) -> TestResult);
}

/// Test: parse_database_url never panics on random input
///
/// Why: Validates that parse_database_url handles any input gracefully,
/// including invalid URLs, unsupported schemes, and malformed syntax.
#[rstest]
#[test]
fn test_fuzz_parse_database_url() {
	fn property(input: String) -> TestResult {
		// parse_database_url should never panic
		let result = std::panic::catch_unwind(|| parse_database_url(&input));

		if result.is_err() {
			return TestResult::failed();
		}

		// Result should always be Ok(DatabaseUrl) or Err(String)
		let parse_result = result.unwrap();
		match parse_result {
			Ok(db_url) => {
				// Valid database URL
				assert!(!db_url.engine.is_empty());
				TestResult::passed()
			}
			Err(e) => {
				// Invalid URL should return error, not panic
				assert!(
					e.contains("Invalid URL")
						|| e.contains("Unsupported")
						|| e.contains("required")
						|| e.contains("format")
				);
				TestResult::passed()
			}
		}
	}

	QuickCheck::new()
		.tests(1000)
		.quickcheck(property as fn(String) -> TestResult);
}

/// Test: parse_bool with edge cases
///
/// Why: Validates specific edge cases that might trigger panics:
/// empty strings, whitespace, null bytes, extremely long strings.
#[quickcheck]
fn test_parse_bool_edge_cases(input: String) -> TestResult {
	// Test with various edge case transformations
	let test_cases = vec![
		input.clone(),
		format!("   {}   ", input),      // Leading/trailing whitespace
		input.repeat(100),               // Very long string
		format!("{}\0{}", input, input), // Null bytes
		input.to_uppercase(),            // Case variations
		input.to_lowercase(),
	];

	for test_input in test_cases {
		let result = std::panic::catch_unwind(|| parse_bool(&test_input));
		if result.is_err() {
			return TestResult::failed();
		}
	}

	TestResult::passed()
}

/// Test: parse_list with special characters
///
/// Why: Validates that parse_list handles Unicode, emojis, control characters.
#[rstest]
#[case("")]
#[case(",,,")]
#[case("a,b,c")]
#[case("emoji😀,test🎉")]
#[case("unicode測試,テスト,тест")]
#[case("newline\n,tab\t,carriage\r")]
fn test_parse_list_special_characters(#[case] input: &str) {
	let result = std::panic::catch_unwind(|| parse_list(input));
	assert!(
		result.is_ok(),
		"parse_list should not panic on input: {:?}",
		input
	);
}

/// Test: parse_list with very long items
///
/// Why: Validates that parse_list handles extremely long input without panic.
#[rstest]
#[test]
fn test_parse_list_very_long_item() {
	let long_item = "a".repeat(10000);
	let result = std::panic::catch_unwind(|| parse_list(&long_item));
	assert!(
		result.is_ok(),
		"parse_list should not panic on very long input"
	);
}

/// Test: parse_dict with malformed input
///
/// Why: Validates that parse_dict handles missing delimiters,
/// multiple equals signs, and empty keys/values.
#[rstest]
#[case("")]
#[case("no_equals")]
#[case("=only_value")]
#[case("only_key=")]
#[case("key1=value1=extra")]
#[case("key1=value1,key2=value2,key3")]
#[case("===")]
#[case(",,,")]
fn test_parse_dict_malformed_input(#[case] input: &str) {
	let result = std::panic::catch_unwind(|| parse_dict(input));
	assert!(
		result.is_ok(),
		"parse_dict should not panic on input: {:?}",
		input
	);
}

/// Test: parse_database_url with invalid schemes
///
/// Why: Validates that parse_database_url rejects unknown database
/// engines gracefully without panicking.
#[rstest]
#[case("")]
#[case("not_a_url")]
#[case("unknown://host/db")]
#[case("http://example.com")]
#[case("ftp://server/file")]
#[case("postgresql://")]
#[case("mysql://host/")]
#[case("sqlite:")]
#[case("://no_scheme")]
fn test_parse_database_url_invalid_schemes(#[case] input: &str) {
	let result = std::panic::catch_unwind(|| parse_database_url(input));
	assert!(
		result.is_ok(),
		"parse_database_url should not panic on input: {:?}",
		input
	);
}

/// Test: parse_bool with quickcheck-generated strings
///
/// Why: Uses quickcheck's Arbitrary trait to generate diverse random strings.
#[quickcheck]
fn quickcheck_parse_bool_never_panics(input: String) -> bool {
	std::panic::catch_unwind(|| parse_bool(&input)).is_ok()
}

/// Test: parse_list with quickcheck-generated strings
///
/// Why: Uses quickcheck's Arbitrary trait to generate diverse random strings.
#[quickcheck]
fn quickcheck_parse_list_never_panics(input: String) -> bool {
	std::panic::catch_unwind(|| parse_list(&input)).is_ok()
}

/// Test: parse_dict with quickcheck-generated strings
///
/// Why: Uses quickcheck's Arbitrary trait to generate diverse random strings.
#[quickcheck]
fn quickcheck_parse_dict_never_panics(input: String) -> bool {
	std::panic::catch_unwind(|| parse_dict(&input)).is_ok()
}

/// Test: parse_database_url with quickcheck-generated strings
///
/// Why: Uses quickcheck's Arbitrary trait to generate diverse random strings.
#[quickcheck]
fn quickcheck_parse_database_url_never_panics(input: String) -> bool {
	std::panic::catch_unwind(|| parse_database_url(&input)).is_ok()
}

/// Test: Concurrent fuzzing to detect race conditions
///
/// Why: Validates that parsers are safe to use concurrently from multiple threads.
#[rstest]
#[test]
fn test_concurrent_fuzzing() {
	use std::sync::Arc;
	use std::thread;

	let test_inputs = Arc::new(vec![
		"true".to_string(),
		"false".to_string(),
		"postgresql://localhost/db".to_string(),
		"key1=value1,key2=value2".to_string(),
		"a,b,c,d,e".to_string(),
		"invalid_data_###".to_string(),
	]);

	let mut handles = vec![];

	for _ in 0..10 {
		let inputs = test_inputs.clone();
		let handle = thread::spawn(move || {
			for input in inputs.iter() {
				let _ = parse_bool(input);
				let _ = parse_list(input);
				let _ = parse_dict(input);
				let _ = parse_database_url(input);
			}
		});
		handles.push(handle);
	}

	for handle in handles {
		handle.join().expect("Thread should not panic");
	}
}

/// Custom generator for database URL-like strings
///
/// Why: Generates strings that look like database URLs but may be malformed,
/// testing parser's robustness against almost-valid input.
#[derive(Clone, Debug)]
struct DatabaseUrlLike(String);

impl Arbitrary for DatabaseUrlLike {
	fn arbitrary(g: &mut Gen) -> Self {
		let schemes = vec!["postgresql", "mysql", "sqlite", "http", "ftp", "invalid"];
		let hosts = vec!["localhost", "127.0.0.1", "example.com", ""];
		let paths = vec!["db", "mydb", "/path/to/db", ":memory:", ""];

		let scheme = g.choose(&schemes).unwrap();
		let host = g.choose(&hosts).unwrap();
		let path = g.choose(&paths).unwrap();

		let url = format!("{}://{}/{}", scheme, host, path);
		DatabaseUrlLike(url)
	}
}

/// Test: parse_database_url with URL-like strings
///
/// Why: Tests parser with strings that look like URLs but may be malformed.
#[quickcheck]
fn quickcheck_parse_database_url_like(input: DatabaseUrlLike) -> bool {
	std::panic::catch_unwind(|| parse_database_url(&input.0)).is_ok()
}
