//! Integration tests for secrets providers
//!
//! This test module validates the integration of secrets providers (Memory, Env)
//! with secret management functionality, including retrieval, rotation, and audit logging.

use reinhardt_conf::settings::secrets::audit::backends::MemorySecretAuditBackend;
use reinhardt_conf::settings::secrets::audit::{
	SecretAccessEvent, SecretAuditBackend, SecretAuditLogger,
};
use reinhardt_conf::settings::secrets::providers::env::EnvSecretProvider;
use reinhardt_conf::settings::secrets::providers::memory::MemorySecretProvider;
use reinhardt_conf::settings::secrets::rotation::{RotationPolicy, SecretRotation};
use reinhardt_conf::settings::secrets::{SecretProvider, SecretString};
use rstest::*;
use serial_test::serial;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Async fixture for MemorySecretProvider
#[fixture]
async fn memory_secrets_provider() -> MemorySecretProvider {
	let provider = MemorySecretProvider::new();

	// Pre-populate with secrets
	provider
		.set_secret(
			"database/password",
			SecretString::new("super_secret_password"),
		)
		.await
		.unwrap();
	provider
		.set_secret("api/key", SecretString::new("api_key_12345"))
		.await
		.unwrap();
	provider
		.set_secret("jwt/secret", SecretString::new("jwt_secret_token"))
		.await
		.unwrap();
	provider
		.set_secret(
			"oauth/client_secret",
			SecretString::new("oauth_client_secret_abc"),
		)
		.await
		.unwrap();

	provider
}

/// Test: Memory provider basic secret retrieval
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_memory_provider_basic_retrieval(
	#[future] memory_secrets_provider: MemorySecretProvider,
) {
	let provider = Arc::new(memory_secrets_provider.await);

	// Retrieve secrets
	let db_password = provider
		.get_secret("database/password")
		.await
		.expect("Failed to get database password");
	assert_eq!(db_password.expose_secret(), "super_secret_password");

	let api_key = provider
		.get_secret("api/key")
		.await
		.expect("Failed to get API key");
	assert_eq!(api_key.expose_secret(), "api_key_12345");

	let jwt_secret = provider
		.get_secret("jwt/secret")
		.await
		.expect("Failed to get JWT secret");
	assert_eq!(jwt_secret.expose_secret(), "jwt_secret_token");
}

/// Test: Memory provider secret update
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_memory_provider_secret_update() {
	let provider = MemorySecretProvider::new();
	provider
		.set_secret("test/secret", SecretString::new("initial_value"))
		.await
		.unwrap();

	// Get initial value
	let initial = provider
		.get_secret("test/secret")
		.await
		.expect("Failed to get initial secret");
	assert_eq!(initial.expose_secret(), "initial_value");

	// Update secret
	provider
		.set_secret("test/secret", SecretString::new("updated_value"))
		.await
		.unwrap();

	// Get updated value
	let updated = provider
		.get_secret("test/secret")
		.await
		.expect("Failed to get updated secret");
	assert_eq!(updated.expose_secret(), "updated_value");
}

/// Test: Memory provider non-existent secret
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_memory_provider_nonexistent_secret(
	#[future] memory_secrets_provider: MemorySecretProvider,
) {
	let provider = Arc::new(memory_secrets_provider.await);

	// Try to get non-existent secret
	let result = provider.get_secret("nonexistent/secret").await;

	// Should return error
	assert!(result.is_err());
}

/// Test: Memory provider list and exists
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_memory_provider_list_and_exists() {
	let provider = MemorySecretProvider::new();

	// Add some secrets
	provider
		.set_secret("key1", SecretString::new("value1"))
		.await
		.unwrap();
	provider
		.set_secret("key2", SecretString::new("value2"))
		.await
		.unwrap();

	// Check exists
	assert!(provider.exists("key1"));
	assert!(provider.exists("key2"));
	assert!(!provider.exists("key3"));

	// List secrets
	let keys = provider.list_secrets().await.unwrap();
	assert!(keys.contains(&"key1".to_string()));
	assert!(keys.contains(&"key2".to_string()));
	assert_eq!(keys.len(), 2);
}

