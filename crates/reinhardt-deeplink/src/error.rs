//! Error types for deeplink configuration and handling.

use thiserror::Error;

/// Errors that can occur during deeplink configuration and handling.
#[derive(Debug, Error)]
pub enum DeeplinkError {
	/// Invalid iOS app ID format.
	///
	/// App IDs must follow the format `TEAM_ID.bundle_identifier`.
	#[error("invalid iOS app ID format: {0}. Expected format: TEAM_ID.bundle_identifier")]
	InvalidAppId(String),

	/// Invalid Android SHA256 fingerprint format.
	///
	/// Fingerprints must be 32 colon-separated hex bytes (e.g., `FA:C6:17:...`).
	#[error("invalid Android fingerprint format: {0}. Expected 32 colon-separated hex bytes")]
	InvalidFingerprint(String),

	/// No paths specified for iOS Universal Links.
	#[error("no paths specified for iOS Universal Links")]
	NoPathsSpecified,

	/// Package name is required for Android App Links.
	#[error("package name required for Android App Links")]
	MissingPackageName,

	/// At least one SHA256 fingerprint is required for Android.
	#[error("at least one SHA256 fingerprint required for Android")]
	MissingFingerprint,

	/// iOS configuration is required but not provided.
	#[error("iOS configuration required but not provided")]
	MissingIosConfig,

	/// Android configuration is required but not provided.
	#[error("Android configuration required but not provided")]
	MissingAndroidConfig,

	/// JSON serialization failed.
	#[error("serialization failed: {0}")]
	Serialization(#[from] serde_json::Error),
}

/// Validates an iOS app ID format.
///
/// Valid format: `TEAM_ID.bundle_identifier` where:
/// - TEAM_ID is typically 10 alphanumeric characters
/// - bundle_identifier follows reverse domain notation
///
/// # Errors
///
/// Returns `DeeplinkError::InvalidAppId` if the format is invalid.
pub fn validate_app_id(app_id: &str) -> Result<(), DeeplinkError> {
	// Must contain at least one dot separating team ID and bundle ID
	if !app_id.contains('.') {
		return Err(DeeplinkError::InvalidAppId(app_id.to_string()));
	}

	// Split into team ID and bundle ID
	let parts: Vec<&str> = app_id.splitn(2, '.').collect();
	if parts.len() != 2 {
		return Err(DeeplinkError::InvalidAppId(app_id.to_string()));
	}

	let team_id = parts[0];
	let bundle_id = parts[1];

	// Team ID should be alphanumeric (typically 10 characters, but we allow flexibility)
	if team_id.is_empty() || !team_id.chars().all(|c| c.is_ascii_alphanumeric()) {
		return Err(DeeplinkError::InvalidAppId(app_id.to_string()));
	}

	// Bundle ID should not be empty and should follow valid identifier rules
	if bundle_id.is_empty() {
		return Err(DeeplinkError::InvalidAppId(app_id.to_string()));
	}

	Ok(())
}

/// Validates an Android SHA256 fingerprint format.
///
/// Valid format: 32 colon-separated hex bytes (e.g., `FA:C6:17:45:...`).
///
/// # Errors
///
/// Returns `DeeplinkError::InvalidFingerprint` if the format is invalid.
pub fn validate_fingerprint(fingerprint: &str) -> Result<(), DeeplinkError> {
	let parts: Vec<&str> = fingerprint.split(':').collect();

	// Must have exactly 32 bytes
	if parts.len() != 32 {
		return Err(DeeplinkError::InvalidFingerprint(fingerprint.to_string()));
	}

	// Each part must be exactly 2 hex characters
	for part in parts {
		if part.len() != 2 || !part.chars().all(|c| c.is_ascii_hexdigit()) {
			return Err(DeeplinkError::InvalidFingerprint(fingerprint.to_string()));
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

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
}
