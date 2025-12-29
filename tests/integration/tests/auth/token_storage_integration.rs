//! Token Storage Integration Tests
//!
//! Comprehensive tests for token storage functionality including:
//! - Happy path operations (store, get, delete)
//! - Error handling (not found, expired tokens)
//! - State transitions (token lifecycle)
//! - Concurrent operations
//! - Edge cases

use reinhardt_auth::{InMemoryTokenStorage, StoredToken, TokenStorage};
use rstest::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Barrier;

// =============================================================================
// Test Fixtures
// =============================================================================

/// In-memory token storage fixture
#[fixture]
fn storage() -> InMemoryTokenStorage {
	InMemoryTokenStorage::new()
}

/// Creates a test token with standard settings
fn create_test_token(token: &str, user_id: i64, expires_in: Option<i64>) -> StoredToken {
	let current_time = chrono::Utc::now().timestamp();
	let expires_at = expires_in.map(|e| current_time + e);

	StoredToken::new(token, user_id)
		.with_expiration(expires_at)
		.with_metadata(HashMap::from([
			("scope".to_string(), "read write".to_string()),
			("ip".to_string(), "127.0.0.1".to_string()),
		]))
}

/// Creates an expired test token
fn create_expired_token(token: &str, user_id: i64) -> StoredToken {
	let past_time = chrono::Utc::now().timestamp() - 3600; // 1 hour ago

	StoredToken::new(token, user_id).with_expiration(Some(past_time))
}

// =============================================================================
// Happy Path Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_store_and_get_token_basic(storage: InMemoryTokenStorage) {
	// Arrange
	let token = create_test_token("test-token-123", 42, Some(3600));

	// Act
	storage
		.store(token.clone())
		.await
		.expect("Store should succeed");

	let retrieved = storage
		.get("test-token-123")
		.await
		.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.token(), "test-token-123");
	assert_eq!(retrieved.user_id(), 42);
	assert!(retrieved.expires_at().is_some());
	assert!(!retrieved.metadata().is_empty());
}

#[rstest]
#[tokio::test]
async fn test_store_multiple_tokens_for_same_user(storage: InMemoryTokenStorage) {
	// Arrange
	let token1 = create_test_token("token-1", 42, Some(3600));
	let token2 = create_test_token("token-2", 42, Some(7200));
	let token3 = create_test_token("token-3", 42, Some(1800));

	// Act
	storage
		.store(token1)
		.await
		.expect("Store token1 should succeed");
	storage
		.store(token2)
		.await
		.expect("Store token2 should succeed");
	storage
		.store(token3)
		.await
		.expect("Store token3 should succeed");

	let user_tokens = storage
		.get_user_tokens(42)
		.await
		.expect("Get user tokens should succeed");

	// Assert
	assert_eq!(user_tokens.len(), 3, "User should have 3 tokens");

	let token_values: Vec<&str> = user_tokens.iter().map(|t| t.token()).collect();
	assert!(token_values.contains(&"token-1"));
	assert!(token_values.contains(&"token-2"));
	assert!(token_values.contains(&"token-3"));
}

#[rstest]
#[tokio::test]
async fn test_delete_single_token(storage: InMemoryTokenStorage) {
	// Arrange
	let token = create_test_token("delete-me", 42, Some(3600));
	storage.store(token).await.expect("Store should succeed");

	// Verify token exists
	let exists = storage.get("delete-me").await;
	assert!(exists.is_ok(), "Token should exist before deletion");

	// Act
	storage
		.delete("delete-me")
		.await
		.expect("Delete should succeed");

	// Assert
	let result = storage.get("delete-me").await;
	assert!(result.is_err(), "Token should not exist after deletion");
}

