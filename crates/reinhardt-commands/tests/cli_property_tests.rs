//! CLI property-based tests
//!
//! Property-based and fuzz tests for CLI parsing and command context.

use proptest::prelude::*;
use reinhardt_commands::CommandContext;
use rstest::*;

// ============================================================================
// Property-Based Tests: CommandContext
// ============================================================================

proptest! {
	/// Test: CommandContext args roundtrip
	///
	/// Category: Property
	/// Verifies that args can be added and retrieved correctly.
	#[rstest]
	fn prop_context_args_roundtrip(args in prop::collection::vec("[a-z]{1,20}", 0..10)) {
		let mut ctx = CommandContext::default();

		for arg in &args {
			ctx.add_arg(arg.clone());
		}

		for (i, arg) in args.iter().enumerate() {
			let retrieved = ctx.arg(i).map(String::as_str);
			prop_assert_eq!(retrieved, Some(arg.as_str()));
		}
	}

	/// Test: CommandContext options roundtrip
	///
	/// Category: Property
	/// Verifies that options can be set and retrieved correctly.
	#[rstest]
	fn prop_context_options_roundtrip(
		options in prop::collection::hash_map("[a-z]{1,10}", "[a-z0-9]{1,20}", 0..10)
	) {
		let mut ctx = CommandContext::default();

		for (key, value) in &options {
			ctx.set_option(key.clone(), value.clone());
		}

		for (key, value) in &options {
			let retrieved = ctx.option(key).map(String::as_str);
			prop_assert_eq!(retrieved, Some(value.as_str()));
		}
	}

	/// Test: CommandContext verbosity bounds
	///
	/// Category: Property
	/// Verifies that verbosity is correctly bounded.
	#[rstest]
	fn prop_context_verbosity_bounds(verbosity in 0u8..=255u8) {
		let mut ctx = CommandContext::default();
		ctx.set_verbosity(verbosity);

		prop_assert_eq!(ctx.verbosity, verbosity);
	}

	/// Test: CommandContext option presence
	///
	/// Category: Property
	/// Verifies has_option consistency with option().
	#[rstest]
	fn prop_context_option_presence(key in "[a-z]{1,10}", value in "[a-z0-9]{1,20}") {
		let mut ctx = CommandContext::default();
		ctx.set_option(key.clone(), value);

		prop_assert!(ctx.has_option(&key));
		prop_assert!(ctx.option(&key).is_some());
	}

	/// Test: CommandContext non-existent option
	///
	/// Category: Property
	/// Verifies that non-existent options return None.
	#[rstest]
	fn prop_context_nonexistent_option(key in "[a-z]{1,10}") {
		let ctx = CommandContext::default();

		prop_assert!(!ctx.has_option(&key));
		prop_assert!(ctx.option(&key).is_none());
	}
}

// ============================================================================
// Property-Based Tests: Argument Parsing
// ============================================================================

proptest! {
	/// Test: Argument index out of bounds
	///
	/// Category: Property
	/// Verifies that out-of-bounds arg access returns None.
	#[rstest]
	fn prop_arg_out_of_bounds(
		args in prop::collection::vec("[a-z]{1,10}", 0..5),
		index in 5usize..100
	) {
		let mut ctx = CommandContext::default();

		for arg in &args {
			ctx.add_arg(arg.clone());
		}

		prop_assert!(ctx.arg(index).is_none());
	}

	/// Test: Unicode in arguments preserved
	///
	/// Category: Property
	/// Verifies that Unicode characters are preserved in arguments.
	#[rstest]
	fn prop_unicode_args_preserved(arg in "\\PC{1,50}") {
		let mut ctx = CommandContext::default();
		ctx.add_arg(arg.clone());

		let retrieved = ctx.arg(0).map(String::as_str);
		prop_assert_eq!(retrieved, Some(arg.as_str()));
	}

	/// Test: Unicode in options preserved
	///
	/// Category: Property
	/// Verifies that Unicode characters are preserved in options.
	#[rstest]
	fn prop_unicode_options_preserved(
		key in "[a-z]{1,10}",
		value in "\\PC{1,50}"
	) {
		let mut ctx = CommandContext::default();
		ctx.set_option(key.clone(), value.clone());

		let retrieved = ctx.option(&key).map(String::as_str);
		prop_assert_eq!(retrieved, Some(value.as_str()));
	}
}

// ============================================================================
// Property-Based Tests: Clone and Equality
// ============================================================================

proptest! {
	/// Test: CommandContext clone invariant
	///
	/// Category: Property
	/// Verifies that cloned contexts have identical content.
	#[rstest]
	fn prop_context_clone_invariant(
		args in prop::collection::vec("[a-z]{1,10}", 0..5),
		verbosity in 0u8..10u8
	) {
		let mut ctx = CommandContext::default();
		for arg in &args {
			ctx.add_arg(arg.clone());
		}
		ctx.set_verbosity(verbosity);

		let cloned = ctx.clone();

		prop_assert_eq!(cloned.verbosity, ctx.verbosity);
		for (i, arg) in args.iter().enumerate() {
			let retrieved = cloned.arg(i).map(String::as_str);
			prop_assert_eq!(retrieved, Some(arg.as_str()));
		}
	}
}

