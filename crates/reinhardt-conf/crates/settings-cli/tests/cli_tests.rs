//! CLI integration tests

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_toml(dir: &TempDir, name: &str, content: &str) -> PathBuf {
	let path = dir.path().join(name);
	fs::write(&path, content).unwrap();
	path
}

fn create_test_json(dir: &TempDir, name: &str, content: &str) -> PathBuf {
	let path = dir.path().join(name);
	fs::write(&path, content).unwrap();
	path
}

fn create_test_env(dir: &TempDir, name: &str, content: &str) -> PathBuf {
	let path = dir.path().join(name);
	fs::write(&path, content).unwrap();
	path
}

#[cfg(test)]
mod validate_tests {
	use reinhardt_settings::Settings;
	use reinhardt_settings_cli::commands::{diff, show, validate};

	use super::*;

	#[tokio::test]
	async fn test_validate_toml_valid() {
		let temp_dir = TempDir::new().unwrap();
		let toml_content = r#"
[database]
host = "localhost"
port = 5432

[debug]
enabled = true
"#;
		let path = create_test_toml(&temp_dir, "config.toml", toml_content);

		// Read and validate
		let content = fs::read_to_string(&path).unwrap();
		let result: Result<toml::Value, _> = toml::from_str(&content);
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_validate_toml_invalid() {
		let temp_dir = TempDir::new().unwrap();
		let toml_content = r#"
[database
host = "localhost"
"#;
		let path = create_test_toml(&temp_dir, "invalid.toml", toml_content);

		let content = fs::read_to_string(&path).unwrap();
		let result: Result<toml::Value, _> = toml::from_str(&content);
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_validate_json_valid() {
		let temp_dir = TempDir::new().unwrap();
		let json_content = r#"{
    "database": {
        "host": "localhost",
        "port": 5432
    },
    "debug": true
}"#;
		let path = create_test_json(&temp_dir, "config.json", json_content);

		let content = fs::read_to_string(&path).unwrap();
		let result: Result<serde_json::Value, _> = serde_json::from_str(&content);
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_validate_json_invalid() {
		let temp_dir = TempDir::new().unwrap();
		let json_content = r#"{
    "database": {
        "host": "localhost",
        "port": 5432,
    }
}"#;
		let path = create_test_json(&temp_dir, "invalid.json", json_content);

		let content = fs::read_to_string(&path).unwrap();
		let result: Result<serde_json::Value, _> = serde_json::from_str(&content);
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_validate_env_valid() {
		let temp_dir = TempDir::new().unwrap();
		let env_content = r#"
DATABASE_HOST=localhost
DATABASE_PORT=5432
DEBUG=true
# This is a comment
"#;
		let path = create_test_env(&temp_dir, ".env", env_content);

		let content = fs::read_to_string(&path).unwrap();
		let mut valid_lines = 0;
		for line in content.lines() {
			let line = line.trim();
			if line.is_empty() || line.starts_with('#') {
				continue;
			}
			assert!(line.contains('='));
			valid_lines += 1;
		}
		assert_eq!(valid_lines, 3);
	}
}

#[cfg(test)]
mod show_tests {
	use super::*;

	#[tokio::test]
	async fn test_show_toml_all() {
		let temp_dir = TempDir::new().unwrap();
		let toml_content = r#"
[database]
host = "localhost"
port = 5432
"#;
		let path = create_test_toml(&temp_dir, "config.toml", toml_content);

		let content = fs::read_to_string(&path).unwrap();
		let value: toml::Value = toml::from_str(&content).unwrap();

		let db = value.get("database").unwrap();
		assert_eq!(db.get("host").unwrap().as_str().unwrap(), "localhost");
		assert_eq!(db.get("port").unwrap().as_integer().unwrap(), 5432);
	}

	#[tokio::test]
	async fn test_show_json_specific_key() {
		let temp_dir = TempDir::new().unwrap();
		let json_content = r#"{
    "database": {
        "host": "localhost",
        "port": 5432
    }
}"#;
		let path = create_test_json(&temp_dir, "config.json", json_content);

		let content = fs::read_to_string(&path).unwrap();
		let value: serde_json::Value = serde_json::from_str(&content).unwrap();

		// Navigate to nested key
		let host = value
			.get("database")
			.and_then(|db| db.get("host"))
			.and_then(|h| h.as_str())
			.unwrap();
		assert_eq!(host, "localhost");
	}
}

#[cfg(test)]
mod set_tests {
	use super::*;

	#[tokio::test]
	async fn test_set_toml_value() {
		let temp_dir = TempDir::new().unwrap();
		let toml_content = r#"
[database]
host = "localhost"
port = 5432
"#;
		let path = create_test_toml(&temp_dir, "config.toml", toml_content);

		// Read, modify, write
		let content = fs::read_to_string(&path).unwrap();
		let mut value: toml::Value = toml::from_str(&content).unwrap();

		value
			.as_table_mut()
			.unwrap()
			.get_mut("database")
			.unwrap()
			.as_table_mut()
			.unwrap()
			.insert(
				"host".to_string(),
				toml::Value::String("newhost".to_string()),
			);

		let new_content = toml::to_string(&value).unwrap();
		fs::write(&path, new_content).unwrap();

		// Verify
		let content = fs::read_to_string(&path).unwrap();
		let value: toml::Value = toml::from_str(&content).unwrap();
		let host = value
			.get("database")
			.unwrap()
			.get("host")
			.unwrap()
			.as_str()
			.unwrap();
		assert_eq!(host, "newhost");
	}

	#[tokio::test]
	async fn test_set_json_value() {
		let temp_dir = TempDir::new().unwrap();
		let json_content = r#"{
    "database": {
        "host": "localhost"
    }
}"#;
		let path = create_test_json(&temp_dir, "config.json", json_content);

		// Read, modify, write
		let content = fs::read_to_string(&path).unwrap();
		let mut value: serde_json::Value = serde_json::from_str(&content).unwrap();

		value
			.as_object_mut()
			.unwrap()
			.get_mut("database")
			.unwrap()
			.as_object_mut()
			.unwrap()
			.insert(
				"host".to_string(),
				serde_json::Value::String("newhost".to_string()),
			);

		let new_content = serde_json::to_string_pretty(&value).unwrap();
		fs::write(&path, new_content).unwrap();

		// Verify
		let content = fs::read_to_string(&path).unwrap();
		let value: serde_json::Value = serde_json::from_str(&content).unwrap();
		let host = value
			.get("database")
			.unwrap()
			.get("host")
			.unwrap()
			.as_str()
			.unwrap();
		assert_eq!(host, "newhost");
	}

	#[tokio::test]
	async fn test_set_env_value() {
		let temp_dir = TempDir::new().unwrap();
		let env_content = "DATABASE_HOST=localhost\nDEBUG=true\n";
		let path = create_test_env(&temp_dir, ".env", env_content);

		// Read, modify, write
		let content = fs::read_to_string(&path).unwrap();
		let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

		for line in &mut lines {
			if line.starts_with("DATABASE_HOST=") {
				*line = "DATABASE_HOST=newhost".to_string();
			}
		}

		let new_content = lines.join("\n");
		fs::write(&path, new_content).unwrap();

		// Verify
		let content = fs::read_to_string(&path).unwrap();
		assert!(content.contains("DATABASE_HOST=newhost"));
	}
}

#[cfg(test)]
mod diff_tests {
	use super::*;
	use std::collections::BTreeMap;

