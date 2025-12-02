//! Integration tests for encrypted settings storage and management
//!
//! This test module validates the complete encryption workflow for configuration files,
//! including encryption key management, file-based encrypted storage, decryption on load,
//! re-encryption on save, and performance characteristics with large settings.

#![allow(clippy::field_reassign_with_default)]
use reinhardt_settings::encryption::{ConfigEncryptor, EncryptedConfig};
use rstest::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test configuration structure with various data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestConfig {
	database_url: String,
	api_key: String,
	max_connections: u32,
	timeout_seconds: u64,
	features: Vec<String>,
	metadata: std::collections::HashMap<String, String>,
}

impl Default for TestConfig {
	fn default() -> Self {
		let mut metadata = std::collections::HashMap::new();
		metadata.insert("env".to_string(), "production".to_string());
		metadata.insert("region".to_string(), "us-west-2".to_string());

		Self {
			database_url: "postgres://user:password@localhost:5432/db".to_string(),
			api_key: "super_secret_api_key_12345".to_string(),
			max_connections: 100,
			timeout_seconds: 30,
			features: vec![
				"auth".to_string(),
				"cache".to_string(),
				"logging".to_string(),
			],
			metadata,
		}
	}
}

/// Large configuration for performance testing
#[cfg(feature = "encryption")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct LargeConfig {
	items: Vec<ConfigItem>,
}

#[cfg(feature = "encryption")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ConfigItem {
	id: u64,
	name: String,
	description: String,
	values: Vec<f64>,
}

#[cfg(feature = "encryption")]
impl LargeConfig {
	fn generate(item_count: usize) -> Self {
		let items = (0..item_count)
			.map(|i| ConfigItem {
				id: i as u64,
				name: format!("item_{}", i),
				description: format!(
					"Description for item {} with some additional text to make it larger",
					i
				),
				values: vec![i as f64, (i * 2) as f64, (i * 3) as f64],
			})
			.collect();

		Self { items }
	}
}

/// Fixture providing a temporary directory for test files
#[fixture]
fn temp_dir() -> TempDir {
	TempDir::new().expect("Failed to create temporary directory")
}

/// Fixture providing a 32-byte encryption key
#[fixture]
fn encryption_key() -> Vec<u8> {
	vec![42u8; 32]
}

/// Fixture providing an alternative encryption key
#[fixture]
fn alternative_key() -> Vec<u8> {
	vec![99u8; 32]
}

/// Helper function to serialize and encrypt configuration
fn encrypt_config<T: Serialize>(
	config: &T,
	encryptor: &ConfigEncryptor,
) -> Result<EncryptedConfig, String> {
	let json_data =
		serde_json::to_vec(config).map_err(|e| format!("Serialization failed: {}", e))?;
	encryptor.encrypt(&json_data)
}

/// Helper function to decrypt and deserialize configuration
fn decrypt_config<T: for<'de> Deserialize<'de>>(
	encrypted: &EncryptedConfig,
	encryptor: &ConfigEncryptor,
) -> Result<T, String> {
	let decrypted_data = encryptor.decrypt(encrypted)?;
	serde_json::from_slice(&decrypted_data).map_err(|e| format!("Deserialization failed: {}", e))
}

/// Helper function to save encrypted config to file
fn save_encrypted_to_file(path: &PathBuf, encrypted: &EncryptedConfig) -> Result<(), String> {
	let json = serde_json::to_string_pretty(encrypted)
		.map_err(|e| format!("Failed to serialize encrypted data: {}", e))?;
	fs::write(path, json).map_err(|e| format!("Failed to write file: {}", e))
}

/// Helper function to load encrypted config from file
fn load_encrypted_from_file(path: &PathBuf) -> Result<EncryptedConfig, String> {
	let json = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
	serde_json::from_str(&json).map_err(|e| format!("Failed to deserialize encrypted data: {}", e))
}

#[cfg(feature = "encryption")]
mod encrypted_storage_tests {
	use super::*;