/// Test: Memory provider delete operation
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_memory_provider_delete() {
	let provider = MemorySecretProvider::new();

	// Set value
	provider
		.set_secret("temp/value", SecretString::new("temporary"))
		.await
		.unwrap();

	// Verify it exists
	assert!(provider.exists("temp/value"));

	// Delete value
	provider.delete_secret("temp/value").await.unwrap();

	// Verify it's gone
	assert!(!provider.exists("temp/value"));
	assert!(provider.get_secret("temp/value").await.is_err());
}

/// Test: Environment variables secrets provider
#[rstest]
#[serial(secrets_env)]
#[tokio::test]
async fn test_env_secrets_provider() {
	// Set environment variables
	unsafe {
		env::set_var("SECRET_DATABASE_PASSWORD", "env_db_password");
		env::set_var("SECRET_API_KEY", "env_api_key");
		env::set_var("SECRET_JWT_TOKEN", "env_jwt_token");
	}

	let provider = Arc::new(EnvSecretProvider::new("SECRET_"));

	// Retrieve secrets from environment
	let db_password = provider
		.get_secret("DATABASE_PASSWORD")
		.await
		.expect("Failed to get database password from env");
	assert_eq!(db_password.expose_secret(), "env_db_password");

	let api_key = provider
		.get_secret("API_KEY")
		.await
		.expect("Failed to get API key from env");
	assert_eq!(api_key.expose_secret(), "env_api_key");

	let jwt_token = provider
		.get_secret("JWT_TOKEN")
		.await
		.expect("Failed to get JWT token from env");
	assert_eq!(jwt_token.expose_secret(), "env_jwt_token");

	// Cleanup
	unsafe {
		env::remove_var("SECRET_DATABASE_PASSWORD");
		env::remove_var("SECRET_API_KEY");
		env::remove_var("SECRET_JWT_TOKEN");
	}
}

/// Test: Environment provider with custom prefix
#[rstest]
#[serial(secrets_env)]
#[tokio::test]
async fn test_env_provider_custom_prefix() {
	unsafe {
		env::set_var("MYAPP_DB_PASS", "custom_password");
		env::set_var("MYAPP_API_SECRET", "custom_api_secret");
	}

	let provider = Arc::new(EnvSecretProvider::new("MYAPP_"));

	let db_pass = provider
		.get_secret("DB_PASS")
		.await
		.expect("Failed to get DB password");
	assert_eq!(db_pass.expose_secret(), "custom_password");

	let api_secret = provider
		.get_secret("API_SECRET")
		.await
		.expect("Failed to get API secret");
	assert_eq!(api_secret.expose_secret(), "custom_api_secret");

	// Cleanup
	unsafe {
		env::remove_var("MYAPP_DB_PASS");
		env::remove_var("MYAPP_API_SECRET");
	}
}

/// Test: Secret rotation basic functionality
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_secret_rotation_basic() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(1),
		max_age: None,
	};
	let rotation = SecretRotation::new(policy);

	// First rotation should be allowed (no history)
	assert!(rotation.should_rotate("test_key").await.unwrap());

	// Perform rotation
	rotation.rotate("test_key").await.unwrap();

	// Immediate rotation should fail
	assert!(!rotation.should_rotate("test_key").await.unwrap());
	assert!(rotation.rotate("test_key").await.is_err());

	// After interval, rotation should be allowed
	sleep(Duration::from_millis(1100)).await;
	assert!(rotation.should_rotate("test_key").await.unwrap());
}

/// Test: Secret rotation with max age
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_secret_rotation_with_max_age() {
	let policy = RotationPolicy {
		interval: Duration::from_secs(3600),   // 1 hour
		max_age: Some(Duration::from_secs(1)), // 1 second max age
	};
	let rotation = SecretRotation::new(policy);

	// Perform initial rotation
	rotation.rotate("max_age_key").await.unwrap();

	// Wait for max age to expire (with buffer for timing variance)
	sleep(Duration::from_millis(1100)).await;

	// Should require rotation due to max age
	// Note: max_age takes precedence over interval
	assert!(rotation.should_rotate("max_age_key").await.unwrap());
}

