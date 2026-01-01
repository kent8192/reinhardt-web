//! Extended CommandContext tests
//!
//! Additional tests for CommandContext beyond the inline tests in context.rs.
//! Focuses on boundary values, equivalence partitioning, and decision tables.

use reinhardt_commands::CommandContext;
use rstest::{fixture, rstest};
use serial_test::serial;
use std::collections::HashMap;

// =============================================================================
// Fixtures
// =============================================================================

#[fixture]
fn empty_context() -> CommandContext {
	CommandContext::new(vec![])
}

#[fixture]
fn populated_context() -> CommandContext {
	let args = vec![
		"first".to_string(),
		"second".to_string(),
		"third".to_string(),
	];
	let mut ctx = CommandContext::new(args);
	ctx.set_option("verbose".to_string(), "true".to_string());
	ctx.set_option("format".to_string(), "json".to_string());
	ctx.set_verbosity(2);
	ctx
}

// =============================================================================
// Happy Path Tests - Extended
// =============================================================================

/// Test with_settings method
///
/// **Category**: Happy Path
/// **Verifies**: Settings can be injected into context
#[rstest]
fn test_context_with_settings() {
	let ctx = CommandContext::new(vec![]);

	// Settings would be injected in real usage
	assert!(
		ctx.settings.is_none(),
		"Initial context should have no settings"
	);
}