	/// Test: Basic encryption and file storage workflow
	///
	/// Validates that configuration can be encrypted and saved to a file,
	/// then loaded and decrypted successfully.
	#[rstest]
	fn test_encrypt_save_load_decrypt_workflow(temp_dir: TempDir, encryption_key: Vec<u8>) {
		let config = TestConfig::default();
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Encrypt configuration
		let encrypted = encrypt_config(&config, &encryptor).expect("Failed to encrypt config");

		// Save to file
		let file_path = temp_dir.path().join("config.encrypted.json");
		save_encrypted_to_file(&file_path, &encrypted).expect("Failed to save encrypted file");

		// Verify file exists
		assert!(file_path.exists());

		// Load from file
		let loaded_encrypted =
			load_encrypted_from_file(&file_path).expect("Failed to load encrypted file");

		// Decrypt and verify
		let decrypted_config: TestConfig =
			decrypt_config(&loaded_encrypted, &encryptor).expect("Failed to decrypt config");

		assert_eq!(decrypted_config, config);
	}

	/// Test: Encryption key management - wrong key fails decryption
	///
	/// Validates that attempting to decrypt with a different key fails
	/// with appropriate error message.
	#[rstest]
	fn test_wrong_encryption_key_fails(encryption_key: Vec<u8>, alternative_key: Vec<u8>) {
		let config = TestConfig::default();
		let encryptor1 = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor1");
		let encryptor2 =
			ConfigEncryptor::new(alternative_key).expect("Failed to create encryptor2");

		// Encrypt with first key
		let encrypted = encrypt_config(&config, &encryptor1).expect("Failed to encrypt config");

		// Attempt to decrypt with second key
		let result = decrypt_config::<TestConfig>(&encrypted, &encryptor2);

		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Decryption failed"));
	}

	/// Test: Decryption on load with correct key
	///
	/// Validates that encrypted configuration loaded from file can be
	/// decrypted successfully with the correct key.
	#[rstest]
	fn test_decryption_on_load_with_correct_key(temp_dir: TempDir, encryption_key: Vec<u8>) {
		let original_config = TestConfig::default();
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Encrypt and save
		let encrypted =
			encrypt_config(&original_config, &encryptor).expect("Failed to encrypt config");
		let file_path = temp_dir.path().join("config.encrypted.json");
		save_encrypted_to_file(&file_path, &encrypted).expect("Failed to save encrypted file");

		// Load and decrypt
		let loaded_encrypted =
			load_encrypted_from_file(&file_path).expect("Failed to load encrypted file");
		let loaded_config: TestConfig =
			decrypt_config(&loaded_encrypted, &encryptor).expect("Failed to decrypt config");

		assert_eq!(loaded_config, original_config);
		assert_eq!(loaded_config.database_url, original_config.database_url);
		assert_eq!(loaded_config.api_key, original_config.api_key);
	}

	/// Test: Re-encryption on save with same key
	///
	/// Validates that configuration can be modified, re-encrypted with the same key,
	/// and saved successfully. The new ciphertext should differ due to random nonce.
	#[rstest]
	fn test_re_encryption_on_save_same_key(temp_dir: TempDir, encryption_key: Vec<u8>) {
		let mut config = TestConfig::default();
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// First encryption
		let encrypted1 = encrypt_config(&config, &encryptor).expect("Failed to encrypt config");
		let file_path = temp_dir.path().join("config.encrypted.json");
		save_encrypted_to_file(&file_path, &encrypted1).expect("Failed to save encrypted file");

		// Modify configuration
		config.api_key = "new_secret_key_67890".to_string();
		config.max_connections = 200;

		// Re-encrypt
		let encrypted2 = encrypt_config(&config, &encryptor).expect("Failed to re-encrypt config");

		// Nonce should be different (random)
		assert_ne!(encrypted1.nonce, encrypted2.nonce);

		// Ciphertext should be different
		assert_ne!(encrypted1.data, encrypted2.data);

		// Save re-encrypted data
		save_encrypted_to_file(&file_path, &encrypted2).expect("Failed to save re-encrypted file");

		// Load and verify modified data
		let loaded_encrypted =
			load_encrypted_from_file(&file_path).expect("Failed to load encrypted file");
		let loaded_config: TestConfig =
			decrypt_config(&loaded_encrypted, &encryptor).expect("Failed to decrypt config");

		assert_eq!(loaded_config, config);
		assert_eq!(loaded_config.api_key, "new_secret_key_67890");
		assert_eq!(loaded_config.max_connections, 200);
	}

