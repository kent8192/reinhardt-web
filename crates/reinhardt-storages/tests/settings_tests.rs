//! Tests for the settings-first storage configuration API.

#![allow(deprecated)] // Tests cover legacy compatibility conversion until removal.

use reinhardt_conf::settings::fragment::SettingsFragment;
use reinhardt_conf::settings::secret_types::SecretString;
use reinhardt_storages::{BackendType, StorageConfig, StorageError, StorageSettings};

#[test]
fn storage_settings_section_is_storage() {
	assert_eq!(StorageSettings::section(), "storage");
}

#[test]
#[cfg(feature = "gcs")]
fn deserializes_gcs_settings_from_toml() {
	let raw = r#"
backend = "gcs"

[gcs]
bucket = "assets"
prefix = "uploads/"
endpoint = "http://127.0.0.1:4443"
service_account_json = { secret = "{\"client_email\":\"test@example.com\"}" }
"#;

	let settings: StorageSettings = toml::from_str(raw).unwrap();

	assert_eq!(settings.backend, BackendType::Gcs);
	let gcs = settings.gcs.as_ref().unwrap();
	assert_eq!(gcs.bucket, "assets");
	assert_eq!(gcs.prefix.as_deref(), Some("uploads/"));
	assert_eq!(gcs.endpoint.as_deref(), Some("http://127.0.0.1:4443"));
	assert_eq!(
		gcs.service_account_json.as_ref().unwrap().expose_secret(),
		r#"{"client_email":"test@example.com"}"#
	);
}

#[test]
#[cfg(feature = "azure")]
fn rejects_selected_backend_without_matching_nested_settings() {
	let settings: StorageSettings = toml::from_str(r#"backend = "azure""#).unwrap();

	let result = settings.to_config();

	match result {
		Err(StorageError::ConfigError(message)) => {
			assert_eq!(message, "Selected backend requires [storage.azure] settings");
		}
		other => panic!("Expected ConfigError, got {other:?}"),
	}
}

#[test]
fn converts_local_settings_to_compat_config() {
	let settings: StorageSettings = toml::from_str(
		r#"
backend = "local"

[local]
base_path = "/tmp/reinhardt-storage"
"#,
	)
	.unwrap();

	let config = settings.to_config().unwrap();

	match config {
		StorageConfig::Local(local) => assert_eq!(local.base_path, "/tmp/reinhardt-storage"),
		other => panic!("expected local config, got {other:?}"),
	}
}

#[test]
fn secret_string_debug_redacts_credentials() {
	let secret = SecretString::new("super-secret-key");

	assert_eq!(format!("{secret:?}"), "SecretString([REDACTED])");
	assert_eq!(secret.expose_secret(), "super-secret-key");
}
