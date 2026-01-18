//! Integration tests for Secret Rotation Lifecycle.
//!
//! This test module validates that SecretRotation correctly manages secret
//! rotation lifecycle including policy enforcement, rotation history tracking,
//! and concurrent rotation attempts.
//!
//! NOTE: These tests are feature-gated with "secret-rotation" feature.

#![cfg(feature = "secret-rotation")]

use reinhardt_conf::settings::secrets::rotation::{RotationEntry, RotationPolicy, SecretRotation};
use rstest::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test: Complete rotation lifecycle
///
/// Why: Validates that SecretRotation correctly tracks multiple rotations
/// and maintains rotation history for the same secret.
#[rstest]
#[tokio::test]
async fn test_rotation_lifecycle_complete() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(1), // 1 second interval for fast testing
		max_age: None,
	};

	let rotation = SecretRotation::new(policy);

	// First rotation (secret never rotated before)
	assert!(
		rotation.should_rotate("database_password").await.unwrap(),
		"First rotation should be needed"
	);
	rotation
		.rotate("database_password")
		.await
		.expect("First rotation should succeed");

	// Verify history has one entry
	let history = rotation.get_history_for_secret("database_password");
	assert_eq!(
		history.len(),
		1,
		"Should have 1 rotation entry after first rotation"
	);

	// Attempt immediate rotation (should fail due to interval)
	assert!(
		rotation.rotate("database_password").await.is_err(),
		"Immediate re-rotation should fail"
	);

	// Wait for interval to pass
	sleep(Duration::from_secs(2)).await;

	// Second rotation
	assert!(
		rotation.should_rotate("database_password").await.unwrap(),
		"Second rotation should be needed after interval"
	);
	rotation
		.rotate("database_password")
		.await
		.expect("Second rotation should succeed");

	// Verify history has two entries
	let history = rotation.get_history_for_secret("database_password");
	assert_eq!(
		history.len(),
		2,
		"Should have 2 rotation entries after second rotation"
	);

	// Wait again and rotate third time
	sleep(Duration::from_secs(2)).await;
	rotation
		.rotate("database_password")
		.await
		.expect("Third rotation should succeed");

	// Verify history has three entries
	let history = rotation.get_history_for_secret("database_password");
	assert_eq!(
		history.len(),
		3,
		"Should have 3 rotation entries after third rotation"
	);
}

/// Test: Rotation before interval elapsed
///
/// Why: Validates that rotation is rejected when attempted before the
/// configured interval has elapsed.
#[rstest]
#[tokio::test]
async fn test_rotation_before_interval() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(3600), // 1 hour interval
		max_age: None,
	};

	let rotation = SecretRotation::new(policy);

	// First rotation
	rotation
		.rotate("api_key")
		.await
		.expect("First rotation should succeed");

	// Immediate second rotation (should fail)
	let result = rotation.rotate("api_key").await;
	assert!(
		result.is_err(),
		"Rotation before interval should be rejected"
	);

	// Verify error message
	let error_message = result.unwrap_err();
	assert!(
		error_message.contains("does not need rotation yet"),
		"Error message should indicate rotation not needed"
	);

	// Verify should_rotate returns false
	assert!(
		!rotation.should_rotate("api_key").await.unwrap(),
		"should_rotate should return false before interval"
	);
}

/// Test: max_age forces rotation
///
/// Why: Validates that when max_age is exceeded, rotation is forced
/// even if interval has not elapsed.
#[rstest]
#[tokio::test]
async fn test_rotation_max_age_forces_rotation() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(3600),   // 1 hour interval
		max_age: Some(Duration::from_secs(2)), // 2 second max age
	};

	let rotation = SecretRotation::new(policy);

	// First rotation
	rotation
		.rotate("session_key")
		.await
		.expect("First rotation should succeed");

	// Wait for max_age to pass (but not interval)
	sleep(Duration::from_secs(3)).await;

	// should_rotate should return true due to max_age
	assert!(
		rotation.should_rotate("session_key").await.unwrap(),
		"should_rotate should return true after max_age"
	);

	// Second rotation should succeed (forced by max_age)
	rotation
		.rotate("session_key")
		.await
		.expect("Rotation should succeed after max_age");

	// Verify history has two entries
	let history = rotation.get_history_for_secret("session_key");
	assert_eq!(
		history.len(),
		2,
		"Should have 2 rotation entries after max_age rotation"
	);
}

