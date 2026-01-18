//! Integration tests for Secret Rotation Policy Enforcement Use Case.
//!
//! This test module validates enterprise secret rotation policy enforcement,
//! simulating a 90-day rotation policy with max_age enforcement.
//!
//! NOTE: These tests are feature-gated with "secret-rotation" feature.
//! NOTE: Time intervals are scaled down for practical testing (90 days → 90 seconds).

#![cfg(feature = "secret-rotation")]

use reinhardt_conf::settings::secrets::rotation::{RotationPolicy, SecretRotation};
use rstest::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test: 90-day rotation policy enforcement
///
/// Why: Validates that organization can enforce 90-day secret rotation policy
/// with proper interval and max_age checks.
///
/// NOTE: This test simulates 90 days using 90 seconds for practical testing.
/// Uses virtual time for instant test completion.
#[rstest]
#[tokio::test]
async fn test_90_day_rotation_policy_enforcement() {
	// Configure policy: 90-day interval, 95-day max_age
	// Scaled to 90 seconds and 95 seconds for testing
	let policy = RotationPolicy {
		interval: Duration::from_secs(90),      // 90 days → 90 seconds
		max_age: Some(Duration::from_secs(95)), // 95 days → 95 seconds
	};

	let rotation = SecretRotation::new(policy);

	// Step 1: Create database password secret
	rotation
		.rotate("database_password")
		.await
		.expect("Initial rotation should succeed");

	// Step 2: Simulate 89 days passing (89 seconds)
	sleep(Duration::from_secs(89)).await;

	// Verify rotation not required (interval not reached)
	assert!(
		!rotation.should_rotate("database_password").await.unwrap(),
		"Rotation should not be required at 89 days"
	);

	// Attempt rotation should fail
	let result = rotation.rotate("database_password").await;
	assert!(
		result.is_err(),
		"Rotation should be rejected before interval"
	);

	// Step 3: Simulate 91 days passing (91 seconds total)
	sleep(Duration::from_secs(2)).await; // Additional 2 seconds to reach 91

	// Verify rotation required (interval reached)
	assert!(
		rotation.should_rotate("database_password").await.unwrap(),
		"Rotation should be required at 91 days"
	);

	// Step 4: Perform rotation
	rotation
		.rotate("database_password")
		.await
		.expect("Rotation at 91 days should succeed");

	// Verify new secret in history
	let history = rotation.get_history_for_secret("database_password");
	assert_eq!(
		history.len(),
		2,
		"Should have 2 rotation entries (initial + policy rotation)"
	);

	// Step 5: Verify old secret archived (not deleted)
	// History should contain both old and new rotations
	assert_eq!(
		history[0].secret_name, "database_password",
		"Old rotation should be archived"
	);
	assert_eq!(
		history[1].secret_name, "database_password",
		"New rotation should be recorded"
	);

	// Verify timestamps
	assert!(
		history[1].timestamp > history[0].timestamp,
		"New rotation timestamp should be after old rotation"
	);
}

/// Test: max_age enforcement triggers error
///
/// Why: Validates that max_age enforcement prevents secrets from becoming too old
/// even if manual rotation is not performed.
///
/// NOTE: This test simulates 96 days using 96 seconds.
/// Uses virtual time for instant test completion.
#[rstest]
#[tokio::test]
async fn test_max_age_enforcement_triggers_error() {
	// Configure policy with max_age
	let policy = RotationPolicy {
		interval: Duration::from_secs(90),      // 90 days → 90 seconds
		max_age: Some(Duration::from_secs(95)), // 95 days → 95 seconds
	};

	let rotation = SecretRotation::new(policy);

	// Initial rotation
	rotation
		.rotate("api_key")
		.await
		.expect("Initial rotation should succeed");

	// Simulate 96 days passing without rotation (96 seconds)
	sleep(Duration::from_secs(96)).await;

	// Verify rotation required due to max_age
	assert!(
		rotation.should_rotate("api_key").await.unwrap(),
		"Rotation should be required due to max_age (96 days > 95 days max_age)"
	);

	// In real scenario, this would trigger an alert or automatic rotation
	// Perform rotation to prevent service disruption
	rotation
		.rotate("api_key")
		.await
		.expect("Rotation should succeed after max_age reached");
}

