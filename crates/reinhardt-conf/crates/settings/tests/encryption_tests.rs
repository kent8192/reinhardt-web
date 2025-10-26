//! Tests for configuration encryption/decryption

use reinhardt_settings::encryption::{ConfigEncryptor, EncryptedConfig};

#[cfg(feature = "encryption")]
mod encryption_tests {
    use super::*;

    #[test]
    fn test_encryptor_new_with_valid_key() {
        let key = vec![0u8; 32]; // 32-byte key for AES-256
        let result = ConfigEncryptor::new(key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encryptor_new_with_invalid_key_length() {
        let key = vec![0u8; 16]; // Invalid: only 16 bytes
        let result = ConfigEncryptor::new(key);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Encryption key must be exactly 32 bytes")
        );
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = vec![42u8; 32]; // Use a non-zero key
        let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

        let plaintext = b"secret configuration data";
        let encrypted = encryptor
            .encrypt(plaintext)
            .expect("Failed to encrypt data");

        // Verify nonce is 12 bytes
        assert_eq!(encrypted.nonce.len(), 12);

        // Verify encrypted data is different from plaintext
        assert_ne!(encrypted.data, plaintext);

        // Decrypt and verify roundtrip
        let decrypted = encryptor
            .decrypt(&encrypted)
            .expect("Failed to decrypt data");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let key = vec![42u8; 32];
        let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

        let plaintext = b"secret data";

        // Encrypt the same plaintext twice
        let encrypted1 = encryptor.encrypt(plaintext).expect("Failed to encrypt");
        let encrypted2 = encryptor.encrypt(plaintext).expect("Failed to encrypt");

        // Nonces should be different (random)
        assert_ne!(encrypted1.nonce, encrypted2.nonce);

        // Ciphertexts should be different due to different nonces
        assert_ne!(encrypted1.data, encrypted2.data);

        // Both should decrypt to the same plaintext
        assert_eq!(encryptor.decrypt(&encrypted1).unwrap(), plaintext.to_vec());
        assert_eq!(encryptor.decrypt(&encrypted2).unwrap(), plaintext.to_vec());
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let key1 = vec![1u8; 32];
        let key2 = vec![2u8; 32];

        let encryptor1 = ConfigEncryptor::new(key1).expect("Failed to create encryptor1");
        let encryptor2 = ConfigEncryptor::new(key2).expect("Failed to create encryptor2");

        let plaintext = b"secret data";
        let encrypted = encryptor1.encrypt(plaintext).expect("Failed to encrypt");

        // Attempt to decrypt with wrong key
        let result = encryptor2.decrypt(&encrypted);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Decryption failed"));
    }

    #[test]
    fn test_decrypt_with_invalid_nonce_length_fails() {
        let key = vec![42u8; 32];
        let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

        let invalid_encrypted = EncryptedConfig::new(vec![1, 2, 3], vec![0; 8]); // Wrong nonce length

        let result = encryptor.decrypt(&invalid_encrypted);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid nonce length"));
    }

    #[test]
    fn test_decrypt_with_tampered_data_fails() {
        let key = vec![42u8; 32];
        let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

        let plaintext = b"secret data";
        let mut encrypted = encryptor.encrypt(plaintext).expect("Failed to encrypt");

        // Tamper with the ciphertext
        if !encrypted.data.is_empty() {
            encrypted.data[0] ^= 1; // Flip one bit
        }

        // Decryption should fail due to authentication tag mismatch
        let result = encryptor.decrypt(&encrypted);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Decryption failed"));
    }

    #[test]
    fn test_encrypt_empty_data() {
        let key = vec![42u8; 32];
        let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

        let plaintext = b"";
        let encrypted = encryptor.encrypt(plaintext).expect("Failed to encrypt");

        let decrypted = encryptor.decrypt(&encrypted).expect("Failed to decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_large_data() {
        let key = vec![42u8; 32];
        let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

        // Test with 1MB of data
        let plaintext = vec![65u8; 1024 * 1024]; // 1MB of 'A's
        let encrypted = encryptor.encrypt(&plaintext).expect("Failed to encrypt");

        let decrypted = encryptor.decrypt(&encrypted).expect("Failed to decrypt");
        assert_eq!(decrypted, plaintext);
    }
}

#[cfg(not(feature = "encryption"))]
mod encryption_fallback_tests {
    use super::*;

    #[test]
    fn test_encryptor_new_without_encryption_feature() {
        let key = vec![1, 2, 3]; // Any non-empty key
        let result = ConfigEncryptor::new(key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encryptor_new_empty_key_fails() {
        let key = vec![];
        let result = ConfigEncryptor::new(key);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Encryption key cannot be empty")
        );
    }

    #[test]
    fn test_encrypt_decrypt_fallback() {
        let key = vec![1, 2, 3];
        let encryptor = ConfigEncryptor::new(key).expect("Failed to create encryptor");

        let plaintext = b"test data";
        let encrypted = encryptor
            .encrypt(plaintext)
            .expect("Failed to encrypt data");

        // Without encryption feature, data should be unchanged
        assert_eq!(encrypted.data, plaintext);
        assert_eq!(encrypted.nonce.len(), 12);

        let decrypted = encryptor
            .decrypt(&encrypted)
            .expect("Failed to decrypt data");
        assert_eq!(decrypted, plaintext);
    }
}