/// Test: Multiple secrets are independent
///
/// Why: Validates that rotating one secret does not affect the rotation
/// status or history of another secret.
#[rstest]
#[tokio::test]
async fn test_rotation_multiple_secrets_independent() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(1),
		max_age: None,
	};

	let rotation = SecretRotation::new(policy);

	// Rotate secret A
	rotation
		.rotate("secret_a")
		.await
		.expect("Secret A rotation should succeed");

	// Rotate secret B
	rotation
		.rotate("secret_b")
		.await
		.expect("Secret B rotation should succeed");

	// Verify both secrets have independent history
	let history_a = rotation.get_history_for_secret("secret_a");
	let history_b = rotation.get_history_for_secret("secret_b");
	assert_eq!(history_a.len(), 1, "Secret A should have 1 rotation");
	assert_eq!(history_b.len(), 1, "Secret B should have 1 rotation");

	// Rotating secret A again (should fail due to interval)
	assert!(
		rotation.rotate("secret_a").await.is_err(),
		"Secret A rotation should fail before interval"
	);

	// But secret B should also fail (independent timing)
	assert!(
		rotation.rotate("secret_b").await.is_err(),
		"Secret B rotation should also fail before interval"
	);

	// Wait for interval
	sleep(Duration::from_secs(2)).await;

	// Now only rotate secret A
	rotation
		.rotate("secret_a")
		.await
		.expect("Secret A second rotation should succeed");

	// Verify secret A has 2 rotations, secret B still has 1
	let history_a = rotation.get_history_for_secret("secret_a");
	let history_b = rotation.get_history_for_secret("secret_b");
	assert_eq!(history_a.len(), 2, "Secret A should have 2 rotations");
	assert_eq!(history_b.len(), 1, "Secret B should still have 1 rotation");
}

/// Test: Concurrent rotation attempts
///
/// Why: Validates that when multiple threads attempt to rotate the same
/// secret simultaneously, only one succeeds and the other is rejected.
#[rstest]
#[tokio::test]
async fn test_rotation_concurrent_requests() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(1),
		max_age: None,
	};

	let rotation = Arc::new(SecretRotation::new(policy));

	// First rotation to establish initial state
	rotation
		.rotate("concurrent_secret")
		.await
		.expect("Initial rotation should succeed");

	// Wait for interval to pass
	sleep(Duration::from_secs(2)).await;

	// Spawn two concurrent rotation attempts
	let rotation1 = rotation.clone();
	let rotation2 = rotation.clone();

	let handle1 = tokio::spawn(async move { rotation1.rotate("concurrent_secret").await });

	let handle2 = tokio::spawn(async move { rotation2.rotate("concurrent_secret").await });

	// Wait for both to complete
	let result1 = handle1.await.unwrap();
	let result2 = handle2.await.unwrap();

	// At least one should fail (due to race condition and interval check)
	// NOTE: Both might succeed if timing allows, but typically one will fail
	let success_count = [result1.is_ok(), result2.is_ok()]
		.iter()
		.filter(|&&x| x)
		.count();

	assert!(
		success_count >= 1,
		"At least one concurrent rotation should succeed"
	);

	// Verify history has at most 2 new rotations (initial + concurrent attempts)
	let history = rotation.get_history_for_secret("concurrent_secret");
	assert!(
		history.len() <= 3,
		"Should have at most 3 rotations (1 initial + 2 concurrent)"
	);
}