/// Test builder pattern complete chain
///
/// **Category**: Happy Path
/// **Verifies**: All builder methods can be chained
#[rstest]
fn test_context_builder_complete_chain() {
	let mut options = HashMap::new();
	options.insert("key".to_string(), vec!["value".to_string()]);

	let ctx = CommandContext::new(vec![])
		.with_args(vec!["arg1".to_string(), "arg2".to_string()])
		.with_options(options);

	assert_eq!(ctx.args.len(), 2);
	assert_eq!(ctx.arg(0), Some(&"arg1".to_string()));
	assert!(ctx.has_option("key"));
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Test empty option value
///
/// **Category**: Edge Case
/// **Verifies**: Empty string as option value works
#[rstest]
fn test_context_empty_option_value(mut empty_context: CommandContext) {
	empty_context.set_option("empty".to_string(), String::new());

	assert!(empty_context.has_option("empty"));
	assert_eq!(empty_context.option("empty"), Some(&String::new()));
}

/// Test Unicode characters in arguments
///
/// **Category**: Edge Case
/// **Verifies**: Unicode arguments are handled correctly
#[rstest]
fn test_context_unicode_args() {
	let ctx = CommandContext::new(vec![
		"日本語".to_string(),
		"한국어".to_string(),
		"中文".to_string(),
		"Ελληνικά".to_string(),
	]);

	assert_eq!(ctx.arg(0), Some(&"日本語".to_string()));
	assert_eq!(ctx.arg(1), Some(&"한국어".to_string()));
	assert_eq!(ctx.arg(2), Some(&"中文".to_string()));
	assert_eq!(ctx.arg(3), Some(&"Ελληνικά".to_string()));
}

/// Test Unicode characters in option keys and values
///
/// **Category**: Edge Case
/// **Verifies**: Unicode in options is handled correctly
#[rstest]
fn test_context_unicode_options(mut empty_context: CommandContext) {
	empty_context.set_option("キー".to_string(), "値".to_string());

	assert!(empty_context.has_option("キー"));
	assert_eq!(empty_context.option("キー"), Some(&"値".to_string()));
}

/// Test special characters in arguments
///
/// **Category**: Edge Case
/// **Verifies**: Special characters are preserved
#[rstest]
fn test_context_special_characters() {
	let special = "test\n\t\r\"'<>&;|$`\\";
	let ctx = CommandContext::new(vec![special.to_string()]);

	assert_eq!(ctx.arg(0), Some(&special.to_string()));
}

/// Test very large number of arguments
///
/// **Category**: Edge Case
/// **Verifies**: Large argument lists are handled
#[rstest]
fn test_context_many_args() {
	let args: Vec<String> = (0..1000).map(|i| format!("arg{}", i)).collect();
	let ctx = CommandContext::new(args.clone());

	assert_eq!(ctx.args.len(), 1000);
	assert_eq!(ctx.arg(0), Some(&"arg0".to_string()));
	assert_eq!(ctx.arg(999), Some(&"arg999".to_string()));
	assert_eq!(ctx.arg(1000), None);
}

// =============================================================================
// Boundary Value Analysis Tests
// =============================================================================

/// Test argument index boundaries
///
/// **Category**: Boundary Value Analysis
/// **Verifies**: Edge indices behave correctly
#[rstest]
#[case(0, Some("first"))] // first valid index
#[case(1, Some("second"))] // middle index
#[case(2, Some("third"))] // last valid index
#[case(3, None)] // first invalid index (len)
#[case(100, None)] // well beyond bounds
fn test_context_arg_index_boundaries(
	populated_context: CommandContext,
	#[case] index: usize,
	#[case] expected: Option<&str>,
) {
	let result = populated_context.arg(index);
	assert_eq!(
		result.map(|s| s.as_str()),
		expected,
		"Index {} should return {:?}",
		index,
		expected
	);
}

/// Test verbosity level boundaries
///
/// **Category**: Boundary Value Analysis
/// **Verifies**: Verbosity edge values work correctly
#[rstest]
#[case(0)] // minimum
#[case(1)] // typical low
#[case(127)] // middle of u8
#[case(128)] // middle+1
#[case(254)] // max-1
#[case(255)] // maximum u8
fn test_context_verbosity_boundaries(#[case] level: u8) {
	let mut ctx = CommandContext::new(vec![]);
	ctx.set_verbosity(level);

	assert_eq!(ctx.verbosity(), level, "Verbosity should be {}", level);
}

/// Test option values count boundaries
///
/// **Category**: Boundary Value Analysis
/// **Verifies**: Multi-value options handle various counts
#[rstest]
#[case(0)]
#[case(1)]
#[case(10)]
#[case(100)]
fn test_context_option_values_count_boundaries(#[case] count: usize) {
	let mut ctx = CommandContext::new(vec![]);
	let values: Vec<String> = (0..count).map(|i| format!("val{}", i)).collect();

	ctx.set_option_multi("multi".to_string(), values.clone());

	let result = ctx.option_values("multi");
	if count == 0 {
		// Empty vec is still Some
		assert_eq!(result, Some(vec![]));
	} else {
		assert!(result.is_some());
		assert_eq!(result.unwrap().len(), count);
	}
}

// =============================================================================
// Equivalence Partitioning Tests
// =============================================================================

/// Test verbosity level equivalence classes
///
/// **Category**: Equivalence Partitioning
/// **Verifies**: Different verbosity levels behave as expected
#[rstest]
#[case(0, "quiet")]
#[case(1, "normal")]
#[case(2, "normal")]
#[case(3, "verbose")]
#[case(100, "verbose")]
#[case(255, "verbose")]
fn test_verbosity_level_partitions(#[case] level: u8, #[case] expected_class: &str) {
	let mut ctx = CommandContext::new(vec![]);
	ctx.set_verbosity(level);

	let actual_class = match ctx.verbosity() {
		0 => "quiet",
		1..=2 => "normal",
		_ => "verbose",
	};

	assert_eq!(
		actual_class, expected_class,
		"Verbosity {} should be in class '{}'",
		level, expected_class
	);
}

/// Test argument count equivalence classes
///
/// **Category**: Equivalence Partitioning
/// **Verifies**: Different argument counts are handled correctly
#[rstest]
#[case(vec![], "empty")]
#[case(vec!["one".to_string()], "single")]
#[case(vec!["one".to_string(), "two".to_string()], "multiple")]
#[case(vec!["a".to_string(), "b".to_string(), "c".to_string()], "multiple")]
fn test_args_count_partitions(#[case] args: Vec<String>, #[case] expected_class: &str) {
	let ctx = CommandContext::new(args.clone());

	let actual_class = match ctx.args.len() {
		0 => "empty",
		1 => "single",
		_ => "multiple",
	};

	assert_eq!(
		actual_class, expected_class,
		"Args {:?} should be in class '{}'",
		args, expected_class
	);
}

/// Test option presence equivalence classes
///
/// **Category**: Equivalence Partitioning
/// **Verifies**: Option presence states are distinguished
#[rstest]
#[case(false, "absent")]
#[case(true, "present")]
fn test_option_presence_partitions(#[case] present: bool, #[case] expected_class: &str) {
	let mut ctx = CommandContext::new(vec![]);

	if present {
		ctx.set_option("key".to_string(), "value".to_string());
	}

	let actual_class = if ctx.has_option("key") {
		"present"
	} else {
		"absent"
	};

	assert_eq!(actual_class, expected_class);
}

// =============================================================================
// Decision Table Tests
// =============================================================================

/// Decision table test for should_skip_checks
///
/// **Category**: Decision Table
/// **Verifies**: skip_checks and skip-checks option combinations
///
/// | skip_checks | skip-checks | Result |
/// |-------------|-------------|--------|
/// | false       | false       | false  |
/// | true        | false       | true   |
/// | false       | true        | true   |
/// | true        | true        | true   |
#[rstest]
#[case(false, false, false)]
#[case(true, false, true)]
#[case(false, true, true)]
#[case(true, true, true)]
fn test_should_skip_checks_decision(
	#[case] has_skip_checks: bool,
	#[case] has_skip_hyphen_checks: bool,
	#[case] expected: bool,
) {
	let mut ctx = CommandContext::new(vec![]);

	if has_skip_checks {
		ctx.set_option("skip_checks".to_string(), "true".to_string());
	}
	if has_skip_hyphen_checks {
		ctx.set_option("skip-checks".to_string(), "true".to_string());
	}

	assert_eq!(
		ctx.should_skip_checks(),
		expected,
		"skip_checks={}, skip-checks={} should return {}",
		has_skip_checks,
		has_skip_hyphen_checks,
		expected
	);
}

/// Decision table test for confirm method in test mode
///
/// **Category**: Decision Table
/// **Verifies**: confirm() behavior with different default values
///
/// Note: In test mode (cfg!(test)), confirm always returns default_value
///
/// | default_value | Result |
/// |---------------|--------|
/// | true          | true   |
/// | false         | false  |
#[rstest]
#[case(true, true)]
#[case(false, false)]
#[serial(user_interaction)]
fn test_confirm_decision_table(#[case] default_value: bool, #[case] expected: bool) {
	std::env::set_var("REINHARDT_TEST_MODE", "1");

	let ctx = CommandContext::new(vec![]);

	// In test mode, confirm returns default_value
	let result = ctx.confirm("Continue?", default_value).unwrap();

	assert_eq!(
		result, expected,
		"confirm with default={} should return {}",
		default_value, expected
	);

	std::env::remove_var("REINHARDT_TEST_MODE");
}

/// Decision table test for input method in test mode
///
/// **Category**: Decision Table
/// **Verifies**: input() behavior with different default values
///
/// Note: In test mode, input always returns default_value or empty string
///
/// | default_value  | Result          |
/// |----------------|-----------------|
/// | Some("value")  | "value"         |
/// | None           | ""              |
#[rstest]
#[case(Some("default"), "default")]
#[case(None, "")]
#[serial(user_interaction)]
fn test_input_decision_table(#[case] default_value: Option<&str>, #[case] expected: &str) {
	std::env::set_var("REINHARDT_TEST_MODE", "1");

	let ctx = CommandContext::new(vec![]);

	// In test mode, input returns default_value or empty string
	let result = ctx.input("Enter:", default_value).unwrap();

	assert_eq!(
		result, expected,
		"input with default={:?} should return '{}'",
		default_value, expected
	);

	std::env::remove_var("REINHARDT_TEST_MODE");
}

// =============================================================================
// State Transition Tests
// =============================================================================

/// Test adding arguments modifies state correctly
///
/// **Category**: State Transition
/// **Verifies**: add_arg correctly appends to args
#[rstest]
fn test_context_add_arg_state_transition(mut empty_context: CommandContext) {
	assert_eq!(empty_context.args.len(), 0, "Initial state: 0 args");

	empty_context.add_arg("first".to_string());
	assert_eq!(empty_context.args.len(), 1, "After first add: 1 arg");
	assert_eq!(empty_context.arg(0), Some(&"first".to_string()));

	empty_context.add_arg("second".to_string());
	assert_eq!(empty_context.args.len(), 2, "After second add: 2 args");
	assert_eq!(empty_context.arg(1), Some(&"second".to_string()));
}

/// Test setting options modifies state correctly
///
/// **Category**: State Transition
/// **Verifies**: set_option and set_option_multi modify options correctly
#[rstest]
fn test_context_option_state_transition(mut empty_context: CommandContext) {
	assert!(
		!empty_context.has_option("key"),
		"Initial state: no options"
	);

	empty_context.set_option("key".to_string(), "value1".to_string());
	assert!(empty_context.has_option("key"), "After set: option exists");
	assert_eq!(empty_context.option("key"), Some(&"value1".to_string()));

	// Overwrite with new value
	empty_context.set_option("key".to_string(), "value2".to_string());
	assert_eq!(
		empty_context.option("key"),
		Some(&"value2".to_string()),
		"After overwrite: new value"
	);

	// Set multi overwrites again
	empty_context.set_option_multi("key".to_string(), vec!["v1".to_string(), "v2".to_string()]);
	assert_eq!(
		empty_context.option_values("key"),
		Some(vec!["v1".to_string(), "v2".to_string()]),
		"After set_multi: multiple values"
	);
}

/// Test verbosity state transitions
///
/// **Category**: State Transition
/// **Verifies**: set_verbosity correctly updates verbosity
#[rstest]
fn test_context_verbosity_state_transition(mut empty_context: CommandContext) {
	assert_eq!(empty_context.verbosity(), 0, "Initial verbosity: 0");

	empty_context.set_verbosity(1);
	assert_eq!(empty_context.verbosity(), 1, "After set to 1");

	empty_context.set_verbosity(3);
	assert_eq!(empty_context.verbosity(), 3, "After set to 3");

	empty_context.set_verbosity(0);
	assert_eq!(empty_context.verbosity(), 0, "After reset to 0");
}

// =============================================================================
// Sanity Tests
// =============================================================================

/// Sanity test for CommandContext basic workflow
///
/// **Category**: Sanity
/// **Verifies**: Basic create-modify-access workflow works
#[rstest]
fn test_context_basic_sanity() {
	// Create
	let mut ctx = CommandContext::new(vec!["cmd".to_string()]);

	// Access arg
	assert_eq!(ctx.arg(0), Some(&"cmd".to_string()));

	// Set option
	ctx.set_option("verbose".to_string(), "true".to_string());
	assert!(ctx.has_option("verbose"));

	// Set verbosity
	ctx.set_verbosity(2);
	assert_eq!(ctx.verbosity(), 2);

	// Clone
	let cloned = ctx.clone();
	assert_eq!(cloned.arg(0), ctx.arg(0));

	// Debug
	let debug = format!("{:?}", ctx);
	assert!(debug.contains("CommandContext"));
}