#[rstest]
#[tokio::test]
async fn test_delete_all_user_tokens(storage: InMemoryTokenStorage) {
	// Arrange
	let user_id = 42;
	for i in 0..5 {
		let token = create_test_token(&format!("token-{}", i), user_id, Some(3600));
		storage.store(token).await.expect("Store should succeed");
	}

	// Verify tokens exist
	let before = storage
		.get_user_tokens(user_id)
		.await
		.expect("Get should work");
	assert_eq!(before.len(), 5, "Should have 5 tokens before deletion");

	// Act
	storage
		.delete_user_tokens(user_id)
		.await
		.expect("Delete user tokens should succeed");

	// Assert
	let after = storage
		.get_user_tokens(user_id)
		.await
		.expect("Get should work");
	assert!(after.is_empty(), "Should have no tokens after deletion");
}

#[rstest]
#[tokio::test]
async fn test_cleanup_expired_tokens(storage: InMemoryTokenStorage) {
	// Arrange
	let current_time = chrono::Utc::now().timestamp();

	// Create mix of expired and valid tokens
	let expired1 = StoredToken::new("expired-1", 1).with_expiration(Some(current_time - 3600));
	let expired2 = StoredToken::new("expired-2", 2).with_expiration(Some(current_time - 1));
	let valid1 = StoredToken::new("valid-1", 3).with_expiration(Some(current_time + 3600));
	let valid2 = StoredToken::new("valid-2", 4).with_expiration(None); // No expiration

	storage.store(expired1).await.expect("Store expired1");
	storage.store(expired2).await.expect("Store expired2");
	storage.store(valid1).await.expect("Store valid1");
	storage.store(valid2).await.expect("Store valid2");

	// Act
	let removed = storage
		.cleanup_expired(current_time)
		.await
		.expect("Cleanup should succeed");

	// Assert
	assert_eq!(removed, 2, "Should have removed 2 expired tokens");

	// Verify valid tokens still exist
	assert!(storage.get("valid-1").await.is_ok(), "valid-1 should exist");
	assert!(storage.get("valid-2").await.is_ok(), "valid-2 should exist");

	// Verify expired tokens are gone
	assert!(
		storage.get("expired-1").await.is_err(),
		"expired-1 should be removed"
	);
	assert!(
		storage.get("expired-2").await.is_err(),
		"expired-2 should be removed"
	);
}

// =============================================================================
// Error Path Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_get_nonexistent_token(storage: InMemoryTokenStorage) {
	// Act
	let result = storage.get("nonexistent-token").await;

	// Assert
	assert!(result.is_err(), "Getting nonexistent token should fail");
}

#[rstest]
#[tokio::test]
async fn test_delete_nonexistent_token_is_idempotent(storage: InMemoryTokenStorage) {
	// Act - Deleting a token that doesn't exist should not error
	let result = storage.delete("nonexistent-token").await;

	// Assert - This should succeed (idempotent operation)
	assert!(result.is_ok(), "Deleting nonexistent token should succeed");
}

#[rstest]
#[tokio::test]
async fn test_get_user_tokens_empty_for_unknown_user(storage: InMemoryTokenStorage) {
	// Act
	let result = storage
		.get_user_tokens(99999)
		.await
		.expect("Should succeed");

	// Assert
	assert!(result.is_empty(), "Unknown user should have no tokens");
}

// =============================================================================
// State Transition Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_token_update_overwrites_existing(storage: InMemoryTokenStorage) {
	// Arrange - Store initial token
	let token_v1 = StoredToken::new("update-token", 42)
		.with_expiration(Some(chrono::Utc::now().timestamp() + 3600))
		.with_metadata(HashMap::from([("version".to_string(), "1".to_string())]));

	storage
		.store(token_v1)
		.await
		.expect("Store v1 should succeed");

	// Act - Update with new version
	let token_v2 = StoredToken::new("update-token", 42)
		.with_expiration(Some(chrono::Utc::now().timestamp() + 7200))
		.with_metadata(HashMap::from([("version".to_string(), "2".to_string())]));

	storage
		.store(token_v2)
		.await
		.expect("Store v2 should succeed");

	// Assert - Retrieved token should have v2 data
	let retrieved = storage
		.get("update-token")
		.await
		.expect("Get should succeed");

	assert_eq!(
		retrieved.metadata().get("version"),
		Some(&"2".to_string()),
		"Token should have version 2"
	);
}

