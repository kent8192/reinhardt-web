//! Integration tests for Encrypted Secrets in Production Use Case.
//!
//! This test module validates that production applications can load encrypted
//! configuration files with secrets, decrypt them securely, and ensure secrets
//! are automatically redacted in logs.
//!
//! NOTE: These tests are feature-gated with "encryption" feature.

#![cfg(feature = "encryption")]

use reinhardt_settings::encryption::{ConfigEncryptor, EncryptedConfig};
use reinhardt_settings::secrets::types::SecretString;
use rstest::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Production configuration with sensitive data
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ProductionConfig {
	database_password: String,
	api_key: String,
	secret_key: String,
	app_name: String,
}

/// Test: Complete encrypted production workflow
///
/// Why: Validates that production applications can encrypt sensitive configuration,
/// save it to disk, load it securely, and decrypt it correctly.
#[rstest]
#[test]
fn test_encrypted_production_workflow() {
	let temp_dir = TempDir::new().unwrap();

	// Step 1: Create production config with secrets
	let production_config = ProductionConfig {
		database_password: "super_secret_db_password_1234".to_string(),
		api_key: "sk_live_api_key_abcdef1234567890".to_string(),
		secret_key: "production-secret-key-very-secure-random-value-here-32bytes".to_string(),
		app_name: "my_production_app".to_string(),
	};

	// Serialize config to JSON
	let config_json = serde_json::to_string(&production_config).unwrap();
	let config_bytes = config_json.as_bytes();

	// Step 2: Encrypt config with AES-256-GCM
	let encryption_key = vec![0x42; 32]; // 32-byte key (in production: load from secure env)
	let encryptor = ConfigEncryptor::new(encryption_key.clone())
		.expect("ConfigEncryptor should initialize with valid 32-byte key");

	let encrypted_config = encryptor
		.encrypt(config_bytes)
		.expect("Encryption should succeed");

	// Step 3: Save encrypted config to file
	let encrypted_file_path = temp_dir.path().join("production.encrypted.json");
	let encrypted_json = serde_json::to_string(&encrypted_config).unwrap();
	let mut file = fs::File::create(&encrypted_file_path).unwrap();
	file.write_all(encrypted_json.as_bytes()).unwrap();

	// Step 4: Application starts - load encrypted config from disk
	let loaded_json = fs::read_to_string(&encrypted_file_path).unwrap();
	let loaded_encrypted_config: EncryptedConfig = serde_json::from_str(&loaded_json).unwrap();

	// Step 5: Decrypt with key from environment
	let decrypted_bytes = encryptor
		.decrypt(&loaded_encrypted_config)
		.expect("Decryption should succeed with correct key");

	let decrypted_json = String::from_utf8(decrypted_bytes).unwrap();
	let decrypted_config: ProductionConfig = serde_json::from_str(&decrypted_json).unwrap();

	// Verify all secrets are correctly decrypted
	assert_eq!(
		decrypted_config.database_password, "super_secret_db_password_1234",
		"Database password should be correctly decrypted"
	);
	assert_eq!(
		decrypted_config.api_key, "sk_live_api_key_abcdef1234567890",
		"API key should be correctly decrypted"
	);
	assert_eq!(
		decrypted_config.secret_key, "production-secret-key-very-secure-random-value-here-32bytes",
		"Secret key should be correctly decrypted"
	);
	assert_eq!(
		decrypted_config.app_name, "my_production_app",
		"App name should be correctly decrypted"
	);
}

/// Test: Encrypted config roundtrip (encrypt → save → load → decrypt)
///
/// Why: Validates the complete lifecycle of encrypted configuration persistence
/// including serialization and deserialization.
#[rstest]
#[test]
fn test_encrypted_config_roundtrip() {
	let temp_dir = TempDir::new().unwrap();

	let original_data = b"This is sensitive production data with secrets!";
	let encryption_key = vec![0x55; 32]; // 32-byte key

	let encryptor = ConfigEncryptor::new(encryption_key.clone()).unwrap();

	// Encrypt
	let encrypted = encryptor.encrypt(original_data).unwrap();

	// Save to file as JSON
	let file_path = temp_dir.path().join("encrypted.json");
	let encrypted_json = serde_json::to_string(&encrypted).unwrap();
	fs::write(&file_path, encrypted_json).unwrap();

	// Load from file
	let loaded_json = fs::read_to_string(&file_path).unwrap();
	let loaded_encrypted: EncryptedConfig = serde_json::from_str(&loaded_json).unwrap();

	// Decrypt
	let decrypted_data = encryptor.decrypt(&loaded_encrypted).unwrap();

	assert_eq!(
		decrypted_data, original_data,
		"Decrypted data should match original after full roundtrip"
	);
}

