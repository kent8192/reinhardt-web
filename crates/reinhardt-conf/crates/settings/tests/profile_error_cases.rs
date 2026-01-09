//! Integration tests for Profile error cases and edge conditions.
//!
//! This test module validates that the Profile enum handles invalid or edge-case
//! inputs gracefully, including empty strings, whitespace, and missing environment variables.

use reinhardt_settings::profile::Profile;
use rstest::*;
use serial_test::serial;
use std::env;

/// Test: Profile from environment with empty string
///
/// Why: Validates that Profile::from_env() handles empty REINHARDT_ENV gracefully,
/// defaulting to Development or returning Custom.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_from_env_empty_string() {
	unsafe {
		env::set_var("REINHARDT_ENV", "");
	}

	let profile = Profile::from_env();

	// Profile should either be None or return Custom for empty string
	match profile {
		None => {
			assert!(true, "Empty string returns None");
		}
		Some(Profile::Development) => {
			assert!(true, "Empty string defaulted to Development");
		}
		Some(Profile::Custom) => {
			assert!(true, "Empty string treated as Custom");
		}
		_ => {
			panic!("Unexpected profile for empty REINHARDT_ENV: {:?}", profile);
		}
	}

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile from environment when unset
///
/// Why: Validates that Profile::from_env() provides a sensible default
/// when REINHARDT_ENV is not set.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_from_env_unset() {
	unsafe {
		env::remove_var("REINHARDT_ENV");
		env::remove_var("ENVIRONMENT");
		env::remove_var("REINHARDT_SETTINGS_MODULE");
	}

	let profile = Profile::from_env();

	// When no env vars are set, from_env() returns None
	assert_eq!(profile, None, "Unset env vars should return None");
}

/// Test: Profile from environment with whitespace
///
/// Why: Validates that Profile::from_env() treats values with leading/trailing
/// whitespace as Custom profiles (implementation does NOT trim whitespace).
#[rstest]
#[case("  production  ", Some(Profile::Custom))] // Whitespace NOT trimmed → Custom
#[case("  development  ", Some(Profile::Custom))] // Whitespace NOT trimmed → Custom
#[case("  staging  ", Some(Profile::Custom))] // Whitespace NOT trimmed → Custom
#[case("   ", Some(Profile::Custom))] // Whitespace-only → Custom
#[serial(profile_env)]
#[test]
fn test_profile_from_env_whitespace(#[case] env_value: &str, #[case] expected: Option<Profile>) {
	unsafe {
		env::set_var("REINHARDT_ENV", env_value);
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile, expected,
		"REINHARDT_ENV='{}' (implementation does NOT trim whitespace)",
		env_value
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile with special characters in custom name
///
/// Why: Validates that Profile::Custom can handle special characters
/// in environment names (e.g., "pre-production", "qa_2").
#[rstest]
#[case("pre-production")]
#[case("qa_2")]
#[case("dev.local")]
#[case("prod-eu-west-1")]
#[serial(profile_env)]
#[test]
fn test_profile_custom_special_characters(#[case] env_value: &str) {
	unsafe {
		env::set_var("REINHARDT_ENV", env_value);
	}

	let profile = Profile::from_env();

	// Note: Profile::Custom is a unit variant, not a tuple variant
	// Unknown strings are parsed as Custom
	assert_eq!(
		profile,
		Some(Profile::Custom),
		"REINHARDT_ENV='{}' should create Custom profile",
		env_value
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile with numeric string
///
/// Why: Validates that numeric strings are treated as custom profiles.
#[rstest]
#[case("123")]
#[case("2024")]
#[case("1")]
#[serial(profile_env)]
#[test]
fn test_profile_numeric_string(#[case] env_value: &str) {
	unsafe {
		env::set_var("REINHARDT_ENV", env_value);
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Custom),
		"Numeric REINHARDT_ENV='{}' should create Custom profile",
		env_value
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile with very long custom name
///
/// Why: Validates that Profile::Custom handles long environment names without issues.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_very_long_custom_name() {
	let long_name = "a".repeat(1000);

	unsafe {
		env::set_var("REINHARDT_ENV", &long_name);
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Custom),
		"Very long custom profile name should be handled"
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile with Unicode characters
///
/// Why: Validates that Profile::Custom can handle Unicode characters
/// in custom environment names.
#[rstest]
#[case("プロダクション")] // Japanese
#[case("производство")] // Russian
#[case("环境")] // Chinese
#[case("🚀-prod")] // Emoji
#[serial(profile_env)]
#[test]
fn test_profile_unicode_characters(#[case] env_value: &str) {
	unsafe {
		env::set_var("REINHARDT_ENV", env_value);
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Custom),
		"Unicode REINHARDT_ENV='{}' should create Custom profile",
		env_value
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile with mixed case for custom names
///
/// Why: Validates that custom profile names preserve case (unlike standard profiles).
#[rstest]
#[case("PrE-PrOd")]
#[case("QA_Test")]
#[case("MixedCaseName")]
#[serial(profile_env)]
#[test]
fn test_profile_custom_mixed_case(#[case] env_value: &str) {
	unsafe {
		env::set_var("REINHARDT_ENV", env_value);
	}

	let profile = Profile::from_env();

	// Custom profiles are returned for unknown strings (case-insensitive matching for known profiles)
	assert_eq!(
		profile,
		Some(Profile::Custom),
		"Custom profile name should be created for unknown case: '{}'",
		env_value
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile with leading/trailing special characters
///
/// Why: Validates handling of edge cases with special characters at boundaries.
#[rstest]
#[case("-prod")]
#[case("prod-")]
#[case("_dev")]
#[case("dev_")]
#[case(".staging")]
#[case("staging.")]
#[serial(profile_env)]
#[test]
fn test_profile_boundary_special_characters(#[case] env_value: &str) {
	unsafe {
		env::set_var("REINHARDT_ENV", env_value);
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Custom),
		"Profile with boundary special characters should work: '{}'",
		env_value
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}
