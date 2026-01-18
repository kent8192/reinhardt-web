//! Integration tests for EnvLoader Variable Interpolation Edge Cases.
//!
//! This test module validates that EnvLoader correctly handles complex variable
//! interpolation scenarios including simple variables, nested variables, circular
//! references, undefined variables, and escaped dollar signs.

use reinhardt_conf::settings::env_loader::EnvLoader;
use rstest::*;
use serial_test::serial;
use std::env;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

/// Test: Simple variable interpolation
///
/// Why: Validates that EnvLoader correctly expands simple variable references
/// in the format $VAR1 and ${VAR1}.
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_simple_variable() {
	// Setup: Create .env file with simple interpolation
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "VAR1=value").unwrap();
	writeln!(file, "VAR2=${{VAR1}}-suffix").unwrap();
	writeln!(file, "VAR3=${{VAR1}}_prefix").unwrap();

	// Load with interpolation enabled
	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with simple interpolation should succeed"
	);

	// VAR2 should expand $VAR1 to empty string (VAR1 not in environment before load)
	// Actually, since .env sets VAR1=value first, then VAR2 should use it
	// Wait, let me check: does EnvLoader set vars immediately or after all parsing?
	// Looking at code: it sets immediately (line 229: env::set_var)
	// So VAR1 is set before VAR2 is parsed
	assert_eq!(
		env::var("VAR1").unwrap(),
		"value",
		"VAR1 should be set to 'value'"
	);
	assert_eq!(
		env::var("VAR2").unwrap(),
		"value-suffix",
		"VAR2 should expand ${{VAR1}} to 'value-suffix'"
	);
	assert_eq!(
		env::var("VAR3").unwrap(),
		"value_prefix",
		"VAR3 should expand ${{VAR1}} to 'value_prefix'"
	);

	// Cleanup
	unsafe {
		env::remove_var("VAR1");
		env::remove_var("VAR2");
		env::remove_var("VAR3");
	}
}

/// Test: Nested variable interpolation
///
/// Why: Validates that EnvLoader handles variables that reference other variables
/// (VAR1=a, VAR2=${VAR1}b, VAR3=${VAR2}c should result in VAR3=abc).
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_nested_variables() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "VAR1=a").unwrap();
	writeln!(file, "VAR2=${{VAR1}}b").unwrap();
	writeln!(file, "VAR3=${{VAR2}}c").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with nested interpolation should succeed"
	);

	assert_eq!(env::var("VAR1").unwrap(), "a", "VAR1 should be 'a'");
	assert_eq!(
		env::var("VAR2").unwrap(),
		"ab",
		"VAR2 should expand ${{VAR1}} to 'ab'"
	);
	assert_eq!(
		env::var("VAR3").unwrap(),
		"abc",
		"VAR3 should expand ${{VAR2}} to 'abc'"
	);

	// Cleanup
	unsafe {
		env::remove_var("VAR1");
		env::remove_var("VAR2");
		env::remove_var("VAR3");
	}
}

/// Test: Circular reference detection
///
/// Why: Validates that EnvLoader handles circular references gracefully.
/// NOTE: Implementation does NOT detect circular references - instead, undefined
/// variables expand to empty string, so VAR1=${VAR2} with VAR2 undefined becomes VAR1="".
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_circular_reference() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "VAR1=${{VAR2}}").unwrap();
	writeln!(file, "VAR2=${{VAR1}}").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	// Implementation does NOT detect circular references
	// Instead, VAR1 expands ${VAR2} where VAR2 is not yet defined → ""
	// Then VAR2 expands ${VAR1} where VAR1="" → ""
	assert!(
		result.is_ok(),
		"Implementation does not detect circular references"
	);

	// Both variables should be empty strings
	assert_eq!(
		env::var("VAR1").unwrap(),
		"",
		"VAR1 should be empty (VAR2 was undefined at parse time)"
	);
	assert_eq!(
		env::var("VAR2").unwrap(),
		"",
		"VAR2 should expand to VAR1's value (empty string)"
	);

	// Cleanup
	unsafe {
		env::remove_var("VAR1");
		env::remove_var("VAR2");
	}
}

/// Test: Undefined variable interpolation
///
/// Why: Validates that undefined variables expand to empty string.
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_undefined_variable() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "VAR1=${{UNDEFINED_VAR}}").unwrap();
	writeln!(file, "VAR2=prefix-${{UNDEFINED_VAR}}-suffix").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with undefined variable should succeed"
	);

	// Undefined variables expand to empty string
	assert_eq!(
		env::var("VAR1").unwrap(),
		"",
		"VAR1 should be empty (UNDEFINED_VAR is undefined)"
	);
	assert_eq!(
		env::var("VAR2").unwrap(),
		"prefix--suffix",
		"VAR2 should have empty string for undefined variable"
	);

	// Cleanup
	unsafe {
		env::remove_var("VAR1");
		env::remove_var("VAR2");
	}
}

