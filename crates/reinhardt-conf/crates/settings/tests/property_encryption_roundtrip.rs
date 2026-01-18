//! Property-Based Tests for Encryption Roundtrip Invariant.
//!
//! This test module validates that encryption is reversible for all valid inputs,
//! ensuring that decrypt(encrypt(data, key), key) == data for any configuration data.
//!
//! NOTE: These tests are feature-gated with "encryption" feature.

#![cfg(feature = "encryption")]

use quickcheck::{Arbitrary, Gen};
use quickcheck_macros::quickcheck;
use reinhardt_conf::settings::encryption::ConfigEncryptor;
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Test configuration structure for property-based testing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestConfig {
	database_url: String,
	debug: bool,
	max_connections: i32,
	api_keys: Vec<String>,
}

impl Arbitrary for TestConfig {
	fn arbitrary(g: &mut Gen) -> Self {
		TestConfig {
			database_url: String::arbitrary(g),
			debug: bool::arbitrary(g),
			max_connections: i32::arbitrary(g),
			api_keys: vec![
				String::arbitrary(g),
				String::arbitrary(g),
				String::arbitrary(g),
			],
		}
	}
}

/// Test: Encryption roundtrip property for TestConfig
///
/// Why: Validates that for any valid configuration and 32-byte key,
/// decrypting encrypted data returns the original configuration.
#[quickcheck]
fn quickcheck_encryption_roundtrip(config: TestConfig) -> bool {
	// Use fixed 32-byte key (256 bits for AES-256-GCM)
	let key = vec![0x42; 32];

	// Skip test if encryptor creation fails
	let encryptor = match ConfigEncryptor::new(key) {
		Ok(enc) => enc,
		Err(_) => return true,
	};

	// Serialize config
	let config_bytes = match serde_json::to_vec(&config) {
		Ok(bytes) => bytes,
		Err(_) => return true,
	};

	// Encrypt
	let encrypted = match encryptor.encrypt(&config_bytes) {
		Ok(enc) => enc,
		Err(_) => return false,
	};

	// Decrypt
	let decrypted_bytes = match encryptor.decrypt(&encrypted) {
		Ok(dec) => dec,
		Err(_) => return false,
	};

	// Deserialize
	let decrypted_config: TestConfig = match serde_json::from_slice(&decrypted_bytes) {
		Ok(cfg) => cfg,
		Err(_) => return false,
	};

	// Verify roundtrip property
	decrypted_config == config
}

/// Test: Encryption roundtrip with different key lengths
///
/// Why: Validates that only 32-byte keys (256 bits) are accepted,
/// and shorter/longer keys are rejected gracefully.
#[rstest]
#[case(16, false)] // 128 bits - too short
#[case(24, false)] // 192 bits - not supported
#[case(32, true)] // 256 bits - valid
#[case(64, false)] // 512 bits - too long
fn test_encryption_key_length_validation(#[case] key_length: usize, #[case] should_succeed: bool) {
	let key = vec![0x42; key_length];
	let result = ConfigEncryptor::new(key);

	assert_eq!(
		result.is_ok(),
		should_succeed,
		"Key length {} should {} succeed",
		key_length,
		if should_succeed { "" } else { "not" }
	);
}

