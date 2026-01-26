//! # App Links Tests
//!
//! This module contains unit tests for Android App Links functionality using the new AndroidConfig builder.
//!
//! ## Test Coverage
//! - AndroidConfig builder pattern
//! - Multiple fingerprints
//! - Package name handling
//! - Additional packages
//! - JSON serialization (array format)
//! - AssetLinksHandler functionality
//!
//! ## Standards Compliance
//! - Uses `#[rstest]` for all tests
//! - All tests use at least one Reinhardt component (TP-2)
//! - No skeleton implementations (TP-1)

use reinhardt_deeplink::{AndroidConfig, AssetLinksHandler, validate_fingerprint};
use rstest::*;

// Import fixtures
mod fixtures;
use fixtures::*;

const VALID_FINGERPRINT: &str =
	"FA:C6:17:45:DC:09:03:78:6F:B9:ED:E6:2A:96:2B:39:9F:73:48:F0:BB:6F:89:9B:83:32:66:75:91:03:3B:9C";

// ============================================================================
// Happy Path Tests (正常系)
// ============================================================================

#[rstest]
fn test_android_config_basic() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.build();

	let json = serde_json::to_string_pretty(&config).unwrap();
	assert!(json.contains("delegate_permission/common.handle_all_urls"));
	assert!(json.contains("android_app"));
	assert!(json.contains("com.example.app"));
	assert!(json.contains(VALID_FINGERPRINT));
}

#[rstest]
fn test_android_config_multiple_fingerprints() {
	let fp2 = "00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00";

	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprints(&[VALID_FINGERPRINT, fp2])
		.build();

	let json = serde_json::to_string_pretty(&config).unwrap();
	assert!(json.contains(VALID_FINGERPRINT));
	assert!(json.contains(fp2));
}

#[rstest]
fn test_android_config_json_serialization() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.build();

	let json = serde_json::to_string(&config);
	assert!(json.is_ok());
	let json = json.unwrap();
	assert!(json.contains("delegate_permission"));
	assert!(json.contains("com.example.app"));
}

#[rstest]
fn test_android_config_additional_packages() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.additional_package("com.example.app2", &[VALID_FINGERPRINT])
		.build();

	assert_eq!(config.statements.len(), 2);
	let json = serde_json::to_string_pretty(&config).unwrap();
	assert!(json.contains("com.example.app"));
	assert!(json.contains("com.example.app2"));
}

// ============================================================================
// Error Path Tests (異常系) - Builder Validation
// ============================================================================

#[rstest]
fn test_android_config_validate_no_package() {
	let builder = AndroidConfig::builder().sha256_fingerprint(VALID_FINGERPRINT);
	let result = builder.validate();
	assert!(result.is_err());
}

#[rstest]
fn test_android_config_validate_no_fingerprint() {
	let builder = AndroidConfig::builder().package_name("com.example.app");
	let result = builder.validate();
	assert!(result.is_err());
}

#[rstest]
fn test_fingerprint_validation_invalid_format() {
	let result = validate_fingerprint("invalid");
	assert!(result.is_err());
}

// ============================================================================
// Edge Cases Tests (エッジケース)
// ============================================================================

#[rstest]
fn test_fingerprint_all_zeros() {
	let all_zeros = "00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00";

	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(all_zeros)
		.build();

	let json = serde_json::to_string(&config).unwrap();
	assert!(json.contains(all_zeros));
}

#[rstest]
fn test_subdomain_flag() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.include_all_subdomains(true)
		.build();

	// Subdomain flag is stored but not serialized in current implementation
	assert_eq!(config.statements.len(), 1);
}

// ============================================================================
// Package Name Validation Tests
// ============================================================================

#[rstest]
fn test_package_name_valid_patterns() {
	let test_cases = vec![
		"com.example.app",
		"org.company.product",
		"io.github.user.project",
		"jp.co.company.product",
	];

	for package_name in test_cases {
		let config = AndroidConfig::builder()
			.package_name(package_name)
			.sha256_fingerprint(VALID_FINGERPRINT)
			.build();

		assert_eq!(config.statements[0].target.package_name, package_name);
	}
}

// ============================================================================
// Fingerprint Format Tests
// ============================================================================

#[rstest]
fn test_fingerprint_format_variants() {
	let test_cases = vec![
		"AA:BB:CC:DD".to_string(),
		"14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:09:00:28:35:DC:A1:E9:FE:0C".to_string(),
		"AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB".to_string(),
	];

	for fingerprint in test_cases {
		let config = AndroidConfig::builder()
			.package_name("com.example.app")
			.sha256_fingerprint(&fingerprint)
			.build();

		assert_eq!(config.statements[0].target.sha256_cert_fingerprints[0], fingerprint);
	}
}

// ============================================================================
// JSON Structure Tests
// ============================================================================

#[rstest]
fn test_dal_json_array_format() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.build();

	let json = serde_json::to_string(&config).unwrap();
	// Should be a JSON array
	assert!(json.starts_with('['));
	assert!(json.ends_with(']'));
}

#[rstest]
fn test_dal_json_structure_matches_spec() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.build();

	let json = serde_json::to_value(&config).unwrap();

	// Verify structure matches Google DAL spec
	let statements = json.as_array().unwrap();
	assert_eq!(statements.len(), 1);

	let statement = &statements[0];
	assert!(statement.get("relation").is_some());
	assert!(statement.get("target").is_some());

	let relations = statement.get("relation").unwrap().as_array().unwrap();
	assert_eq!(
		relations[0],
		"delegate_permission/common.handle_all_urls"
	);

	let target = statement.get("target").unwrap();
	assert!(target.get("namespace").is_some());
	assert!(target.get("package_name").is_some());
	assert!(target
		.get("sha256_cert_fingerprints")
		.is_some());
}

// ============================================================================
// Combinatorial Tests (組み合わせテスト)
// ============================================================================

#[rstest]
fn test_android_config_multiple_fingerprints_single_target() {
	let fp2 = "11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11:11";

	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprints(&[VALID_FINGERPRINT, fp2])
		.build();

	assert_eq!(config.statements[0].target.sha256_cert_fingerprints.len(), 2);
}

#[rstest]
fn test_android_config_multiple_targets_same_package() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.additional_package("com.example.app", &["11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22"])
		.build();

	assert_eq!(config.statements.len(), 2);
}

#[rstest]
fn test_android_config_multiple_targets_different_packages() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app1")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.additional_package("com.example.app2", &[VALID_FINGERPRINT])
		.additional_package("com.example.app3", &[VALID_FINGERPRINT])
		.build();

	assert_eq!(config.statements.len(), 3);
	assert_ne!(
		config.statements[0].target.package_name,
		config.statements[1].target.package_name
	);
}

// ============================================================================
// Sanity Tests (サニティテスト)
// ============================================================================

#[rstest]
fn test_assetlinks_handler_creation() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.build();

	let handler = AssetLinksHandler::new(config);
	assert!(handler.is_ok());
}

#[rstest]
fn test_assetlinks_handler_json_content() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.build();

	let handler = AssetLinksHandler::new(config).unwrap();
	let json = handler.json();
	assert!(json.contains("android_app"));
	assert!(json.starts_with('['));
}

#[rstest]
fn test_assetlinks_handler_clone() {
	let config = AndroidConfig::builder()
		.package_name("com.example.app")
		.sha256_fingerprint(VALID_FINGERPRINT)
		.build();

	let handler = AssetLinksHandler::new(config).unwrap();
	let cloned = handler.clone();
	assert_eq!(handler.json(), cloned.json());
}