/// Test: Escaped dollar sign
///
/// Why: Validates that \$ prevents variable expansion and results in literal $.
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_escaped_dollar() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "VAR1=\\$NOT_INTERPOLATED").unwrap();
	writeln!(file, "VAR2=\\${{ALSO_NOT_INTERPOLATED}}").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with escaped dollar should succeed"
	);

	// Escaped dollar should result in literal $
	assert_eq!(
		env::var("VAR1").unwrap(),
		"$NOT_INTERPOLATED",
		"VAR1 should have literal $ (escaped)"
	);
	assert_eq!(
		env::var("VAR2").unwrap(),
		"${ALSO_NOT_INTERPOLATED}",
		"VAR2 should have literal ${{}} (escaped)"
	);

	// Cleanup
	unsafe {
		env::remove_var("VAR1");
		env::remove_var("VAR2");
	}
}

/// Test: Interpolation with pre-existing environment variables
///
/// Why: Validates that interpolation can reference variables from the actual
/// environment (not just .env file).
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_with_env_vars() {
	// Setup: Set environment variable before loading .env
	unsafe {
		env::set_var("EXISTING_VAR", "existing_value");
	}

	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "NEW_VAR=${{EXISTING_VAR}}_extended").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with env var interpolation should succeed"
	);

	// NEW_VAR should expand EXISTING_VAR from environment
	assert_eq!(
		env::var("NEW_VAR").unwrap(),
		"existing_value_extended",
		"NEW_VAR should expand ${{EXISTING_VAR}} from environment"
	);

	// Cleanup
	unsafe {
		env::remove_var("EXISTING_VAR");
		env::remove_var("NEW_VAR");
	}
}

/// Test: Interpolation disabled (default behavior)
///
/// Why: Validates that when interpolation is disabled, variable references
/// remain as literal strings.
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_disabled() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "VAR1=value").unwrap();
	writeln!(file, "VAR2=$VAR1_suffix").unwrap();
	writeln!(file, "VAR3=${{VAR1}}_prefix").unwrap();

	// Load WITHOUT interpolation (default)
	let loader = EnvLoader::new().path(&env_path).interpolate(false);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env without interpolation should succeed"
	);

	// Variables should remain as literal strings
	assert_eq!(env::var("VAR1").unwrap(), "value");
	assert_eq!(
		env::var("VAR2").unwrap(),
		"$VAR1_suffix",
		"VAR2 should be literal (interpolation disabled)"
	);
	assert_eq!(
		env::var("VAR3").unwrap(),
		"${VAR1}_prefix",
		"VAR3 should be literal (interpolation disabled)"
	);

	// Cleanup
	unsafe {
		env::remove_var("VAR1");
		env::remove_var("VAR2");
		env::remove_var("VAR3");
	}
}

/// Test: Complex interpolation with multiple variables
///
/// Why: Validates that a single value can reference multiple variables.
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_multiple_variables() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "FIRST=first").unwrap();
	writeln!(file, "SECOND=second").unwrap();
	writeln!(file, "COMBINED=$FIRST-and-${{SECOND}}").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with multiple variable interpolation should succeed"
	);

	assert_eq!(
		env::var("COMBINED").unwrap(),
		"first-and-second",
		"COMBINED should expand both variables"
	);

	// Cleanup
	unsafe {
		env::remove_var("FIRST");
		env::remove_var("SECOND");
		env::remove_var("COMBINED");
	}
}

/// Test: Interpolation with special characters in values
///
/// Why: Validates that interpolated values containing special characters
/// (spaces, punctuation) are handled correctly.
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_special_characters() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "SPECIAL=\"hello world!@#&%\"").unwrap();
	writeln!(file, "EXPANDED=${{SPECIAL}}-extended").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with special characters should succeed"
	);

	assert_eq!(
		env::var("SPECIAL").unwrap(),
		"hello world!@#&%",
		"SPECIAL should have special characters (quotes removed)"
	);
	assert_eq!(
		env::var("EXPANDED").unwrap(),
		"hello world!@#&%-extended",
		"EXPANDED should preserve special characters from interpolation"
	);

	// Cleanup
	unsafe {
		env::remove_var("SPECIAL");
		env::remove_var("EXPANDED");
	}
}

/// Test: Empty variable interpolation
///
/// Why: Validates that empty variables are handled correctly in interpolation.
#[rstest]
#[serial(env_interpolation)]
#[test]
fn test_interpolation_empty_variable() {
	let temp_dir = TempDir::new().unwrap();
	let env_path = temp_dir.path().join(".env");
	let mut file = File::create(&env_path).unwrap();
	writeln!(file, "EMPTY=").unwrap();
	writeln!(file, "USES_EMPTY=prefix-${{EMPTY}}-suffix").unwrap();

	let loader = EnvLoader::new().path(&env_path).interpolate(true);
	let result = loader.load();

	assert!(
		result.is_ok(),
		"Loading .env with empty variable should succeed"
	);

	assert_eq!(
		env::var("EMPTY").unwrap(),
		"",
		"EMPTY should be empty string"
	);
	assert_eq!(
		env::var("USES_EMPTY").unwrap(),
		"prefix--suffix",
		"USES_EMPTY should have empty string for EMPTY variable"
	);

	// Cleanup
	unsafe {
		env::remove_var("EMPTY");
		env::remove_var("USES_EMPTY");
	}
}
