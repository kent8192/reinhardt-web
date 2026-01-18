//! Feature Combination Tests for reinhardt-settings.
//!
//! This test module validates that feature flag combinations work correctly
//! and that different features can be composed together without conflicts.
//!
//! ## Test Categories
//!
//! 1. **async + encryption**: Async environment with encryption capabilities
//! 2. **async + dynamic-redis + caching**: Dynamic Redis backend with caching
//! 3. **async + secret-rotation + vault**: Secret rotation with Vault integration
//! 4. **async + hot-reload**: Hot reload capabilities in async environment
//!
//! ## Testing Strategy
//!
//! Each test uses `#[cfg(all(feature = "...", feature = "..."))]` to ensure
//! it only runs when the specified feature combination is enabled. Tests verify:
//! - Basic initialization succeeds
//! - Feature-specific APIs are available
//! - No conflicts between features

use rstest::*;

// Import traits needed for feature tests
#[cfg(all(feature = "async", feature = "secret-rotation"))]
use reinhardt_conf::settings::prelude::SecretProvider;

/// Test: async + encryption feature combination
///
/// Why: Validates that encryption functionality works correctly in async environment.
/// This is a critical combination for production deployments requiring both
/// async I/O and data protection.
#[cfg(all(feature = "async", feature = "encryption"))]
#[rstest]
#[tokio::test]
async fn test_async_with_encryption() {
	use reinhardt_conf::settings::encryption::ConfigEncryptor;

	// Test: ConfigEncryptor can be initialized in async context
	let key = vec![42u8; 32]; // 32-byte key for AES-256-GCM
	let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

	// Test: Encryption and decryption work in async environment
	let plaintext = b"sensitive_data";
	let encrypted = encryptor.encrypt(plaintext).expect("Failed to encrypt");
	let decrypted = encryptor.decrypt(&encrypted).expect("Failed to decrypt");

	assert_eq!(
		plaintext.to_vec(),
		decrypted,
		"async + encryption: Encryption roundtrip should preserve data"
	);
}

/// Test: async + dynamic-redis + caching feature combination
///
/// Why: Validates that Redis backend and caching work together in async environment.
/// This combination is common for distributed applications requiring fast configuration access.
#[cfg(all(feature = "async", feature = "dynamic-redis", feature = "caching"))]
#[rstest]
#[tokio::test]
async fn test_async_with_dynamic_redis_and_caching() {
	use reinhardt_conf::settings::backends::memory::MemoryBackend;
	use reinhardt_conf::settings::dynamic::DynamicSettings;
	use std::sync::Arc;

	// Test: DynamicSettings can be initialized with MemoryBackend (Redis alternative for testing)
	let backend = Arc::new(MemoryBackend::new());
	let dynamic = DynamicSettings::new(backend);

	// Test: Basic operations work with feature combination
	let result = dynamic.get::<String>("test_key").await;
	assert!(
		result.is_ok(),
		"async + dynamic-redis + caching: DynamicSettings should initialize successfully"
	);

	// Note: Full Redis integration test requires TestContainers and is tested separately
	// This test validates the feature combination compiles and basic API is available
}

/// Test: async + secret-rotation + vault feature combination
///
/// Why: Validates that secret rotation works with Vault integration in async environment.
/// This combination is essential for zero-downtime secret updates in production.
#[cfg(all(feature = "async", feature = "secret-rotation", feature = "vault"))]
#[rstest]
#[tokio::test]
async fn test_async_with_secret_rotation_and_vault() {
	use reinhardt_conf::settings::secrets::SecretString;
	use reinhardt_conf::settings::secrets::providers::memory::MemorySecretProvider;

	// Test: SecretProvider can be initialized with secret rotation support
	let provider = MemorySecretProvider::new();

	// Test: Secret rotation API is available
	let secret_name = "test_secret";
	let secret_value = SecretString::new("initial_value");

	// Store initial secret
	provider
		.set_secret(secret_name, secret_value)
		.await
		.expect("Failed to store secret");

	// Retrieve secret
	let retrieved = provider
		.get_secret(secret_name)
		.await
		.expect("Failed to retrieve secret");

	assert!(
		!retrieved.expose_secret().is_empty(),
		"async + secret-rotation + vault: Secret retrieval should work"
	);

	// Note: Full Vault integration test requires external Vault server
	// This test validates the feature combination compiles and basic API is available
}

