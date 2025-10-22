//! Encryption integration tests

use reinhardt_settings::encryption::ConfigEncryptor;
use std::fs;
use std::sync::Once;
use tempfile::TempDir;

// Test helpers inlined from common module

static INIT: Once = Once::new();

fn init_test_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}

#[test]
fn test_file_encryption_roundtrip() {
    init_test_logging();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");
    let encrypted_path = temp_dir.path().join("config.enc");
    let decrypted_path = temp_dir.path().join("config.dec");

    // Create original config file
    let original_content = r#"
[database]
host = "localhost"
port = 5432
password = "secret123"

[api]
key = "api_secret_key"
"#;

    fs::write(&config_path, original_content).expect("Failed to write config");

    // Generate encryption key
    let key = ConfigEncryptor::generate_key();
    let encryptor = ConfigEncryptor::new(key);

    // Encrypt file
    let content = fs::read(&config_path).expect("Failed to read config");
    let encrypted_config = encryptor.encrypt(&content).expect("Failed to encrypt");
    let encrypted_json = serde_json::to_vec(&encrypted_config).expect("Failed to serialize");
    fs::write(&encrypted_path, encrypted_json).expect("Failed to write encrypted file");

    // Decrypt file
    let encrypted_content = fs::read(&encrypted_path).expect("Failed to read encrypted file");
    let encrypted_config: reinhardt_settings::encryption::EncryptedConfig =
        serde_json::from_slice(&encrypted_content).expect("Failed to deserialize");
    let decrypted_content = encryptor
        .decrypt(&encrypted_config)
        .expect("Failed to decrypt");
    fs::write(&decrypted_path, &decrypted_content).expect("Failed to write decrypted file");

    // Verify content matches
    let decrypted_str = String::from_utf8(decrypted_content).expect("Failed to convert to string");
    assert_eq!(original_content, decrypted_str);
}

#[test]
fn test_encryption_key_rotation() {
    init_test_logging();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let original_content = "sensitive configuration data";

    // Encrypt with old key
    let old_key = [1u8; 32];
    let old_encryptor = ConfigEncryptor::new(old_key);
    let encrypted_config = old_encryptor
        .encrypt(original_content.as_bytes())
        .expect("Failed to encrypt with old key");

    // Decrypt with old key
    let decrypted = old_encryptor
        .decrypt(&encrypted_config)
        .expect("Failed to decrypt with old key");

    // Re-encrypt with new key
    let new_key = [2u8; 32];
    let new_encryptor = ConfigEncryptor::new(new_key);
    let re_encrypted_config = new_encryptor
        .encrypt(&decrypted)
        .expect("Failed to re-encrypt with new key");

    // Verify decryption with new key
    let final_decrypted = new_encryptor
        .decrypt(&re_encrypted_config)
        .expect("Failed to decrypt with new key");

    assert_eq!(
        String::from_utf8(final_decrypted).unwrap(),
        original_content
    );
}

#[test]
fn test_encrypted_config_loading() {
    init_test_logging();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("encrypted.json");

    // Create a config structure
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct AppConfig {
        database_url: String,
        api_key: String,
        debug: bool,
    }

    let config = AppConfig {
        database_url: "postgres://localhost/mydb".to_string(),
        api_key: "secret_api_key_12345".to_string(),
        debug: false,
    };

    // Encrypt config
    let key = ConfigEncryptor::generate_key();
    let encryptor = ConfigEncryptor::new(key);

    let encrypted_config = encryptor
        .encrypt_json(&config)
        .expect("Failed to encrypt JSON");

    // Save encrypted config
    let encrypted_json = serde_json::to_string_pretty(&encrypted_config).unwrap();
    fs::write(&config_path, encrypted_json).expect("Failed to write encrypted config");

    // Load and decrypt config
    let encrypted_content = fs::read_to_string(&config_path).expect("Failed to read config");
    let encrypted_config: reinhardt_settings::encryption::EncryptedConfig =
        serde_json::from_str(&encrypted_content).expect("Failed to parse");

    let decrypted_config: AppConfig = encryptor
        .decrypt_json(&encrypted_config)
        .expect("Failed to decrypt JSON");

    // Verify
    assert_eq!(config, decrypted_config);
}

#[test]
fn test_encryption_with_wrong_key_fails() {
    init_test_logging();

    let original_content = "secret data";
    let correct_key = [42u8; 32];
    let wrong_key = [99u8; 32];

    // Encrypt with correct key
    let encryptor = ConfigEncryptor::new(correct_key);
    let encrypted_config = encryptor
        .encrypt(original_content.as_bytes())
        .expect("Failed to encrypt");

    // Try to decrypt with wrong key
    let wrong_encryptor = ConfigEncryptor::new(wrong_key);
    let result = wrong_encryptor.decrypt(&encrypted_config);

    assert!(result.is_err(), "Decryption should fail with wrong key");
}

#[test]
fn test_encryption_preserves_binary_data() {
    init_test_logging();

    // Test with binary data containing null bytes
    let binary_data = vec![0u8, 1, 2, 3, 255, 254, 253, 0, 100, 200];
    let key = ConfigEncryptor::generate_key();
    let encryptor = ConfigEncryptor::new(key);

    // Encrypt
    let encrypted_config = encryptor
        .encrypt(&binary_data)
        .expect("Failed to encrypt binary data");

    // Decrypt
    let decrypted = encryptor
        .decrypt(&encrypted_config)
        .expect("Failed to decrypt binary data");

    assert_eq!(binary_data, decrypted);
}

#[test]
fn test_encryption_with_password_derivation() {
    init_test_logging();

    let password = "user_password_123";
    let salt = b"random_salt_16_b"; // 16 bytes

    // Create encryptor from password
    let encryptor = ConfigEncryptor::from_password(password, salt)
        .expect("Failed to create encryptor from password");

    let original_content = "password-encrypted data";

    // Encrypt
    let encrypted_config = encryptor
        .encrypt(original_content.as_bytes())
        .expect("Failed to encrypt");

    // Create another encryptor with same password and salt
    let encryptor2 =
        ConfigEncryptor::from_password(password, salt).expect("Failed to create second encryptor");

    // Decrypt
    let decrypted = encryptor2
        .decrypt(&encrypted_config)
        .expect("Failed to decrypt");

    assert_eq!(String::from_utf8(decrypted).unwrap(), original_content);
}

#[test]
fn test_encryption_different_salts_different_keys() {
    init_test_logging();

    let password = "same_password";
    let salt1 = b"salt_version_001";
    let salt2 = b"salt_version_002";

    let encryptor1 =
        ConfigEncryptor::from_password(password, salt1).expect("Failed to create encryptor1");
    let encryptor2 =
        ConfigEncryptor::from_password(password, salt2).expect("Failed to create encryptor2");

    let original_content = "test data";

    // Encrypt with first encryptor
    let encrypted_config = encryptor1
        .encrypt(original_content.as_bytes())
        .expect("Failed to encrypt");

    // Try to decrypt with second encryptor (different salt = different key)
    let result = encryptor2.decrypt(&encrypted_config);

    assert!(
        result.is_err(),
        "Decryption should fail with different salt"
    );
}