#[rstest]
#[tokio::test]
async fn test_token_lifecycle_create_use_delete(storage: InMemoryTokenStorage) {
	// Phase 1: Create
	let token = create_test_token("lifecycle-token", 42, Some(3600));
	storage.store(token).await.expect("Store should succeed");

	// Phase 2: Use (multiple reads)
	for _ in 0..5 {
		let result = storage.get("lifecycle-token").await;
		assert!(result.is_ok(), "Token should be retrievable");
	}

	// Phase 3: Delete
	storage
		.delete("lifecycle-token")
		.await
		.expect("Delete should succeed");

	// Phase 4: Verify gone
	let result = storage.get("lifecycle-token").await;
	assert!(result.is_err(), "Token should not exist after deletion");
}

#[rstest]
#[tokio::test]
async fn test_token_expiration_state_change(storage: InMemoryTokenStorage) {
	// Arrange - Token that expires in 1 second
	let current_time = chrono::Utc::now().timestamp();
	let token = StoredToken::new("expiring-token", 42).with_expiration(Some(current_time + 1));

	storage.store(token).await.expect("Store should succeed");

	// Initially valid
	let retrieved = storage
		.get("expiring-token")
		.await
		.expect("Get should succeed");
	assert!(
		!retrieved.is_expired(current_time),
		"Token should not be expired initially"
	);

	// After expiration time
	assert!(
		retrieved.is_expired(current_time + 2),
		"Token should be expired after expiration time"
	);
}

// =============================================================================
// Edge Cases Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_token_with_empty_string(storage: InMemoryTokenStorage) {
	// Arrange
	let token = StoredToken::new("", 42);

	// Act
	storage
		.store(token)
		.await
		.expect("Store empty token should succeed");
	let retrieved = storage
		.get("")
		.await
		.expect("Get empty token should succeed");

	// Assert
	assert_eq!(retrieved.token(), "");
	assert_eq!(retrieved.user_id(), 42);
}

#[rstest]
#[tokio::test]
async fn test_token_with_special_characters(storage: InMemoryTokenStorage) {
	// Arrange
	let special_token = "token/with\\special:chars?query=1&other=2#fragment";
	let token = create_test_token(special_token, 42, Some(3600));

	// Act
	storage.store(token).await.expect("Store should succeed");
	let retrieved = storage
		.get(special_token)
		.await
		.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.token(), special_token);
}

#[rstest]
#[tokio::test]
async fn test_token_with_unicode(storage: InMemoryTokenStorage) {
	// Arrange
	let unicode_token = "トークン_токен_🔑";
	let token = create_test_token(unicode_token, 42, Some(3600));

	// Act
	storage.store(token).await.expect("Store should succeed");
	let retrieved = storage
		.get(unicode_token)
		.await
		.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.token(), unicode_token);
}

#[rstest]
#[tokio::test]
async fn test_token_with_very_long_value(storage: InMemoryTokenStorage) {
	// Arrange
	let long_token: String = (0..10000).map(|_| 'a').collect();
	let token = create_test_token(&long_token, 42, Some(3600));

	// Act
	storage.store(token).await.expect("Store should succeed");
	let retrieved = storage.get(&long_token).await.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.token().len(), 10000);
}

#[rstest]
#[tokio::test]
async fn test_token_with_zero_user_id(storage: InMemoryTokenStorage) {
	// Arrange
	let token = create_test_token("zero-user-token", 0, Some(3600));

	// Act
	storage.store(token).await.expect("Store should succeed");
	let retrieved = storage
		.get("zero-user-token")
		.await
		.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.user_id(), 0);
}

#[rstest]
#[tokio::test]
async fn test_token_with_negative_user_id(storage: InMemoryTokenStorage) {
	// Arrange
	let token = create_test_token("negative-user-token", -1, Some(3600));

	// Act
	storage.store(token).await.expect("Store should succeed");
	let retrieved = storage
		.get("negative-user-token")
		.await
		.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.user_id(), -1);
}