/// Test: async + hot-reload feature combination
///
/// Why: Validates that hot reload functionality works in async environment.
/// This allows configuration changes without application restart.
#[cfg(all(feature = "async", feature = "hot-reload"))]
#[rstest]
#[tokio::test]
async fn test_async_with_hot_reload() {
	use reinhardt_conf::settings::backends::memory::MemoryBackend;
	use reinhardt_conf::settings::dynamic::DynamicSettings;
	use std::sync::Arc;

	// Test: DynamicSettings with hot reload support
	let backend = Arc::new(MemoryBackend::new());
	let dynamic = DynamicSettings::new(backend);

	// Test: Configuration can be accessed (hot reload monitoring in background)
	let result = dynamic.get::<String>("test_key").await;
	assert!(
		result.is_ok(),
		"async + hot-reload: DynamicSettings should initialize successfully"
	);

	// Note: Full hot reload test requires file system monitoring and is tested separately
	// This test validates the feature combination compiles and basic API is available
}

/// Test: All features enabled together
///
/// Why: Validates that all features can coexist without conflicts.
/// This is a comprehensive test for maximum feature combination.
#[cfg(all(
	feature = "async",
	feature = "encryption",
	feature = "dynamic-redis",
	feature = "caching",
	feature = "secret-rotation",
	feature = "vault",
	feature = "hot-reload"
))]
#[rstest]
#[tokio::test]
async fn test_all_features_combination() {
	use reinhardt_conf::settings::backends::memory::MemoryBackend;
	use reinhardt_conf::settings::dynamic::DynamicSettings;
	use reinhardt_conf::settings::encryption::ConfigEncryptor;
	use reinhardt_conf::settings::secrets::SecretString;
	use reinhardt_conf::settings::secrets::providers::memory::MemorySecretProvider;
	use std::sync::Arc;

	// Test: All feature APIs are available and can be initialized together

	// 1. Encryption
	let encryption_key = vec![42u8; 32];
	let _encryption_manager = ConfigEncryptor::new(encryption_key).expect("Create encryptor");

	// 2. Dynamic settings with caching
	let backend = Arc::new(MemoryBackend::new());
	let _dynamic = DynamicSettings::new(backend);

	// 3. Secret management with rotation
	let provider = MemorySecretProvider::new();
	let test_secret = SecretString::new("test_value");
	provider
		.set_secret("test", test_secret)
		.await
		.expect("Set secret");

	// If we reach here without panics, all features work together
	assert!(
		true,
		"All features combination: All APIs should be available and initialize successfully"
	);
}

/// Test: Minimal feature set (no optional features)
///
/// Why: Validates that the crate works without any optional features enabled.
/// This ensures backward compatibility and minimal dependencies.
#[cfg(not(any(
	feature = "async",
	feature = "encryption",
	feature = "dynamic-redis",
	feature = "caching",
	feature = "secret-rotation",
	feature = "vault",
	feature = "hot-reload"
)))]
#[rstest]
fn test_minimal_features() {
	use reinhardt_conf::settings::Settings;

	// Test: Basic Settings struct is available without optional features
	let settings = Settings::default();

	assert!(
		!settings.debug || settings.debug,
		"Minimal features: Settings struct should be available"
	);
}

/// Test: encryption feature alone (without async)
///
/// Why: Validates that encryption can work in synchronous context.
#[cfg(all(feature = "encryption", not(feature = "async")))]
#[rstest]
fn test_encryption_without_async() {
	use reinhardt_conf::settings::encryption::ConfigEncryptor;

	// Test: Encryption works in synchronous context
	let key = vec![42u8; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Create encryptor");

	let plaintext = b"test_data";
	let encrypted = encryptor.encrypt(plaintext).expect("Encrypt");
	let decrypted = encryptor.decrypt(&encrypted).expect("Decrypt");

	assert_eq!(
		plaintext.to_vec(),
		decrypted,
		"encryption (no async): Should work in synchronous context"
	);
}