/// Test: Wrong decryption key fails
///
/// Why: Validates that encrypted config cannot be decrypted with incorrect key,
/// ensuring security against unauthorized access.
#[rstest]
#[test]
fn test_encrypted_config_wrong_key_fails() {
	let original_data = b"Secret production data";

	let encryption_key = vec![0x11; 32];
	let wrong_key = vec![0x22; 32];

	let encryptor = ConfigEncryptor::new(encryption_key).unwrap();
	let wrong_encryptor = ConfigEncryptor::new(wrong_key).unwrap();

	// Encrypt with correct key
	let encrypted = encryptor.encrypt(original_data).unwrap();

	// Attempt to decrypt with wrong key
	let result = wrong_encryptor.decrypt(&encrypted);

	assert!(result.is_err(), "Decryption with wrong key should fail");

	let error_message = result.unwrap_err();
	assert!(
		error_message.contains("Decryption failed"),
		"Error message should indicate decryption failure"
	);
}

/// Test: Tampered encrypted data fails decryption
///
/// Why: Validates that AES-256-GCM authentication detects data tampering
/// and prevents decryption of corrupted data.
#[rstest]
#[test]
fn test_encrypted_config_tampered_data_fails() {
	let original_data = b"Production secrets";
	let encryption_key = vec![0x33; 32];

	let encryptor = ConfigEncryptor::new(encryption_key).unwrap();
	let mut encrypted = encryptor.encrypt(original_data).unwrap();

	// Tamper with encrypted data (flip a bit)
	if !encrypted.data.is_empty() {
		encrypted.data[0] ^= 0xFF;
	}

	// Attempt to decrypt tampered data
	let result = encryptor.decrypt(&encrypted);

	assert!(result.is_err(), "Decryption of tampered data should fail");

	let error_message = result.unwrap_err();
	assert!(
		error_message.contains("Decryption failed"),
		"Error message should indicate decryption failure"
	);
}

/// Test: SecretString redaction in debug output
///
/// Why: Validates that SecretString automatically redacts sensitive values
/// in debug and display output, preventing accidental exposure in logs.
#[rstest]
#[test]
fn test_secret_string_redaction() {
	let sensitive_value = "super_secret_password_1234";
	let secret = SecretString::new(sensitive_value);

	// Test Debug output
	let debug_output = format!("{:?}", secret);
	assert!(
		!debug_output.contains(sensitive_value),
		"Debug output should not contain the actual secret"
	);
	assert!(
		debug_output.contains("[REDACTED]") || debug_output.contains("SecretString"),
		"Debug output should indicate redaction"
	);

	// Test Display output
	let display_output = format!("{}", secret);
	assert!(
		!display_output.contains(sensitive_value),
		"Display output should not contain the actual secret"
	);
	assert!(
		display_output.contains("[REDACTED]"),
		"Display output should show [REDACTED]"
	);

	// Test that expose_secret() correctly exposes the value
	assert_eq!(
		secret.expose_secret(),
		sensitive_value,
		"expose_secret() should provide access to the actual value"
	);
}