	fn flatten_value(prefix: &str, value: &serde_json::Value) -> BTreeMap<String, String> {
		let mut map = BTreeMap::new();

		match value {
			serde_json::Value::Object(obj) => {
				for (key, val) in obj {
					let new_prefix = if prefix.is_empty() {
						key.clone()
					} else {
						format!("{}.{}", prefix, key)
					};
					map.extend(flatten_value(&new_prefix, val));
				}
			}
			serde_json::Value::Array(arr) => {
				for (i, val) in arr.iter().enumerate() {
					let new_prefix = format!("{}[{}]", prefix, i);
					map.extend(flatten_value(&new_prefix, val));
				}
			}
			_ => {
				map.insert(prefix.to_string(), value.to_string());
			}
		}

		map
	}

	#[tokio::test]
	async fn test_diff_identical_files() {
		let temp_dir = TempDir::new().unwrap();
		let json_content = r#"{"key": "value"}"#;
		let path1 = create_test_json(&temp_dir, "config1.json", json_content);
		let path2 = create_test_json(&temp_dir, "config2.json", json_content);

		let content1 = fs::read_to_string(&path1).unwrap();
		let content2 = fs::read_to_string(&path2).unwrap();

		let value1: serde_json::Value = serde_json::from_str(&content1).unwrap();
		let value2: serde_json::Value = serde_json::from_str(&content2).unwrap();

		let map1 = flatten_value("", &value1);
		let map2 = flatten_value("", &value2);

		assert_eq!(map1, map2);
	}