	/// Test: Re-encryption with different key (key rotation)
	///
	/// Validates key rotation workflow: decrypt with old key, re-encrypt with new key.
	#[rstest]
	fn test_re_encryption_with_different_key(
		temp_dir: TempDir,
		encryption_key: Vec<u8>,
		alternative_key: Vec<u8>,
	) {
		let config = TestConfig::default();
		let encryptor_old =
			ConfigEncryptor::new(encryption_key).expect("Failed to create old encryptor");
		let encryptor_new =
			ConfigEncryptor::new(alternative_key).expect("Failed to create new encryptor");

		// Encrypt with old key
		let encrypted_old =
			encrypt_config(&config, &encryptor_old).expect("Failed to encrypt with old key");
		let file_path = temp_dir.path().join("config.encrypted.json");
		save_encrypted_to_file(&file_path, &encrypted_old).expect("Failed to save encrypted file");

		// Load and decrypt with old key
		let loaded_encrypted =
			load_encrypted_from_file(&file_path).expect("Failed to load encrypted file");
		let decrypted_config: TestConfig = decrypt_config(&loaded_encrypted, &encryptor_old)
			.expect("Failed to decrypt with old key");

		assert_eq!(decrypted_config, config);

		// Re-encrypt with new key
		let encrypted_new = encrypt_config(&decrypted_config, &encryptor_new)
			.expect("Failed to re-encrypt with new key");
		save_encrypted_to_file(&file_path, &encrypted_new)
			.expect("Failed to save re-encrypted file");

		// Load and decrypt with new key
		let loaded_new =
			load_encrypted_from_file(&file_path).expect("Failed to load re-encrypted file");
		let final_config: TestConfig =
			decrypt_config(&loaded_new, &encryptor_new).expect("Failed to decrypt with new key");

		assert_eq!(final_config, config);

		// Verify old key no longer works
		let result = decrypt_config::<TestConfig>(&loaded_new, &encryptor_old);
		assert!(result.is_err());
	}

	/// Test: Encryption algorithm selection (AES-256-GCM)
	///
	/// Validates that AES-256-GCM is used correctly:
	/// - 32-byte key requirement
	/// - 12-byte nonce generation
	/// - Authentication tag verification
	#[rstest]
	fn test_encryption_algorithm_selection(encryption_key: Vec<u8>) {
		let config = TestConfig::default();

		// Verify key length requirement
		assert_eq!(
			encryption_key.len(),
			32,
			"Key must be 32 bytes for AES-256-GCM"
		);

		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Encrypt
		let encrypted = encrypt_config(&config, &encryptor).expect("Failed to encrypt config");

		// Verify nonce length (12 bytes for GCM)
		assert_eq!(
			encrypted.nonce.len(),
			12,
			"Nonce must be 12 bytes for AES-256-GCM"
		);

		// Verify authentication tag is included (ciphertext should be longer than plaintext)
		let json_data = serde_json::to_vec(&config).expect("Failed to serialize config");
		assert!(
			encrypted.data.len() > json_data.len(),
			"Encrypted data should include authentication tag"
		);

		// Verify decryption with authentication
		let decrypted_config: TestConfig =
			decrypt_config(&encrypted, &encryptor).expect("Failed to decrypt config");
		assert_eq!(decrypted_config, config);
	}

	/// Test: Encryption with invalid key length
	///
	/// Validates that encryptor creation fails with non-32-byte keys.
	#[rstest]
	#[case(vec![0u8; 16], "16-byte key")]
	#[case(vec![0u8; 24], "24-byte key")]
	#[case(vec![0u8; 31], "31-byte key")]
	#[case(vec![0u8; 33], "33-byte key")]
	#[case(vec![0u8; 64], "64-byte key")]
	fn test_invalid_key_lengths(#[case] key: Vec<u8>, #[case] description: &str) {
		let result = ConfigEncryptor::new(key);
		assert!(
			result.is_err(),
			"{} should fail: expected error but got success",
			description
		);
		let error = result.unwrap_err();
		assert!(
			error.contains("Encryption key must be exactly 32 bytes"),
			"{} error should mention key length requirement, got: {}",
			description,
			error
		);
	}

