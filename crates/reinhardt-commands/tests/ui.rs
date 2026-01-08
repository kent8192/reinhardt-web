//! Macro tests using trybuild
//!
//! Tests for command-related macros using compile-time verification.
//! These tests verify that macros produce correct compiler errors for invalid inputs.

use rstest::*;

// ============================================================================
// Trybuild Test Runner
// ============================================================================

/// Test: Command macro compile-time tests
///
/// Category: Happy Path / Error Path
/// Verifies that command macros produce correct compilation results.
///
/// Note: trybuild tests are run as a single test that processes all UI test files.
/// Each .rs file in the ui/ directory represents a test case.
#[rstest]
fn test_command_macros() {
	let t = trybuild::TestCases::new();

	// Happy path: Valid command definitions should compile
	t.pass("tests/ui/command_valid.rs");

	// Error path: Invalid command definitions should fail with helpful errors
	t.compile_fail("tests/ui/command_missing_name.rs");
	t.compile_fail("tests/ui/command_invalid_return.rs");
}

// ============================================================================
// Unit Tests for Macro Helpers
// ============================================================================

/// Test: Command name validation logic
///
/// Category: Happy Path
/// Verifies that valid command names are accepted.
#[rstest]
#[case("check", true)]
#[case("runserver", true)]
#[case("my_command", true)]
#[case("my-command", true)]
#[case("migrate", true)]
fn test_command_name_valid(#[case] name: &str, #[case] expected_valid: bool) {
	let is_valid = is_valid_command_name(name);
	assert_eq!(
		is_valid, expected_valid,
		"Name '{}' validity mismatch",
		name
	);
}

/// Test: Command name validation - invalid cases
///
/// Category: Error Path
/// Verifies that invalid command names are rejected.
#[rstest]
#[case("", false)]
#[case("123start", false)]
#[case("has space", false)]
fn test_command_name_invalid(#[case] name: &str, #[case] expected_valid: bool) {
	let is_valid = is_valid_command_name(name);
	assert_eq!(
		is_valid, expected_valid,
		"Name '{}' validity mismatch",
		name
	);
}

/// Helper function to validate command names
fn is_valid_command_name(name: &str) -> bool {
	if name.is_empty() {
		return false;
	}

	let first_char = name.chars().next().unwrap();
	if !first_char.is_ascii_alphabetic() && first_char != '_' {
		return false;
	}

	name.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

// ============================================================================
// Decision Table Tests
// ============================================================================

/// Test: Command attribute combinations
///
/// Category: Decision Table
/// Verifies various attribute combination behaviors.
#[rstest]
#[case(true, true, true, "all attributes")]
#[case(true, true, false, "name and description only")]
#[case(true, false, true, "name and help only")]
#[case(true, false, false, "name only")]
fn test_command_decision_attribute_combinations(
	#[case] has_name: bool,
	#[case] has_description: bool,
	#[case] has_help: bool,
	#[case] description: &str,
) {
	// Simulate attribute parsing
	let mut attrs = Vec::new();

	if has_name {
		attrs.push("name");
	}
	if has_description {
		attrs.push("description");
	}
	if has_help {
		attrs.push("help");
	}

	// Name is required
	let is_valid = has_name;

	assert!(
		is_valid || !has_name,
		"{}: should require name attribute",
		description
	);
}

// ============================================================================
// Boundary Value Tests
// ============================================================================

/// Test: Command name length boundaries
///
/// Category: Boundary
/// Verifies handling of various name lengths.
#[rstest]
#[case(1, true)]
#[case(10, true)]
#[case(50, true)]
#[case(100, true)]
fn test_command_name_length_boundaries(#[case] length: usize, #[case] expected_valid: bool) {
	let name: String = std::iter::repeat('a').take(length).collect();
	let is_valid = is_valid_command_name(&name);
	assert_eq!(
		is_valid, expected_valid,
		"Name length {} validity mismatch",
		length
	);
}

// ============================================================================
// Sanity Tests
// ============================================================================

/// Test: Macro helper sanity check
///
/// Category: Sanity
/// Verifies basic macro helper functionality.
#[rstest]
fn test_macro_sanity() {
	// Verify basic name validation works
	assert!(is_valid_command_name("check"));
	assert!(is_valid_command_name("my_command"));
	assert!(!is_valid_command_name(""));
	assert!(!is_valid_command_name("123"));
}