#[rstest]
#[tokio::test]
async fn test_token_with_max_user_id(storage: InMemoryTokenStorage) {
	// Arrange
	let token = create_test_token("max-user-token", i64::MAX, Some(3600));

	// Act
	storage.store(token).await.expect("Store should succeed");
	let retrieved = storage
		.get("max-user-token")
		.await
		.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.user_id(), i64::MAX);
}

#[rstest]
#[tokio::test]
async fn test_token_with_large_metadata(storage: InMemoryTokenStorage) {
	// Arrange
	let mut metadata = HashMap::new();
	for i in 0..100 {
		metadata.insert(format!("key_{}", i), format!("value_{}", i));
	}

	let token = StoredToken::new("large-metadata-token", 42)
		.with_expiration(Some(chrono::Utc::now().timestamp() + 3600))
		.with_metadata(metadata.clone());

	// Act
	storage.store(token).await.expect("Store should succeed");
	let retrieved = storage
		.get("large-metadata-token")
		.await
		.expect("Get should succeed");

	// Assert
	assert_eq!(retrieved.metadata().len(), 100);
	assert_eq!(
		retrieved.metadata().get("key_50"),
		Some(&"value_50".to_string())
	);
}

// =============================================================================
// Concurrent Operations Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_concurrent_store_operations() {
	// Arrange
	let storage = Arc::new(InMemoryTokenStorage::new());
	let barrier = Arc::new(Barrier::new(10));

	// Act - Store 10 tokens concurrently
	let handles: Vec<_> = (0..10)
		.map(|i| {
			let storage = Arc::clone(&storage);
			let barrier = Arc::clone(&barrier);

			tokio::spawn(async move {
				barrier.wait().await;
				let token = create_test_token(&format!("concurrent-{}", i), i, Some(3600));
				storage.store(token).await
			})
		})
		.collect();

	// Wait for all to complete
	for handle in handles {
		handle
			.await
			.expect("Task should complete")
			.expect("Store should succeed");
	}

	// Assert - All tokens should exist
	for i in 0..10 {
		let result = storage.get(&format!("concurrent-{}", i)).await;
		assert!(result.is_ok(), "Token {} should exist", i);
	}
}

#[rstest]
#[tokio::test]
async fn test_concurrent_read_operations() {
	// Arrange
	let storage = Arc::new(InMemoryTokenStorage::new());
	let token = create_test_token("read-token", 42, Some(3600));
	storage.store(token).await.expect("Store should succeed");

	let barrier = Arc::new(Barrier::new(50));

	// Act - Read the same token 50 times concurrently
	let handles: Vec<_> = (0..50)
		.map(|_| {
			let storage = Arc::clone(&storage);
			let barrier = Arc::clone(&barrier);

			tokio::spawn(async move {
				barrier.wait().await;
				storage.get("read-token").await
			})
		})
		.collect();

	// Assert - All reads should succeed
	for handle in handles {
		let result = handle
			.await
			.expect("Task should complete")
			.expect("Get should succeed");
		assert_eq!(result.token(), "read-token");
		assert_eq!(result.user_id(), 42);
	}
}

#[rstest]
#[tokio::test]
async fn test_concurrent_read_write_operations() {
	// Arrange
	let storage = Arc::new(InMemoryTokenStorage::new());
	let barrier = Arc::new(Barrier::new(20));

	// Act - Mix of reads and writes
	let handles: Vec<_> = (0..20)
		.map(|i| {
			let storage = Arc::clone(&storage);
			let barrier = Arc::clone(&barrier);

			tokio::spawn(async move {
				barrier.wait().await;

				if i % 2 == 0 {
					// Write
					let token = create_test_token(&format!("rw-token-{}", i), i, Some(3600));
					storage.store(token).await.map(|_| None)
				} else {
					// Read (may or may not exist)
					storage
						.get(&format!("rw-token-{}", i - 1))
						.await
						.ok()
						.map(Some)
						.ok_or_else(|| reinhardt_auth::TokenStorageError::NotFound)
				}
			})
		})
		.collect();

	// Wait for all - some reads may fail if write hasn't happened yet
	for handle in handles {
		let _ = handle.await;
	}

	// Assert - All write operations should have succeeded
	for i in (0..20).step_by(2) {
		let result = storage.get(&format!("rw-token-{}", i)).await;
		assert!(
			result.is_ok(),
			"Token {} should exist after writes complete",
			i
		);
	}
}