	/// Test: Tamper detection - corrupted ciphertext
	///
	/// Validates that tampering with encrypted data is detected and decryption fails.
	#[rstest]
	fn test_tamper_detection_corrupted_ciphertext(encryption_key: Vec<u8>) {
		let config = TestConfig::default();
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Encrypt
		let mut encrypted = encrypt_config(&config, &encryptor).expect("Failed to encrypt config");

		// Tamper with ciphertext (flip one bit)
		if !encrypted.data.is_empty() {
			encrypted.data[0] ^= 1;
		}

		// Decryption should fail due to authentication tag mismatch
		let result = decrypt_config::<TestConfig>(&encrypted, &encryptor);
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Decryption failed"));
	}

	/// Test: Tamper detection - corrupted nonce
	///
	/// Validates that tampering with the nonce is detected and decryption fails.
	#[rstest]
	fn test_tamper_detection_corrupted_nonce(encryption_key: Vec<u8>) {
		let config = TestConfig::default();
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Encrypt
		let mut encrypted = encrypt_config(&config, &encryptor).expect("Failed to encrypt config");

		// Tamper with nonce (flip one bit)
		if !encrypted.nonce.is_empty() {
			encrypted.nonce[0] ^= 1;
		}

		// Decryption should fail
		let result = decrypt_config::<TestConfig>(&encrypted, &encryptor);
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Decryption failed"));
	}

	/// Test: Performance with large encrypted settings (1000 items)
	///
	/// Validates that encryption/decryption performs acceptably with large configurations.
	/// This test measures the time taken for encryption and decryption operations.
	#[rstest]
	fn test_performance_with_large_encrypted_settings_1000_items(
		temp_dir: TempDir,
		encryption_key: Vec<u8>,
	) {
		let large_config = LargeConfig::generate(1000);
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Measure encryption time
		let start_encrypt = std::time::Instant::now();
		let encrypted =
			encrypt_config(&large_config, &encryptor).expect("Failed to encrypt large config");
		let encrypt_duration = start_encrypt.elapsed();

		println!("Encryption time for 1000 items: {:?}", encrypt_duration);

		// Save to file
		let file_path = temp_dir.path().join("large_config.encrypted.json");
		save_encrypted_to_file(&file_path, &encrypted)
			.expect("Failed to save large encrypted file");

		// Verify file size
		let metadata = fs::metadata(&file_path).expect("Failed to read file metadata");
		println!("Encrypted file size: {} bytes", metadata.len());

		// Load from file
		let loaded_encrypted =
			load_encrypted_from_file(&file_path).expect("Failed to load large encrypted file");

		// Measure decryption time
		let start_decrypt = std::time::Instant::now();
		let decrypted_config: LargeConfig =
			decrypt_config(&loaded_encrypted, &encryptor).expect("Failed to decrypt large config");
		let decrypt_duration = start_decrypt.elapsed();

		println!("Decryption time for 1000 items: {:?}", decrypt_duration);

		// Verify correctness
		assert_eq!(decrypted_config, large_config);
		assert_eq!(decrypted_config.items.len(), 1000);

		// Performance assertions (should complete in reasonable time)
		assert!(
			encrypt_duration.as_secs() < 5,
			"Encryption took too long: {:?}",
			encrypt_duration
		);
		assert!(
			decrypt_duration.as_secs() < 5,
			"Decryption took too long: {:?}",
			decrypt_duration
		);
	}