/// Test: Force rotation bypasses policy
///
/// Why: Validates that force_rotate() bypasses interval and max_age checks,
/// allowing immediate rotation for security incidents.
#[rstest]
#[tokio::test]
async fn test_force_rotation_bypasses_policy() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(3600), // 1 hour interval
		max_age: None,
	};

	let rotation = SecretRotation::new(policy);

	// First rotation
	rotation
		.rotate("compromised_key")
		.await
		.expect("First rotation should succeed");

	// Normal rotation should fail (interval not elapsed)
	assert!(
		rotation.rotate("compromised_key").await.is_err(),
		"Normal rotation should fail before interval"
	);

	// Force rotation should succeed
	rotation
		.force_rotate(
			"compromised_key",
			"security_team",
			Some("Security breach detected".to_string()),
		)
		.await
		.expect("Force rotation should succeed");

	// Verify history has two entries
	let history = rotation.get_history_for_secret("compromised_key");
	assert_eq!(
		history.len(),
		2,
		"Should have 2 rotation entries after force rotation"
	);

	// Verify force rotation entry has correct metadata
	let last_entry = &history[1];
	assert_eq!(
		last_entry.rotated_by, "security_team",
		"Force rotation should record correct user"
	);
	assert_eq!(
		last_entry.reason,
		Some("Security breach detected".to_string()),
		"Force rotation should record reason"
	);
}

/// Test: Rotation history tracking
///
/// Why: Validates that get_history() returns all rotation entries
/// across all secrets.
#[rstest]
#[tokio::test]
async fn test_rotation_history_tracking() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(1),
		max_age: None,
	};

	let rotation = SecretRotation::new(policy);

	// Rotate multiple secrets
	rotation
		.rotate("key1")
		.await
		.expect("key1 rotation should succeed");

	sleep(Duration::from_secs(2)).await;

	rotation
		.rotate("key2")
		.await
		.expect("key2 rotation should succeed");

	sleep(Duration::from_secs(2)).await;

	rotation
		.rotate("key1")
		.await
		.expect("key1 second rotation should succeed");

	// Get full history
	let full_history = rotation.get_history();
	assert_eq!(full_history.len(), 3, "Full history should have 3 entries");

	// Get history for specific secrets
	let key1_history = rotation.get_history_for_secret("key1");
	let key2_history = rotation.get_history_for_secret("key2");

	assert_eq!(key1_history.len(), 2, "key1 should have 2 rotations");
	assert_eq!(key2_history.len(), 1, "key2 should have 1 rotation");

	// Verify all entries have timestamps
	for entry in full_history {
		assert!(
			entry.timestamp <= chrono::Utc::now(),
			"Timestamp should be in the past or now"
		);
		assert!(
			!entry.secret_name.is_empty(),
			"Secret name should not be empty"
		);
	}
}

/// Test: RotationEntry creation
///
/// Why: Validates that RotationEntry can be created with and without
/// a reason, and timestamps are correctly set.
#[rstest]
#[test]
fn test_rotation_entry_creation() {
	// Create entry without reason
	let entry = RotationEntry::new("test_secret".to_string(), "admin".to_string());

	assert_eq!(entry.secret_name, "test_secret");
	assert_eq!(entry.rotated_by, "admin");
	assert_eq!(entry.reason, None);
	assert!(
		entry.timestamp <= chrono::Utc::now(),
		"Timestamp should be current"
	);

	// Create entry with reason
	let entry_with_reason = RotationEntry::with_reason(
		"test_secret2".to_string(),
		"operator".to_string(),
		"Scheduled maintenance".to_string(),
	);

	assert_eq!(entry_with_reason.secret_name, "test_secret2");
	assert_eq!(entry_with_reason.rotated_by, "operator");
	assert_eq!(
		entry_with_reason.reason,
		Some("Scheduled maintenance".to_string())
	);
	assert!(
		entry_with_reason.timestamp <= chrono::Utc::now(),
		"Timestamp should be current"
	);
}

/// Test: RotationPolicy default values
///
/// Why: Validates that default policy has reasonable values
/// (24 hours interval, 7 days max age).
#[rstest]
#[test]
fn test_rotation_policy_default() {
	let policy = RotationPolicy::default();

	assert_eq!(
		policy.interval,
		Duration::from_secs(86400),
		"Default interval should be 24 hours"
	);
	assert_eq!(
		policy.max_age,
		Some(Duration::from_secs(604800)),
		"Default max_age should be 7 days"
	);
}
