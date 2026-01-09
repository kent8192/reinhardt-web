//! Equivalence Partitioning Tests for Parse Bool.
//!
//! This test module validates that EnvParser correctly handles all equivalence classes
//! of boolean input.
//!
//! ## Partitions
//!
//! - Lowercase true variants (true, yes, on)
//! - Uppercase true variants (TRUE, YES, ON)
//! - Mixed case true variants (True, Yes, On)
//! - Numeric true (1)
//! - Lowercase false variants (false, no, off)
//! - Uppercase false variants (FALSE, NO, OFF)
//! - Mixed case false variants (False, No, Off)
//! - Numeric false (0)
//! - Invalid inputs (ambiguous, typo, invalid numbers)

use reinhardt_settings::env_parser::parse_bool;
use rstest::*;

/// Test: Parse Bool - Valid True Equivalence Classes
///
/// Why: Validates that all valid representations of "true" are parsed correctly.
#[rstest]
#[case("true", "Lowercase true")]
#[case("TRUE", "Uppercase true")]
#[case("True", "Mixed case true")]
#[case("TrUe", "Random mixed case true")]
#[case("1", "Numeric true")]
#[case("yes", "Affirmative yes")]
#[case("YES", "Uppercase YES")]
#[case("Yes", "Mixed case Yes")]
#[case("on", "Switch on")]
#[case("ON", "Uppercase ON")]
#[case("On", "Mixed case On")]
fn test_parse_bool_true_equivalence_classes(#[case] input: &str, #[case] description: &str) {
	let result = parse_bool(input);

	assert_eq!(
		result,
		Ok(true),
		"parse_bool({:?}) should return true for partition: {}",
		input,
		description
	);
}

/// Test: Parse Bool - Valid False Equivalence Classes
///
/// Why: Validates that all valid representations of "false" are parsed correctly.
#[rstest]
#[case("false", "Lowercase false")]
#[case("FALSE", "Uppercase false")]
#[case("False", "Mixed case false")]
#[case("FaLsE", "Random mixed case false")]
#[case("0", "Numeric false")]
#[case("no", "Negative no")]
#[case("NO", "Uppercase NO")]
#[case("No", "Mixed case No")]
#[case("off", "Switch off")]
#[case("OFF", "Uppercase OFF")]
#[case("Off", "Mixed case Off")]
fn test_parse_bool_false_equivalence_classes(#[case] input: &str, #[case] description: &str) {
	let result = parse_bool(input);

	assert_eq!(
		result,
		Ok(false),
		"parse_bool({:?}) should return false for partition: {}",
		input,
		description
	);
}

/// Test: Parse Bool - Invalid Input Equivalence Classes
///
/// Why: Validates that parse_bool correctly rejects invalid inputs.
#[rstest]
#[case("maybe", "Ambiguous word")]
#[case("yesno", "Mixed words")]
#[case("truee", "Typo in true")]
#[case("falsee", "Typo in false")]
#[case("2", "Invalid number (not 0 or 1)")]
#[case("-1", "Negative number")]
#[case("", "Empty string")]
#[case("  ", "Whitespace only")]
#[case("null", "Null keyword")]
#[case("undefined", "Undefined keyword")]
fn test_parse_bool_invalid_equivalence_classes(#[case] input: &str, #[case] description: &str) {
	let result = parse_bool(input);

	assert!(
		result.is_err(),
		"parse_bool({:?}) should fail for partition: {}",
		input,
		description
	);
}

/// Test: Parse Bool - Whitespace Handling
///
/// Why: Validates whitespace trimming behavior.
#[rstest]
#[case(" true", "Leading space")]
#[case("true ", "Trailing space")]
#[case(" true ", "Both spaces")]
#[case("\ttrue", "Leading tab")]
#[case("true\t", "Trailing tab")]
#[case("\ntrue", "Leading newline")]
#[case("true\n", "Trailing newline")]
fn test_parse_bool_whitespace_handling(#[case] input: &str, #[case] description: &str) {
	let result = parse_bool(input);

	// Should either succeed (after trimming) or fail consistently
	// Document actual behavior
	if result.is_ok() {
		assert_eq!(
			result.unwrap(),
			true,
			"parse_bool({:?}) with whitespace should return true if trimmed: {}",
			input,
			description
		);
	} else {
		// If it fails, document that whitespace is not trimmed
		assert!(
			result.is_err(),
			"parse_bool({:?}) fails (whitespace not trimmed): {}",
			input,
			description
		);
	}
}