// ============================================================================
// Fuzz Tests: Extreme Values
// ============================================================================

proptest! {
	/// Test: Many args handling
	///
	/// Category: Fuzz
	/// Verifies handling of many arguments.
	#[rstest]
	fn fuzz_many_args(args in prop::collection::vec("[a-z]{1,5}", 50..100)) {
		let mut ctx = CommandContext::default();

		for arg in &args {
			ctx.add_arg(arg.clone());
		}

		// Verify all args are accessible
		for (i, arg) in args.iter().enumerate() {
			let retrieved = ctx.arg(i).map(String::as_str);
			prop_assert_eq!(retrieved, Some(arg.as_str()));
		}
	}

	/// Test: Many options handling
	///
	/// Category: Fuzz
	/// Verifies handling of many options.
	#[rstest]
	fn fuzz_many_options(
		options in prop::collection::vec(("[a-z]{1,5}", "[a-z]{1,5}"), 50..100)
	) {
		let mut ctx = CommandContext::default();

		for (key, value) in &options {
			ctx.set_option(key.clone(), value.clone());
		}

		// Verify last value for each key is accessible (overwrites)
		// Note: HashMap behavior means later values overwrite earlier ones
		for (key, _) in &options {
			prop_assert!(ctx.has_option(key));
		}
	}

	/// Test: Long strings handling
	///
	/// Category: Fuzz
	/// Verifies handling of long string values.
	#[rstest]
	fn fuzz_long_strings(
		long_arg in "[a-z]{100,500}",
		long_option_value in "[a-z]{100,500}"
	) {
		let mut ctx = CommandContext::default();
		ctx.add_arg(long_arg.clone());
		ctx.set_option("long".to_string(), long_option_value.clone());

		let retrieved_arg = ctx.arg(0).map(String::as_str);
		let retrieved_opt = ctx.option("long").map(String::as_str);
		prop_assert_eq!(retrieved_arg, Some(long_arg.as_str()));
		prop_assert_eq!(retrieved_opt, Some(long_option_value.as_str()));
	}

	/// Test: Empty strings handling
	///
	/// Category: Fuzz
	/// Verifies handling of empty string values.
	#[rstest]
	fn fuzz_empty_strings(_dummy in Just(())) {
		let mut ctx = CommandContext::default();
		ctx.add_arg(String::new());
		ctx.set_option("empty".to_string(), String::new());

		let retrieved_arg = ctx.arg(0).map(String::as_str);
		let retrieved_opt = ctx.option("empty").map(String::as_str);
		prop_assert_eq!(retrieved_arg, Some(""));
		prop_assert_eq!(retrieved_opt, Some(""));
	}
}

// ============================================================================
// Deterministic Tests for Edge Cases
// ============================================================================

/// Test: Context with special characters
///
/// Category: Edge Case
/// Verifies handling of special characters.
#[rstest]
#[case("path/to/file")]
#[case("--option-like")]
#[case("=value=")]
#[case("'quoted'")]
#[case("\"double-quoted\"")]
fn test_special_chars_in_args(#[case] arg: &str) {
	let mut ctx = CommandContext::default();
	ctx.add_arg(arg.to_string());

	assert_eq!(ctx.arg(0).map(String::as_str), Some(arg));
}

/// Test: Context with whitespace
///
/// Category: Edge Case
/// Verifies handling of whitespace in values.
#[rstest]
#[case("  leading")]
#[case("trailing  ")]
#[case("  both  ")]
#[case("internal space")]
#[case("\ttab")]
#[case("\nnewline")]
fn test_whitespace_in_args(#[case] arg: &str) {
	let mut ctx = CommandContext::default();
	ctx.add_arg(arg.to_string());

	assert_eq!(ctx.arg(0).map(String::as_str), Some(arg));
}

/// Test: Option key formats
///
/// Category: Edge Case
/// Verifies handling of various option key formats.
#[rstest]
#[case("simple")]
#[case("with-hyphen")]
#[case("with_underscore")]
#[case("mixedCase")]
#[case("UPPERCASE")]
fn test_option_key_formats(#[case] key: &str) {
	let mut ctx = CommandContext::default();
	ctx.set_option(key.to_string(), "value".to_string());

	assert!(ctx.has_option(key));
	assert_eq!(ctx.option(key).map(String::as_str), Some("value"));
}

// ============================================================================
// Sanity Tests
// ============================================================================

/// Test: Property tests sanity check
///
/// Category: Sanity
/// Verifies basic property test infrastructure works.
#[rstest]
fn test_property_tests_sanity() {
	let mut ctx = CommandContext::default();

	// Basic operations
	ctx.add_arg("test".to_string());
	ctx.set_option("key".to_string(), "value".to_string());
	ctx.set_verbosity(1);

	// Verify
	assert_eq!(ctx.arg(0).map(String::as_str), Some("test"));
	assert_eq!(ctx.option("key").map(String::as_str), Some("value"));
	assert_eq!(ctx.verbosity, 1);
}