/// Test: Encryption roundtrip with different data sizes
///
/// Why: Validates that encryption works correctly for various data sizes,
/// from empty to very large configurations.
#[rstest]
#[case(0)] // Empty data
#[case(1)] // Single byte
#[case(100)] // Small config
#[case(10_000)] // Medium config
#[case(100_000)] // Large config
fn test_encryption_roundtrip_various_sizes(#[case] data_size: usize) {
	let key = vec![0x42; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Encryptor creation should succeed");

	// Create data of specified size
	let data = vec![0xAB; data_size];

	// Encrypt
	let encrypted = encryptor.encrypt(&data).expect("Encryption should succeed");

	// Decrypt
	let decrypted = encryptor
		.decrypt(&encrypted)
		.expect("Decryption should succeed");

	// Verify roundtrip
	assert_eq!(
		decrypted, data,
		"Decrypted data should match original for size {}",
		data_size
	);
}

/// Test: Encryption roundtrip with Unicode data
///
/// Why: Validates that encryption correctly handles Unicode characters
/// in configuration values.
#[rstest]
#[case("Hello, World!")] // ASCII
#[case("日本語テスト")] // Japanese
#[case("Тестирование")] // Cyrillic
#[case("emoji 😀🎉🔒")] // Emoji
#[case("مرحبا")] // Arabic
fn test_encryption_roundtrip_unicode(#[case] text: &str) {
	let key = vec![0x42; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Encryptor creation should succeed");

	let config = json!({
		"message": text,
		"enabled": true
	});

	let config_bytes = serde_json::to_vec(&config).expect("Serialization should succeed");

	// Encrypt
	let encrypted = encryptor
		.encrypt(&config_bytes)
		.expect("Encryption should succeed");

	// Decrypt
	let decrypted = encryptor
		.decrypt(&encrypted)
		.expect("Decryption should succeed");

	let decrypted_config: serde_json::Value =
		serde_json::from_slice(&decrypted).expect("Deserialization should succeed");

	// Verify Unicode text preserved
	assert_eq!(
		decrypted_config["message"].as_str().unwrap(),
		text,
		"Unicode text should be preserved"
	);
}

/// Test: Different keys produce different ciphertexts
///
/// Why: Validates that the same plaintext encrypted with different keys
/// produces different ciphertexts (no key reuse detection).
#[rstest]
#[test]
fn test_different_keys_different_ciphertexts() {
	let data = b"sensitive configuration data";

	let key1 = vec![0x42; 32];
	let key2 = vec![0x43; 32];

	let encryptor1 = ConfigEncryptor::new(key1).expect("Encryptor1 creation should succeed");
	let encryptor2 = ConfigEncryptor::new(key2).expect("Encryptor2 creation should succeed");

	let encrypted1 = encryptor1
		.encrypt(data)
		.expect("Encryption with key1 should succeed");
	let encrypted2 = encryptor2
		.encrypt(data)
		.expect("Encryption with key2 should succeed");

	// Verify different ciphertexts
	assert_ne!(
		encrypted1.data, encrypted2.data,
		"Different keys should produce different ciphertexts"
	);
}

/// Test: Same plaintext encrypted twice produces different ciphertexts
///
/// Why: Validates that encryption uses unique nonces, ensuring the same
/// plaintext encrypted twice produces different ciphertexts (semantic security).
#[rstest]
#[test]
fn test_same_plaintext_different_ciphertexts() {
	let key = vec![0x42; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Encryptor creation should succeed");

	let data = b"sensitive configuration data";

	let encrypted1 = encryptor
		.encrypt(data)
		.expect("First encryption should succeed");
	let encrypted2 = encryptor
		.encrypt(data)
		.expect("Second encryption should succeed");

	// Verify different ciphertexts
	assert_ne!(
		encrypted1.data, encrypted2.data,
		"Same plaintext should produce different ciphertexts (due to unique nonces)"
	);

	// But both decrypt to same plaintext
	let decrypted1 = encryptor
		.decrypt(&encrypted1)
		.expect("First decryption should succeed");
	let decrypted2 = encryptor
		.decrypt(&encrypted2)
		.expect("Second decryption should succeed");

	assert_eq!(decrypted1, data);
	assert_eq!(decrypted2, data);
}

/// Test: Tampered ciphertext is detected
///
/// Why: Validates that AES-GCM authentication tag correctly detects
/// any tampering with the ciphertext.
#[rstest]
#[test]
fn test_tampered_ciphertext_detection() {
	let key = vec![0x42; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Encryptor creation should succeed");

	let data = b"sensitive configuration data";

	let mut encrypted = encryptor.encrypt(data).expect("Encryption should succeed");

	// Tamper with ciphertext (flip first byte)
	if !encrypted.data.is_empty() {
		encrypted.data[0] ^= 0xFF;
	}

	// Verify decryption fails due to tampering
	let result = encryptor.decrypt(&encrypted);
	assert!(
		result.is_err(),
		"Decryption should fail for tampered ciphertext"
	);
}

/// Test: Tampered nonce is detected
///
/// Why: Validates that modifying the nonce causes decryption to fail,
/// ensuring nonce integrity.
#[rstest]
#[test]
fn test_tampered_nonce_detection() {
	let key = vec![0x42; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Encryptor creation should succeed");

	let data = b"sensitive configuration data";

	let mut encrypted = encryptor.encrypt(data).expect("Encryption should succeed");

	// Tamper with nonce (flip first byte)
	if !encrypted.nonce.is_empty() {
		encrypted.nonce[0] ^= 0xFF;
	}

	// Verify decryption fails due to tampered nonce
	let result = encryptor.decrypt(&encrypted);
	assert!(result.is_err(), "Decryption should fail for tampered nonce");
}

/// Test: Wrong key cannot decrypt
///
/// Why: Validates that ciphertext encrypted with one key cannot be decrypted
/// with a different key (key confidentiality).
#[rstest]
#[test]
fn test_wrong_key_cannot_decrypt() {
	let key1 = vec![0x42; 32];
	let key2 = vec![0x43; 32];

	let encryptor1 = ConfigEncryptor::new(key1).expect("Encryptor1 creation should succeed");
	let encryptor2 = ConfigEncryptor::new(key2).expect("Encryptor2 creation should succeed");

	let data = b"sensitive configuration data";

	let encrypted = encryptor1.encrypt(data).expect("Encryption should succeed");

	// Verify decryption with wrong key fails
	let result = encryptor2.decrypt(&encrypted);
	assert!(result.is_err(), "Decryption with wrong key should fail");
}

/// Test: Encryption roundtrip with complex nested JSON
///
/// Why: Validates that encryption correctly handles complex nested structures
/// without data loss or corruption.
#[rstest]
#[test]
fn test_encryption_roundtrip_complex_json() {
	let key = vec![0x42; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Encryptor creation should succeed");

	let config = json!({
		"database": {
			"host": "localhost",
			"port": 5432,
			"credentials": {
				"username": "admin",
				"password": "secret123"
			},
			"pools": [
				{"name": "read", "size": 10},
				{"name": "write", "size": 5}
			]
		},
		"features": {
			"auth": true,
			"cache": false,
			"api": {
				"rate_limit": 1000,
				"timeout_ms": 5000
			}
		}
	});

	let config_bytes = serde_json::to_vec(&config).expect("Serialization should succeed");

	// Encrypt
	let encrypted = encryptor
		.encrypt(&config_bytes)
		.expect("Encryption should succeed");

	// Decrypt
	let decrypted = encryptor
		.decrypt(&encrypted)
		.expect("Decryption should succeed");

	let decrypted_config: serde_json::Value =
		serde_json::from_slice(&decrypted).expect("Deserialization should succeed");

	// Verify complete structure preserved
	assert_eq!(
		decrypted_config, config,
		"Complex nested JSON should be preserved"
	);
	assert_eq!(decrypted_config["database"]["port"].as_i64().unwrap(), 5432);
	assert_eq!(
		decrypted_config["database"]["credentials"]["password"]
			.as_str()
			.unwrap(),
		"secret123"
	);
	assert_eq!(
		decrypted_config["features"]["api"]["rate_limit"]
			.as_i64()
			.unwrap(),
		1000
	);
}

/// Test: Concurrent encryption/decryption operations
///
/// Why: Validates that ConfigEncryptor is safe to use concurrently
/// from multiple threads.
#[rstest]
#[test]
fn test_concurrent_encryption_operations() {
	use std::sync::Arc;
	use std::thread;

	let key = vec![0x42; 32];
	let encryptor = Arc::new(ConfigEncryptor::new(key).expect("Encryptor creation should succeed"));

	let test_data = vec![
		b"data1".to_vec(),
		b"data2".to_vec(),
		b"data3".to_vec(),
		b"data4".to_vec(),
		b"data5".to_vec(),
	];

	let mut handles = vec![];

	for data in test_data {
		let encryptor_clone = encryptor.clone();
		let handle = thread::spawn(move || {
			let encrypted = encryptor_clone.encrypt(&data).unwrap();
			let decrypted = encryptor_clone.decrypt(&encrypted).unwrap();
			assert_eq!(decrypted, data);
		});
		handles.push(handle);
	}

	for handle in handles {
		handle.join().expect("Thread should not panic");
	}
}

/// Test: Encryption with empty data
///
/// Why: Validates that encryption handles edge case of empty data correctly.
#[rstest]
#[test]
fn test_encryption_empty_data() {
	let key = vec![0x42; 32];
	let encryptor = ConfigEncryptor::new(key).expect("Encryptor creation should succeed");

	let empty_data = b"";

	let encrypted = encryptor
		.encrypt(empty_data)
		.expect("Encryption of empty data should succeed");

	let decrypted = encryptor
		.decrypt(&encrypted)
		.expect("Decryption of empty data should succeed");

	assert_eq!(decrypted, empty_data, "Empty data should be preserved");
}
