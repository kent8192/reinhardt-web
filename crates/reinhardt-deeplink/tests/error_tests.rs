//! Error module tests
//!
//! Tests for DeeplinkError and DeeplinkResult covering:
//! - Sanity: Type alias correctness, trait implementations, message formatting
//! - New error types: InvalidAppId, InvalidFingerprint, NoPathsSpecified,
//!   MissingPackageName, MissingFingerprint, MissingIosConfig,
//!   MissingAndroidConfig, Serialization

use reinhardt_deeplink::{DeeplinkError, validate_app_id, validate_fingerprint};
use rstest::*;

// ============================================================================
// Sanity Tests
// ============================================================================

#[rstest]
fn test_error_type_alias() {
	// DeeplinkError is the main error type
	let error = DeeplinkError::InvalidAppId("test".to_string());
	assert!(matches!(error, DeeplinkError::InvalidAppId(_)));

	let error = DeeplinkError::NoPathsSpecified;
	assert!(matches!(error, DeeplinkError::NoPathsSpecified));
}

#[rstest]
fn test_error_send_sync() {
	// All error types should be Send + Sync
	fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<DeeplinkError>();
}

#[rstest]
fn test_error_display_invalid_app_id() {
	let error = DeeplinkError::InvalidAppId("invalid".to_string());
	let message = error.to_string();
	assert!(message.contains("invalid iOS app ID format"));
	assert!(message.contains("invalid"));
}

#[rstest]
fn test_error_display_invalid_fingerprint() {
	let error = DeeplinkError::InvalidFingerprint("bad".to_string());
	let message = error.to_string();
	assert!(message.contains("invalid Android fingerprint format"));
	assert!(message.contains("bad"));
}

#[rstest]
fn test_error_display_no_paths() {
	let error = DeeplinkError::NoPathsSpecified;
	let message = error.to_string();
	assert!(message.contains("no paths specified"));
}

#[rstest]
fn test_error_display_missing_package() {
	let error = DeeplinkError::MissingPackageName;
	let message = error.to_string();
	assert!(message.contains("package name required"));
}

#[rstest]
fn test_error_display_missing_fingerprint() {
	let error = DeeplinkError::MissingFingerprint;
	let message = error.to_string();
	assert!(message.contains("at least one SHA256 fingerprint required"));
}

#[rstest]
fn test_error_display_missing_ios_config() {
	let error = DeeplinkError::MissingIosConfig;
	let message = error.to_string();
	assert!(message.contains("iOS configuration required"));
}

#[rstest]
fn test_error_display_missing_android_config() {
	let error = DeeplinkError::MissingAndroidConfig;
	let message = error.to_string();
	assert!(message.contains("Android configuration required"));
}

#[rstest]
fn test_error_from_serde_json() {
	// Create a JSON error by parsing invalid JSON
	let json_error = serde_json::from_str::<serde_json::Value>("{invalid json}").unwrap_err();
	let error = DeeplinkError::from(json_error);
	assert!(matches!(error, DeeplinkError::Serialization(_)));
}

// ============================================================================
// Validation Function Tests
// ============================================================================

#[rstest]
#[case("TEAM123456.com.example.app", true)]
#[case("ABC123XYZ0.com.example.myapp", true)]
#[case("TEAM.bundle", true)]
#[case("invalid", false)]
#[case("", false)]
#[case(".com.example", false)]
#[case("TEAM.", false)]
fn test_validate_app_id(#[case] app_id: &str, #[case] expected_valid: bool) {
	let result = validate_app_id(app_id);
	assert_eq!(result.is_ok(), expected_valid, "app_id: {}", app_id);
}

#[rstest]
#[case(
	"FA:C6:17:45:DC:09:03:78:6F:B9:ED:E6:2A:96:2B:39:9F:73:48:F0:BB:6F:89:9B:83:32:66:75:91:03:3B:9C",
	true
)]
#[case(
	"00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00",
	true
)]
#[case("invalid", false)]
#[case("", false)]
#[case("FA:C6:17", false)]
#[case(
	"FA:C6:17:45:DC:09:03:78:6F:B9:ED:E6:2A:96:2B:39:9F:73:48:F0:BB:6F:89:9B:83:32:66:75:91:03:3B:XX",
	false
)]
fn test_validate_fingerprint(#[case] fingerprint: &str, #[case] expected_valid: bool) {
	let result = validate_fingerprint(fingerprint);
	assert_eq!(
		result.is_ok(),
		expected_valid,
		"fingerprint: {}",
		fingerprint
	);
}

// ============================================================================
// Error Display Format Tests
// ============================================================================

#[rstest]
fn test_error_debug_format() {
	let error = DeeplinkError::InvalidAppId("test".to_string());
	let debug_str = format!("{:?}", error);
	assert!(debug_str.contains("test"));
}

#[rstest]
fn test_error_serialization_message() {
	// Create a JSON error by parsing invalid JSON
	let json_error = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
	let error = DeeplinkError::Serialization(json_error);
	let message = error.to_string();
	assert!(message.contains("serialization failed"));
}