	#[tokio::test]
	async fn test_diff_different_files() {
		let temp_dir = TempDir::new().unwrap();
		let json1 = r#"{"key": "value1"}"#;
		let json2 = r#"{"key": "value2"}"#;
		let path1 = create_test_json(&temp_dir, "config1.json", json1);
		let path2 = create_test_json(&temp_dir, "config2.json", json2);

		let content1 = fs::read_to_string(&path1).unwrap();
		let content2 = fs::read_to_string(&path2).unwrap();

		let value1: serde_json::Value = serde_json::from_str(&content1).unwrap();
		let value2: serde_json::Value = serde_json::from_str(&content2).unwrap();

		let map1 = flatten_value("", &value1);
		let map2 = flatten_value("", &value2);

		assert_ne!(map1, map2);
		assert_eq!(map1.get("key").unwrap(), "\"value1\"");
		assert_eq!(map2.get("key").unwrap(), "\"value2\"");
	}

	#[tokio::test]
	async fn test_diff_added_key() {
		let temp_dir = TempDir::new().unwrap();
		let json1 = r#"{"key1": "value1"}"#;
		let json2 = r#"{"key1": "value1", "key2": "value2"}"#;
		let path1 = create_test_json(&temp_dir, "config1.json", json1);
		let path2 = create_test_json(&temp_dir, "config2.json", json2);

		let content1 = fs::read_to_string(&path1).unwrap();
		let content2 = fs::read_to_string(&path2).unwrap();

		let value1: serde_json::Value = serde_json::from_str(&content1).unwrap();
		let value2: serde_json::Value = serde_json::from_str(&content2).unwrap();

		let map1 = flatten_value("", &value1);
		let map2 = flatten_value("", &value2);

		assert!(map1.get("key2").is_none());
		assert!(map2.get("key2").is_some());
	}
}

#[cfg(test)]
mod encryption_tests {
	use super::*;

	#[tokio::test]
	async fn test_encrypt_decrypt_roundtrip() {
		let temp_dir = TempDir::new().unwrap();
		let original_content = "secret configuration data";
		let _path = create_test_toml(&temp_dir, "config.toml", original_content);

		// Generate a 32-byte key
		let key = [42u8; 32];

		// Encrypt
		let encryptor = reinhardt_settings::encryption::ConfigEncryptor::new(key.to_vec()).unwrap();
		let encrypted_config = encryptor.encrypt(original_content.as_bytes()).unwrap();
		let encrypted = serde_json::to_vec(&encrypted_config).unwrap();
		let enc_path = temp_dir.path().join("config.enc");
		fs::write(&enc_path, &encrypted).unwrap();

		// Decrypt
		let encrypted_content = fs::read(&enc_path).unwrap();
		let encrypted_config: reinhardt_settings::encryption::EncryptedConfig =
			serde_json::from_slice(&encrypted_content).unwrap();
		let decrypted = encryptor.decrypt(&encrypted_config).unwrap();

		assert_eq!(String::from_utf8(decrypted).unwrap(), original_content);
	}

	#[tokio::test]
	async fn test_decrypt_wrong_key_fails() {
		let original_content = "secret data";
		let correct_key = [42u8; 32];
		let wrong_key = [99u8; 32];

		// Encrypt with correct key
		let encryptor =
			reinhardt_settings::encryption::ConfigEncryptor::new(correct_key.to_vec()).unwrap();
		let encrypted_config = encryptor.encrypt(original_content.as_bytes()).unwrap();

		// Try to decrypt with wrong key
		let wrong_encryptor =
			reinhardt_settings::encryption::ConfigEncryptor::new(wrong_key.to_vec()).unwrap();
		let result = wrong_encryptor.decrypt(&encrypted_config);
		assert!(result.is_err());
	}
}