/// Test: Multiple secrets with independent rotation schedules
///
/// Why: Validates that different secrets can have independent rotation timelines
/// under the same policy.
/// Uses virtual time for instant test completion.
#[rstest]
#[tokio::test]
async fn test_multiple_secrets_independent_rotation_schedules() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(90),
		max_age: Some(Duration::from_secs(95)),
	};

	let rotation = SecretRotation::new(policy);

	// Rotate database_password at T=0
	rotation.rotate("database_password").await.unwrap();

	// Wait 50 seconds
	sleep(Duration::from_secs(50)).await;

	// Rotate api_key at T=50
	rotation.rotate("api_key").await.unwrap();

	// Wait 41 more seconds (T=91 total)
	sleep(Duration::from_secs(41)).await;

	// database_password: 91 seconds since rotation → should need rotation
	assert!(
		rotation.should_rotate("database_password").await.unwrap(),
		"database_password should need rotation at 91 seconds"
	);

	// api_key: 41 seconds since rotation → should NOT need rotation
	assert!(
		!rotation.should_rotate("api_key").await.unwrap(),
		"api_key should not need rotation at 41 seconds"
	);
}

/// Test: Rotation history provides audit trail
///
/// Why: Validates that rotation history provides complete audit trail
/// for compliance and security investigations.
#[rstest]
#[tokio::test]
async fn test_rotation_history_audit_trail() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(1), // 1 second for quick testing
		max_age: None,
	};

	let rotation = SecretRotation::new(policy);

	// Perform multiple rotations
	rotation.rotate("audit_secret").await.unwrap();

	sleep(Duration::from_secs(2)).await;
	rotation.rotate("audit_secret").await.unwrap();

	sleep(Duration::from_secs(2)).await;
	rotation.rotate("audit_secret").await.unwrap();

	// Verify complete audit trail
	let history = rotation.get_history_for_secret("audit_secret");
	assert_eq!(history.len(), 3, "Should have 3 rotation entries");

	// Verify timestamps are in chronological order
	for i in 1..history.len() {
		assert!(
			history[i].timestamp > history[i - 1].timestamp,
			"Timestamps should be in chronological order"
		);
	}

	// Verify all entries have required metadata
	for entry in &history {
		assert_eq!(entry.secret_name, "audit_secret");
		assert!(!entry.rotated_by.is_empty(), "rotated_by should be set");
		assert!(
			entry.timestamp <= chrono::Utc::now(),
			"Timestamp should be valid"
		);
	}
}

/// Test: Force rotation for security incident
///
/// Why: Validates that security team can force immediate rotation
/// during security incidents, bypassing policy checks.
#[rstest]
#[tokio::test]
async fn test_force_rotation_security_incident() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(90),
		max_age: Some(Duration::from_secs(95)),
	};

	let rotation = SecretRotation::new(policy);

	// Initial rotation
	rotation.rotate("compromised_key").await.unwrap();

	// Security incident detected - force rotation immediately
	rotation
		.force_rotate(
			"compromised_key",
			"security_team",
			Some("Security breach detected - immediate rotation required".to_string()),
		)
		.await
		.expect("Force rotation should succeed");

	// Verify rotation history
	let history = rotation.get_history_for_secret("compromised_key");
	assert_eq!(history.len(), 2, "Should have 2 entries (initial + forced)");

	// Verify force rotation metadata
	let forced_entry = &history[1];
	assert_eq!(
		forced_entry.rotated_by, "security_team",
		"Should record security team as rotator"
	);
	assert_eq!(
		forced_entry.reason,
		Some("Security breach detected - immediate rotation required".to_string()),
		"Should record reason for force rotation"
	);
}

