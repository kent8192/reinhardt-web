//! Integration tests for Profile environment-based detection.
//!
//! This test module validates the Profile enum's ability to detect the application
//! environment from environment variables, with support for development, staging,
//! production, and custom profiles.

use reinhardt_conf::settings::profile::Profile;
use rstest::*;
use serial_test::serial;
use std::env;

/// Test: Profile from environment - Development
///
/// Why: Validates that Profile::from_env() correctly detects Development profile.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_from_env_development() {
	unsafe {
		env::set_var("REINHARDT_ENV", "development");
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Development),
		"REINHARDT_ENV='development' should return Development profile"
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile from environment - Staging
///
/// Why: Validates that Profile::from_env() correctly detects Staging profile.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_from_env_staging() {
	unsafe {
		env::set_var("REINHARDT_ENV", "staging");
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Staging),
		"REINHARDT_ENV='staging' should return Staging profile"
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile from environment - Production
///
/// Why: Validates that Profile::from_env() correctly detects Production profile.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_from_env_production() {
	unsafe {
		env::set_var("REINHARDT_ENV", "production");
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Production),
		"REINHARDT_ENV='production' should return Production profile"
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile from environment - Custom string
///
/// Why: Validates that unknown profile names are mapped to Profile::Custom.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_custom_string() {
	unsafe {
		env::set_var("REINHARDT_ENV", "custom_name");
	}

	let profile = Profile::from_env();

	// Note: Profile::Custom is a unit variant, not a tuple variant
	assert_eq!(
		profile,
		Some(Profile::Custom),
		"REINHARDT_ENV='custom_name' should return Custom profile"
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile from environment - Case insensitive
///
/// Why: Validates that profile detection is case-insensitive for known profiles.
#[rstest]
#[case("PRODUCTION", Some(Profile::Production))]
#[case("Production", Some(Profile::Production))]
#[case("DEVELOPMENT", Some(Profile::Development))]
#[case("Development", Some(Profile::Development))]
#[case("STAGING", Some(Profile::Staging))]
#[case("Staging", Some(Profile::Staging))]
#[serial(profile_env)]
#[test]
fn test_profile_case_insensitive(#[case] env_value: &str, #[case] expected: Option<Profile>) {
	unsafe {
		env::set_var("REINHARDT_ENV", env_value);
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile, expected,
		"REINHARDT_ENV='{}' should be case-insensitive",
		env_value
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
	}
}

/// Test: Profile from ENVIRONMENT variable
///
/// Why: Validates that Profile::from_env() also checks ENVIRONMENT variable
/// as a fallback to REINHARDT_ENV.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_from_environment_var() {
	unsafe {
		env::remove_var("REINHARDT_ENV");
		env::set_var("ENVIRONMENT", "production");
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Production),
		"ENVIRONMENT='production' should return Production profile"
	);

	unsafe {
		env::remove_var("ENVIRONMENT");
	}
}

/// Test: Profile priority - REINHARDT_ENV over ENVIRONMENT
///
/// Why: Validates that REINHARDT_ENV takes priority when both are set.
#[rstest]
#[serial(profile_env)]
#[test]
fn test_profile_priority() {
	unsafe {
		env::set_var("REINHARDT_ENV", "development");
		env::set_var("ENVIRONMENT", "production");
	}

	let profile = Profile::from_env();

	assert_eq!(
		profile,
		Some(Profile::Development),
		"REINHARDT_ENV should take priority over ENVIRONMENT"
	);

	unsafe {
		env::remove_var("REINHARDT_ENV");
		env::remove_var("ENVIRONMENT");
	}
}