	/// Test: Performance with very large encrypted settings (10000 items)
	///
	/// Stress test for encryption/decryption performance with very large configurations.
	#[rstest]
	fn test_performance_with_large_encrypted_settings_10000_items(
		temp_dir: TempDir,
		encryption_key: Vec<u8>,
	) {
		let very_large_config = LargeConfig::generate(10000);
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Measure encryption time
		let start_encrypt = std::time::Instant::now();
		let encrypted = encrypt_config(&very_large_config, &encryptor)
			.expect("Failed to encrypt very large config");
		let encrypt_duration = start_encrypt.elapsed();

		println!("Encryption time for 10000 items: {:?}", encrypt_duration);

		// Save to file
		let file_path = temp_dir.path().join("very_large_config.encrypted.json");
		save_encrypted_to_file(&file_path, &encrypted)
			.expect("Failed to save very large encrypted file");

		// Verify file size
		let metadata = fs::metadata(&file_path).expect("Failed to read file metadata");
		println!("Encrypted file size: {} bytes", metadata.len());

		// Load from file
		let loaded_encrypted =
			load_encrypted_from_file(&file_path).expect("Failed to load very large encrypted file");

		// Measure decryption time
		let start_decrypt = std::time::Instant::now();
		let decrypted_config: LargeConfig = decrypt_config(&loaded_encrypted, &encryptor)
			.expect("Failed to decrypt very large config");
		let decrypt_duration = start_decrypt.elapsed();

		println!("Decryption time for 10000 items: {:?}", decrypt_duration);

		// Verify correctness
		assert_eq!(decrypted_config.items.len(), 10000);
		assert_eq!(decrypted_config.items[0].id, 0);
		assert_eq!(decrypted_config.items[9999].id, 9999);

		// Performance assertions (should complete in reasonable time)
		assert!(
			encrypt_duration.as_secs() < 30,
			"Encryption took too long: {:?}",
			encrypt_duration
		);
		assert!(
			decrypt_duration.as_secs() < 30,
			"Decryption took too long: {:?}",
			decrypt_duration
		);
	}

	/// Test: Multiple encrypted files with different keys
	///
	/// Validates that multiple encrypted configuration files can coexist
	/// with different encryption keys.
	#[rstest]
	fn test_multiple_encrypted_files_with_different_keys(temp_dir: TempDir) {
		let key1 = vec![1u8; 32];
		let key2 = vec![2u8; 32];
		let key3 = vec![3u8; 32];

		let encryptor1 = ConfigEncryptor::new(key1).expect("Failed to create encryptor1");
		let encryptor2 = ConfigEncryptor::new(key2).expect("Failed to create encryptor2");
		let encryptor3 = ConfigEncryptor::new(key3).expect("Failed to create encryptor3");

		// Create different configs
		let mut config1 = TestConfig::default();
		config1.api_key = "key_for_encryptor_1".to_string();

		let mut config2 = TestConfig::default();
		config2.api_key = "key_for_encryptor_2".to_string();

		let mut config3 = TestConfig::default();
		config3.api_key = "key_for_encryptor_3".to_string();

		// Encrypt with different keys
		let encrypted1 = encrypt_config(&config1, &encryptor1).expect("Failed to encrypt config1");
		let encrypted2 = encrypt_config(&config2, &encryptor2).expect("Failed to encrypt config2");
		let encrypted3 = encrypt_config(&config3, &encryptor3).expect("Failed to encrypt config3");

		// Save to different files
		let file1 = temp_dir.path().join("config1.encrypted.json");
		let file2 = temp_dir.path().join("config2.encrypted.json");
		let file3 = temp_dir.path().join("config3.encrypted.json");

		save_encrypted_to_file(&file1, &encrypted1).expect("Failed to save config1");
		save_encrypted_to_file(&file2, &encrypted2).expect("Failed to save config2");
		save_encrypted_to_file(&file3, &encrypted3).expect("Failed to save config3");

		// Load and decrypt with corresponding keys
		let loaded1 = load_encrypted_from_file(&file1).expect("Failed to load config1");
		let loaded2 = load_encrypted_from_file(&file2).expect("Failed to load config2");
		let loaded3 = load_encrypted_from_file(&file3).expect("Failed to load config3");

		let decrypted1: TestConfig =
			decrypt_config(&loaded1, &encryptor1).expect("Failed to decrypt config1");
		let decrypted2: TestConfig =
			decrypt_config(&loaded2, &encryptor2).expect("Failed to decrypt config2");
		let decrypted3: TestConfig =
			decrypt_config(&loaded3, &encryptor3).expect("Failed to decrypt config3");

		assert_eq!(decrypted1.api_key, "key_for_encryptor_1");
		assert_eq!(decrypted2.api_key, "key_for_encryptor_2");
		assert_eq!(decrypted3.api_key, "key_for_encryptor_3");

		// Verify wrong key fails
		let wrong_result = decrypt_config::<TestConfig>(&loaded1, &encryptor2);
		assert!(wrong_result.is_err());
	}

	/// Test: Encryption key derivation from password
	///
	/// Demonstrates key derivation from a user-provided password using PBKDF2.
	#[rstest]
	fn test_encryption_key_derivation_from_password() {
		use sha2::Sha256;

		let password = b"user_password_123";
		let salt = b"random_salt_value";

		// Derive 32-byte key using PBKDF2-SHA256
		let mut key = [0u8; 32];
		pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, 100_000, &mut key);