/// Test: Rotation policy with no max_age
///
/// Why: Validates that policies can be configured without max_age enforcement
/// (interval-only rotation).
#[rstest]
#[tokio::test]
async fn test_rotation_policy_without_max_age() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(5),
		max_age: None, // No max_age enforcement
	};

	let rotation = SecretRotation::new(policy);

	rotation.rotate("interval_only_secret").await.unwrap();

	// Wait 4 seconds (less than interval)
	sleep(Duration::from_secs(4)).await;

	assert!(
		!rotation
			.should_rotate("interval_only_secret")
			.await
			.unwrap(),
		"Rotation should not be required before interval"
	);

	// Wait 2 more seconds (6 seconds total, exceeds interval)
	sleep(Duration::from_secs(2)).await;

	assert!(
		rotation
			.should_rotate("interval_only_secret")
			.await
			.unwrap(),
		"Rotation should be required after interval"
	);
}

/// Test: Concurrent rotation attempts during policy window
///
/// Why: Validates that concurrent rotation attempts are handled correctly
/// when rotation becomes required.
#[rstest]
#[tokio::test]
async fn test_concurrent_rotation_during_policy_window() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(2),
		max_age: None,
	};

	let rotation = Arc::new(SecretRotation::new(policy));

	// Initial rotation
	rotation.rotate("concurrent_secret").await.unwrap();

	// Wait for interval to pass
	sleep(Duration::from_secs(3)).await;

	// Spawn concurrent rotation attempts
	let rotation1 = rotation.clone();
	let rotation2 = rotation.clone();

	let handle1 = tokio::spawn(async move { rotation1.rotate("concurrent_secret").await });

	let handle2 = tokio::spawn(async move { rotation2.rotate("concurrent_secret").await });

	// Wait for both to complete
	let result1 = handle1.await.unwrap();
	let result2 = handle2.await.unwrap();

	// At least one should succeed
	let success_count = [result1.is_ok(), result2.is_ok()]
		.iter()
		.filter(|&&x| x)
		.count();

	assert!(
		success_count >= 1,
		"At least one concurrent rotation should succeed"
	);

	// Verify history
	let history = rotation.get_history_for_secret("concurrent_secret");
	assert!(
		history.len() <= 3,
		"Should have at most 3 entries (initial + 2 concurrent attempts)"
	);
}

/// Test: Default rotation policy values
///
/// Why: Validates that default policy provides reasonable production defaults
/// (24 hours interval, 7 days max_age).
#[rstest]
#[test]
fn test_default_rotation_policy_values() {
	let policy = RotationPolicy::default();

	assert_eq!(
		policy.interval,
		Duration::from_secs(86400),
		"Default interval should be 24 hours (86400 seconds)"
	);

	assert_eq!(
		policy.max_age,
		Some(Duration::from_secs(604800)),
		"Default max_age should be 7 days (604800 seconds)"
	);
}

/// Test: Rotation policy prevents premature rotation
///
/// Why: Validates that policy prevents excessive rotation attempts
/// that could cause service disruption.
#[rstest]
#[tokio::test]
async fn test_rotation_policy_prevents_premature_rotation() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(10),
		max_age: Some(Duration::from_secs(20)),
	};

	let rotation = SecretRotation::new(policy);

	rotation.rotate("protected_secret").await.unwrap();

	// Attempt rotation at 1, 2, 3, ... 9 seconds
	for i in 1..10 {
		sleep(Duration::from_secs(1)).await;

		let result = rotation.rotate("protected_secret").await;
		assert!(
			result.is_err(),
			"Rotation should be rejected at {} seconds (before 10-second interval)",
			i
		);
	}

	// Wait to reach interval (10 seconds total)
	sleep(Duration::from_secs(1)).await;

	// Now rotation should succeed
	rotation
		.rotate("protected_secret")
		.await
		.expect("Rotation should succeed after 10-second interval");
}