// =============================================================================
// Use Case Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_user_session_management_use_case(storage: InMemoryTokenStorage) {
	// Scenario: User logs in from multiple devices
	let user_id = 42;

	// Login from device 1
	let device1_token = StoredToken::new("session-device1", user_id)
		.with_expiration(Some(chrono::Utc::now().timestamp() + 86400))
		.with_metadata(HashMap::from([(
			"device".to_string(),
			"iPhone".to_string(),
		)]));

	storage
		.store(device1_token)
		.await
		.expect("Store device1 session");

	// Login from device 2
	let device2_token = StoredToken::new("session-device2", user_id)
		.with_expiration(Some(chrono::Utc::now().timestamp() + 86400))
		.with_metadata(HashMap::from([(
			"device".to_string(),
			"MacBook".to_string(),
		)]));

	storage
		.store(device2_token)
		.await
		.expect("Store device2 session");

	// Verify both sessions active
	let sessions = storage
		.get_user_tokens(user_id)
		.await
		.expect("Get sessions");
	assert_eq!(sessions.len(), 2, "User should have 2 active sessions");

	// User logs out from all devices
	storage
		.delete_user_tokens(user_id)
		.await
		.expect("Logout all");

	// Verify no sessions remain
	let remaining = storage
		.get_user_tokens(user_id)
		.await
		.expect("Check sessions");
	assert!(
		remaining.is_empty(),
		"No sessions should remain after logout all"
	);
}

#[rstest]
#[tokio::test]
async fn test_token_refresh_use_case(storage: InMemoryTokenStorage) {
	// Scenario: Token refresh flow
	let user_id = 42;

	// Initial token
	let old_token = StoredToken::new("old-token", user_id)
		.with_expiration(Some(chrono::Utc::now().timestamp() + 300)); // 5 minutes left

	storage.store(old_token).await.expect("Store old token");

	// Generate new token (simulating refresh)
	let new_token = StoredToken::new("new-token", user_id)
		.with_expiration(Some(chrono::Utc::now().timestamp() + 3600)); // 1 hour

	storage.store(new_token).await.expect("Store new token");

	// Revoke old token
	storage.delete("old-token").await.expect("Delete old token");

	// Verify state
	assert!(
		storage.get("old-token").await.is_err(),
		"Old token should be revoked"
	);
	assert!(
		storage.get("new-token").await.is_ok(),
		"New token should be valid"
	);
}

// =============================================================================
// Decision Table Tests
// =============================================================================

#[rstest]
#[case(true, true, true)] // Exists + Not expired = Can access
#[case(true, false, false)] // Exists + Expired = Cannot access
#[case(false, true, false)] // Not exists + Any = Cannot access
#[case(false, false, false)] // Not exists + Any = Cannot access
#[tokio::test]
async fn test_token_access_decision_table(
	#[case] token_exists: bool,
	#[case] token_valid: bool,
	#[case] expected_access: bool,
) {
	// Arrange
	let storage = InMemoryTokenStorage::new();
	let current_time = chrono::Utc::now().timestamp();

	if token_exists {
		let expires_at = if token_valid {
			Some(current_time + 3600) // Valid for 1 hour
		} else {
			Some(current_time - 3600) // Expired 1 hour ago
		};

		let token = StoredToken::new("access-token", 42).with_expiration(expires_at);
		storage.store(token).await.expect("Store should succeed");
	}

	// Act
	let can_access = storage
		.get("access-token")
		.await
		.ok()
		.map(|t| !t.is_expired(current_time))
		.unwrap_or(false);

	// Assert
	assert_eq!(
		can_access, expected_access,
		"Access for (exists={}, valid={}) should be {}",
		token_exists, token_valid, expected_access
	);
}