		// Create encryptor with derived key
		let encryptor = ConfigEncryptor::new(key.to_vec()).expect("Failed to create encryptor");

		let config = TestConfig::default();
		let encrypted = encrypt_config(&config, &encryptor).expect("Failed to encrypt config");

		// Derive the same key again
		let mut key2 = [0u8; 32];
		pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, 100_000, &mut key2);

		let encryptor2 = ConfigEncryptor::new(key2.to_vec()).expect("Failed to create encryptor2");

		// Decrypt with re-derived key
		let decrypted_config: TestConfig =
			decrypt_config(&encrypted, &encryptor2).expect("Failed to decrypt config");

		assert_eq!(decrypted_config, config);
	}

	/// Test: Encryption with empty configuration
	///
	/// Validates that empty configuration can be encrypted and decrypted.
	#[rstest]
	fn test_encryption_with_empty_configuration(temp_dir: TempDir, encryption_key: Vec<u8>) {
		#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
		struct EmptyConfig {}

		let empty_config = EmptyConfig {};
		let encryptor = ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor");

		// Encrypt
		let encrypted =
			encrypt_config(&empty_config, &encryptor).expect("Failed to encrypt empty config");

		// Save
		let file_path = temp_dir.path().join("empty_config.encrypted.json");
		save_encrypted_to_file(&file_path, &encrypted)
			.expect("Failed to save empty encrypted file");

		// Load and decrypt
		let loaded_encrypted =
			load_encrypted_from_file(&file_path).expect("Failed to load empty encrypted file");
		let decrypted_config: EmptyConfig =
			decrypt_config(&loaded_encrypted, &encryptor).expect("Failed to decrypt empty config");

		assert_eq!(decrypted_config, empty_config);
	}

	/// Test: Concurrent encryption operations
	///
	/// Validates that multiple encryption operations can be performed concurrently
	/// without interference.
	#[rstest]
	fn test_concurrent_encryption_operations(encryption_key: Vec<u8>) {
		use std::sync::Arc;
		use std::thread;

		let encryptor =
			Arc::new(ConfigEncryptor::new(encryption_key).expect("Failed to create encryptor"));
		let mut handles = vec![];

		// Spawn multiple threads encrypting different configs
		for i in 0..10 {
			let encryptor_clone = Arc::clone(&encryptor);
			let handle = thread::spawn(move || {
				let mut config = TestConfig::default();
				config.api_key = format!("api_key_{}", i);

				let encrypted = encrypt_config(&config, &encryptor_clone)
					.expect("Failed to encrypt config in thread");

				let decrypted: TestConfig = decrypt_config(&encrypted, &encryptor_clone)
					.expect("Failed to decrypt config in thread");

				assert_eq!(decrypted.api_key, format!("api_key_{}", i));
			});
			handles.push(handle);
		}

		// Wait for all threads
		for handle in handles {
			handle.join().expect("Thread panicked");
		}
	}
}

#[cfg(not(feature = "encryption"))]
mod encryption_fallback_tests {
	use super::*;

	/// Test: Fallback behavior without encryption feature
	///
	/// Validates that without the encryption feature, data is stored as-is.
	#[rstest]
	fn test_fallback_without_encryption_feature(temp_dir: TempDir) {
		let key = vec![1u8; 16]; // Any non-empty key
		let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

		let config = TestConfig::default();
		let encrypted = encrypt_config(&config, &encryptor).expect("Failed to encrypt config");

		// Without encryption feature, data should be unchanged
		let json_data = serde_json::to_vec(&config).expect("Failed to serialize config");
		assert_eq!(encrypted.data, json_data);

		// Save and load
		let file_path = temp_dir.path().join("config.fallback.json");
		save_encrypted_to_file(&file_path, &encrypted).expect("Failed to save file");

		let loaded_encrypted = load_encrypted_from_file(&file_path).expect("Failed to load file");
		let decrypted_config: TestConfig =
			decrypt_config(&loaded_encrypted, &encryptor).expect("Failed to decrypt config");

		assert_eq!(decrypted_config, config);
	}
}
