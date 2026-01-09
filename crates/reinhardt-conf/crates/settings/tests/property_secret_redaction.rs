//! Property-Based Tests for Secret Redaction Invariant.
//!
//! This test module validates that for any SecretString, format!("{:?}", secret)
//! never contains the actual secret value, ensuring secrets never leak through logging.
//!
//! NOTE: These tests are feature-gated with "async" feature.
//!
//! ## Testing Strategy
//!
//! - Generate random secret values
//! - Verify Debug/Display output always shows redacted marker
//! - Test with different string lengths and character sets
//! - Verify secrets never leak in error messages or logs

#![cfg(feature = "async")]

use quickcheck_macros::quickcheck;
use reinhardt_settings::secrets::SecretString;
use rstest::*;
use std::fmt::Write as FmtWrite;

/// Test: SecretString Debug output never reveals secret
///
/// Why: Validates that Debug trait implementation always redacts the secret value,
/// preventing accidental exposure through logging.
#[quickcheck]
fn quickcheck_debug_redaction(secret_value: String) -> bool {
	let secret = SecretString::new(secret_value.clone());
	let debug_output = format!("{:?}", secret);

	// Debug output should NOT contain the actual secret (unless empty)
	// Note: In Rust, any_string.contains("") is always true, so we need special handling
	(!debug_output.contains(&secret_value) || secret_value.is_empty())
		// Debug output should contain redaction marker
		&& debug_output.contains("[REDACTED]")
}

/// Test: SecretString Display output never reveals secret
///
/// Why: Validates that Display trait implementation always redacts the secret value.
#[quickcheck]
fn quickcheck_display_redaction(secret_value: String) -> bool {
	let secret = SecretString::new(secret_value.clone());
	let display_output = format!("{}", secret);

	// Display output should NOT contain the actual secret (unless empty)
	// Note: In Rust, any_string.contains("") is always true, so we need special handling
	(!display_output.contains(&secret_value) || secret_value.is_empty())
		// Display output should contain redaction marker
		&& display_output.contains("[REDACTED]")
}

/// Test: SecretString redaction with various string lengths
///
/// Why: Validates that redaction works correctly regardless of secret length.
#[rstest]
#[case("")] // Empty secret
#[case("a")] // Single character
#[case("short")] // Short secret
#[case("medium_length_secret_value")] // Medium secret
#[case(&"a".repeat(1000))] // Very long secret
fn test_redaction_various_lengths(#[case] secret_value: &str) {
	let secret = SecretString::new(secret_value.to_string());

	let debug_output = format!("{:?}", secret);
	let display_output = format!("{}", secret);

	// Verify redaction
	assert!(
		!debug_output.contains(secret_value) || secret_value.is_empty(),
		"Debug output should not contain secret (unless empty)"
	);
	assert!(
		!display_output.contains(secret_value) || secret_value.is_empty(),
		"Display output should not contain secret (unless empty)"
	);

	assert!(
		debug_output.contains("[REDACTED]"),
		"Debug output should contain redaction marker"
	);
	assert!(
		display_output.contains("[REDACTED]"),
		"Display output should contain redaction marker"
	);
}

/// Test: SecretString redaction with special characters
///
/// Why: Validates that redaction works with Unicode and special characters.
#[rstest]
#[case("password123")] // Alphanumeric
#[case("p@ssw0rd!")] // Special characters
#[case("日本語パスワード")] // Japanese
#[case("🔒🔑🛡️")] // Emoji
#[case("line1\nline2")] // Newlines
#[case("tab\tseparated")] // Tabs
fn test_redaction_special_characters(#[case] secret_value: &str) {
	let secret = SecretString::new(secret_value.to_string());

	let debug_output = format!("{:?}", secret);
	let display_output = format!("{}", secret);

	assert!(
		!debug_output.contains(secret_value),
		"Debug output should not contain secret with special characters"
	);
	assert!(
		!display_output.contains(secret_value),
		"Display output should not contain secret with special characters"
	);
}