/// Test: Large encrypted configuration
///
/// Why: Validates that encryption handles large production configurations
/// with multiple secrets and complex nested structures.
#[rstest]
#[test]
fn test_large_encrypted_configuration() {
	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct LargeConfig {
		database_configs: Vec<DatabaseConfig>,
		api_keys: Vec<String>,
		secrets: Vec<String>,
		metadata: String,
	}

	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct DatabaseConfig {
		name: String,
		host: String,
		password: String,
	}

	let large_config = LargeConfig {
		database_configs: (0..10)
			.map(|i| DatabaseConfig {
				name: format!("db_{}", i),
				host: format!("db{}.internal.example.com", i),
				password: format!("db_password_{}_very_long_secret", i),
			})
			.collect(),
		api_keys: (0..20)
			.map(|i| format!("sk_live_api_key_{}_abcdef1234567890", i))
			.collect(),
		secrets: (0..50)
			.map(|i| format!("secret_value_{}_extremely_sensitive_data", i))
			.collect(),
		metadata: "Production environment configuration".to_string(),
	};

	let config_json = serde_json::to_string(&large_config).unwrap();
	let encryption_key = vec![0x77; 32];

	let encryptor = ConfigEncryptor::new(encryption_key).unwrap();

	// Encrypt large config
	let encrypted = encryptor.encrypt(config_json.as_bytes()).unwrap();

	// Decrypt
	let decrypted_bytes = encryptor.decrypt(&encrypted).unwrap();
	let decrypted_json = String::from_utf8(decrypted_bytes).unwrap();
	let decrypted_config: LargeConfig = serde_json::from_str(&decrypted_json).unwrap();

	// Verify all data is correctly preserved
	assert_eq!(
		decrypted_config, large_config,
		"Large config should be correctly encrypted and decrypted"
	);
	assert_eq!(
		decrypted_config.database_configs.len(),
		10,
		"All database configs should be preserved"
	);
	assert_eq!(
		decrypted_config.api_keys.len(),
		20,
		"All API keys should be preserved"
	);
	assert_eq!(
		decrypted_config.secrets.len(),
		50,
		"All secrets should be preserved"
	);
}

/// Test: Invalid encryption key length
///
/// Why: Validates that ConfigEncryptor correctly rejects keys that are not
/// exactly 32 bytes (256 bits) for AES-256-GCM.
#[rstest]
#[case(16, "16-byte key (too short)")]
#[case(24, "24-byte key (too short)")]
#[case(48, "48-byte key (too long)")]
#[case(0, "empty key")]
fn test_invalid_encryption_key_length(#[case] key_len: usize, #[case] description: &str) {
	let invalid_key = vec![0xFF; key_len];

	let result = ConfigEncryptor::new(invalid_key);

	assert!(
		result.is_err(),
		"ConfigEncryptor should reject {}",
		description
	);

	let error_message = result.unwrap_err();
	assert!(
		error_message.contains("32 bytes"),
		"Error message should mention required key size"
	);
}

/// Test: Production config with env var loading simulation
///
/// Why: Validates the pattern where encryption key is loaded from environment
/// variable in production (simulated by passing key as parameter).
#[rstest]
#[test]
fn test_production_env_key_loading_pattern() {
	// Simulate loading encryption key from environment
	// In production: let encryption_key = env::var("CONFIG_ENCRYPTION_KEY").unwrap();
	let encryption_key_hex = "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60";
	let encryption_key = hex::decode(encryption_key_hex).unwrap();

	assert_eq!(
		encryption_key.len(),
		32,
		"Hex-decoded key should be 32 bytes"
	);

	let encryptor = ConfigEncryptor::new(encryption_key).unwrap();

	let production_secrets = serde_json::json!({
		"database_url": "postgresql://user:pass@prod-db.internal:5432/maindb",
		"redis_url": "redis://:password@prod-redis.internal:6379/0",
		"jwt_secret": "production_jwt_secret_very_secure_random_string",
		"stripe_api_key": "sk_live_stripe_api_key_1234567890abcdef",
	});

	let secrets_json = serde_json::to_string(&production_secrets).unwrap();

	// Encrypt
	let encrypted = encryptor.encrypt(secrets_json.as_bytes()).unwrap();

	// Decrypt (application startup)
	let decrypted_bytes = encryptor.decrypt(&encrypted).unwrap();
	let decrypted_json = String::from_utf8(decrypted_bytes).unwrap();
	let decrypted_secrets: serde_json::Value = serde_json::from_str(&decrypted_json).unwrap();

	// Verify secrets are correctly loaded
	assert_eq!(
		decrypted_secrets["database_url"].as_str().unwrap(),
		"postgresql://user:pass@prod-db.internal:5432/maindb"
	);
	assert_eq!(
		decrypted_secrets["jwt_secret"].as_str().unwrap(),
		"production_jwt_secret_very_secure_random_string"
	);
}