/// Test: Secret audit logging
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_secret_audit_logging() {
	let backend = Arc::new(MemorySecretAuditBackend::new());
	let audit_logger = SecretAuditLogger::new(backend.clone());

	// Log access
	let event = SecretAccessEvent::new(
		"audited/secret".to_string(),
		"test_user".to_string(),
		true,
		Some("Test access".to_string()),
	);

	audit_logger.log_access(event).await.unwrap();

	// Verify audit log
	let logs = backend.get_accesses(None).await.unwrap();
	assert!(!logs.is_empty());
	assert_eq!(logs[0].secret_name, "audited/secret");
	assert_eq!(logs[0].accessor, "test_user");
	assert!(logs[0].success);
	assert_eq!(logs[0].context, Some("Test access".to_string()));
}

/// Test: Concurrent secret access
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_concurrent_secret_access() {
	let provider = Arc::new(MemorySecretProvider::new());

	// Pre-populate secrets
	for i in 0..10 {
		provider
			.set_secret(
				&format!("concurrent/secret{}", i),
				SecretString::new(format!("value_{}", i)),
			)
			.await
			.unwrap();
	}

	// Spawn concurrent tasks
	let mut handles = vec![];
	for i in 0..10 {
		let provider_clone = provider.clone();
		let handle = tokio::spawn(async move {
			let key = format!("concurrent/secret{}", i);
			let value = provider_clone
				.get_secret(&key)
				.await
				.expect("Failed to get secret");
			assert_eq!(value.expose_secret(), format!("value_{}", i));
		});
		handles.push(handle);
	}

	// Wait for all tasks
	for handle in handles {
		handle.await.expect("Task panicked");
	}
}

/// Test: Secret provider with nested keys
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_nested_secret_keys() {
	let provider = MemorySecretProvider::new();

	provider
		.set_secret(
			"app/production/database/password",
			SecretString::new("prod_db_pass"),
		)
		.await
		.unwrap();
	provider
		.set_secret("app/production/api/key", SecretString::new("prod_api_key"))
		.await
		.unwrap();
	provider
		.set_secret(
			"app/staging/database/password",
			SecretString::new("staging_db_pass"),
		)
		.await
		.unwrap();
	provider
		.set_secret("app/staging/api/key", SecretString::new("staging_api_key"))
		.await
		.unwrap();

	// Retrieve nested secrets
	assert_eq!(
		provider
			.get_secret("app/production/database/password")
			.await
			.expect("Failed to get prod db password")
			.expose_secret(),
		"prod_db_pass"
	);
	assert_eq!(
		provider
			.get_secret("app/production/api/key")
			.await
			.expect("Failed to get prod api key")
			.expose_secret(),
		"prod_api_key"
	);
	assert_eq!(
		provider
			.get_secret("app/staging/database/password")
			.await
			.expect("Failed to get staging db password")
			.expose_secret(),
		"staging_db_pass"
	);
	assert_eq!(
		provider
			.get_secret("app/staging/api/key")
			.await
			.expect("Failed to get staging api key")
			.expose_secret(),
		"staging_api_key"
	);
}

/// Test: Get secret with metadata
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_get_secret_with_metadata() {
	let provider = MemorySecretProvider::new();

	provider
		.set_secret("metadata_key", SecretString::new("test_value"))
		.await
		.unwrap();

	let (secret, metadata) = provider
		.get_secret_with_metadata("metadata_key")
		.await
		.unwrap();

	assert_eq!(secret.expose_secret(), "test_value");
	assert!(metadata.created_at.is_some());
	assert!(metadata.updated_at.is_some());
}

/// Test: Multiple audit events
#[rstest]
#[serial(secrets)]
#[tokio::test]
async fn test_multiple_audit_events() {
	let backend = Arc::new(MemorySecretAuditBackend::new());
	let audit_logger = SecretAuditLogger::new(backend.clone());

	// Log multiple events
	for i in 0..5 {
		let event = SecretAccessEvent::new(format!("secret{}", i), "app".to_string(), true, None);
		audit_logger.log_access(event).await.unwrap();
	}

	// Verify all events logged
	let logs = backend.get_accesses(None).await.unwrap();
	assert_eq!(logs.len(), 5);
}