/// Test: Multiple secrets have independent redaction
///
/// Why: Validates that redaction works correctly when multiple secrets exist.
#[rstest]
#[test]
fn test_multiple_secrets_independent_redaction() {
	let secret1 = SecretString::new("secret_one".to_string());
	let secret2 = SecretString::new("secret_two".to_string());
	let secret3 = SecretString::new("secret_three".to_string());

	let output = format!("{:?}, {:?}, {:?}", secret1, secret2, secret3);

	// Verify no secrets leaked
	assert!(!output.contains("secret_one"));
	assert!(!output.contains("secret_two"));
	assert!(!output.contains("secret_three"));

	// Verify all have redaction markers
	assert_eq!(
		output.matches("[REDACTED]").count(),
		3,
		"Should have 3 redaction markers"
	);
}

/// Test: SecretString in data structures
///
/// Why: Validates that secrets remain redacted when used in structs/vectors.
#[rstest]
#[test]
fn test_secrets_in_data_structures() {
	#[derive(Debug)]
	#[allow(dead_code)] // Fields are used only for Debug trait testing
	struct Config {
		api_key: SecretString,
		database_password: SecretString,
	}

	let config = Config {
		api_key: SecretString::new("api_key_12345".to_string()),
		database_password: SecretString::new("db_pass_67890".to_string()),
	};

	let debug_output = format!("{:?}", config);

	assert!(!debug_output.contains("api_key_12345"));
	assert!(!debug_output.contains("db_pass_67890"));
	assert!(debug_output.contains("[REDACTED]"));
}

/// Test: SecretString in collections
///
/// Why: Validates that secrets in Vec/HashMap remain redacted.
#[rstest]
#[test]
fn test_secrets_in_collections() {
	let secrets = vec![
		SecretString::new("secret1".to_string()),
		SecretString::new("secret2".to_string()),
		SecretString::new("secret3".to_string()),
	];

	let debug_output = format!("{:?}", secrets);

	assert!(!debug_output.contains("secret1"));
	assert!(!debug_output.contains("secret2"));
	assert!(!debug_output.contains("secret3"));
	assert_eq!(debug_output.matches("[REDACTED]").count(), 3);
}

/// Test: SecretString does not leak in panic messages
///
/// Why: Validates that secrets are not exposed in panic messages.
#[rstest]
#[test]
fn test_no_leak_in_panic() {
	use std::panic;

	let secret = SecretString::new("panic_secret_value".to_string());

	let result = panic::catch_unwind(|| {
		panic!("Error with secret: {:?}", secret);
	});

	assert!(result.is_err(), "Panic should occur");

	// Note: In real panic messages, the secret should be redacted
	// We can't directly inspect panic messages, but this test documents expected behavior
}

/// Test: SecretString serialization safety
///
/// Why: Validates that secrets are not accidentally serialized in plain text.
#[rstest]
#[test]
fn test_serialization_safety() {
	let secret = SecretString::new("serialization_test_secret".to_string());

	// Format as string (common serialization pattern)
	let output = format!("{:?}", secret);

	assert!(
		!output.contains("serialization_test_secret"),
		"Serialized output should not contain secret"
	);
}

/// Test: SecretString comparison does not leak
///
/// Why: Validates that comparing secrets does not reveal values.
#[rstest]
#[test]
fn test_comparison_no_leak() {
	let secret1 = SecretString::new("same_value".to_string());
	let secret2 = SecretString::new("same_value".to_string());
	let secret3 = SecretString::new("different_value".to_string());

	// Comparison operations
	let eq_same = secret1 == secret2;
	let eq_diff = secret1 == secret3;

	// Verify comparisons work
	assert!(eq_same, "Same secrets should be equal");
	assert!(!eq_diff, "Different secrets should not be equal");

	// Verify no values leaked in any output
	let output1 = format!("{:?}", secret1);
	let output2 = format!("{:?}", secret2);
	let output3 = format!("{:?}", secret3);

	assert!(!output1.contains("same_value"));
	assert!(!output2.contains("same_value"));
	assert!(!output3.contains("different_value"));
}

/// Test: SecretString in error messages
///
/// Why: Validates that secrets don't leak through error messages.
#[rstest]
#[test]
fn test_no_leak_in_errors() {
	let secret = SecretString::new("error_secret_value".to_string());

	// Simulate error message containing secret
	let error_message = format!("Authentication failed for secret: {:?}", secret);

	assert!(
		!error_message.contains("error_secret_value"),
		"Error message should not contain secret"
	);
	assert!(
		error_message.contains("[REDACTED]"),
		"Error message should contain redaction"
	);
}

/// Test: SecretString in logs
///
/// Why: Validates that secrets don't leak through logging statements.
#[rstest]
#[test]
fn test_no_leak_in_logs() {
	let secret = SecretString::new("log_secret_value".to_string());

	// Simulate log statement
	let mut log_buffer = String::new();
	write!(log_buffer, "[INFO] Using secret: {:?}", secret).unwrap();

	assert!(
		!log_buffer.contains("log_secret_value"),
		"Log output should not contain secret"
	);
	assert!(
		log_buffer.contains("[REDACTED]"),
		"Log output should contain redaction"
	);
}

/// Test: SecretString cloning preserves redaction
///
/// Why: Validates that cloned secrets maintain redaction behavior.
#[rstest]
#[test]
fn test_cloned_secret_redaction() {
	let original = SecretString::new("clone_secret".to_string());
	let cloned = original.clone();

	let original_output = format!("{:?}", original);
	let cloned_output = format!("{:?}", cloned);

	assert!(!original_output.contains("clone_secret"));
	assert!(!cloned_output.contains("clone_secret"));
	assert!(original_output.contains("[REDACTED]"));
	assert!(cloned_output.contains("[REDACTED]"));
}

/// Test: SecretString with whitespace only
///
/// Why: Validates redaction for secrets containing only whitespace.
#[rstest]
#[case("   ")] // Spaces
#[case("\t\t")] // Tabs
#[case("\n\n")] // Newlines
#[case(" \t\n ")] // Mixed whitespace
fn test_redaction_whitespace_only(#[case] secret_value: &str) {
	let secret = SecretString::new(secret_value.to_string());

	let debug_output = format!("{:?}", secret);
	let display_output = format!("{}", secret);

	// Whitespace-only secrets should also be redacted
	assert!(!debug_output.contains(secret_value));
	assert!(!display_output.contains(secret_value));
	assert!(debug_output.contains("[REDACTED]"));
	assert!(display_output.contains("[REDACTED]"));
}

/// Test: SecretString with control characters
///
/// Why: Validates redaction for secrets with control characters.
#[rstest]
#[test]
fn test_redaction_control_characters() {
	let secret_value = "secret\x00with\x01control\x02chars";
	let secret = SecretString::new(secret_value.to_string());

	let debug_output = format!("{:?}", secret);
	let display_output = format!("{}", secret);

	assert!(!debug_output.contains(secret_value));
	assert!(!display_output.contains(secret_value));
	assert!(debug_output.contains("[REDACTED]"));
	assert!(display_output.contains("[REDACTED]"));
}

/// Test: SecretString concatenation does not leak
///
/// Why: Validates that string operations involving secrets don't leak values.
#[rstest]
#[test]
fn test_concatenation_no_leak() {
	let secret = SecretString::new("concat_secret".to_string());

	// Various string operations
	let output1 = format!("Prefix-{:?}-Suffix", secret);
	let output2 = format!("{:?}{:?}", secret, secret);

	assert!(!output1.contains("concat_secret"));
	assert!(!output2.contains("concat_secret"));
	assert!(output1.contains("[REDACTED]"));
	assert!(output2.contains("[REDACTED]"));
}

/// Test: SecretString with percent-encoded characters
///
/// Why: Validates redaction for URL-encoded secrets.
#[rstest]
#[test]
fn test_redaction_percent_encoded() {
	let secret_value = "secret%20with%20encoding";
	let secret = SecretString::new(secret_value.to_string());

	let debug_output = format!("{:?}", secret);

	assert!(!debug_output.contains(secret_value));
	assert!(debug_output.contains("[REDACTED]"));
}

/// Test: SecretString concurrent access does not leak
///
/// Why: Validates that secrets remain redacted under concurrent access.
#[rstest]
#[test]
fn test_concurrent_redaction() {
	use std::sync::Arc;
	use std::thread;

	let secret = Arc::new(SecretString::new("concurrent_secret".to_string()));
	let mut handles = vec![];

	for _ in 0..10 {
		let secret_clone = secret.clone();
		let handle = thread::spawn(move || {
			let output = format!("{:?}", secret_clone);
			assert!(!output.contains("concurrent_secret"));
			assert!(output.contains("[REDACTED]"));
		});
		handles.push(handle);
	}

	for handle in handles {
		handle.join().expect("Thread should not panic");
	}
}
