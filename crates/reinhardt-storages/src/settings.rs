//! Settings fragment for storage backends.

#![allow(deprecated)] // Settings conversion targets legacy config during the compatibility window.

use crate::config::{BackendType, StorageConfig};
use crate::{Result, StorageError};
#[cfg(any(feature = "azure", feature = "gcs"))]
use reinhardt_conf::settings::secret_types::SecretString;
use reinhardt_conf::settings::{
	fragment::SettingsValidation,
	profile::Profile,
	validation::{ValidationError, ValidationResult},
};
use reinhardt_core::macros::settings;
use serde::{Deserialize, Serialize};

fn default_backend() -> BackendType {
	BackendType::Local
}

/// Storage configuration fragment.
///
/// This fragment maps to the `[storage]` section and can be composed with the
/// `#[settings]` macro from downstream applications.
#[settings(fragment = true, section = "storage", validate = false)]
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageSettings {
	/// Selected storage backend.
	#[serde(default = "default_backend")]
	pub backend: BackendType,
	/// Amazon S3 backend settings.
	#[cfg(feature = "s3")]
	#[serde(default)]
	pub s3: Option<S3StorageSettings>,
	/// Google Cloud Storage backend settings.
	#[cfg(feature = "gcs")]
	#[serde(default)]
	pub gcs: Option<GcsStorageSettings>,
	/// Azure Blob Storage backend settings.
	#[cfg(feature = "azure")]
	#[serde(default)]
	pub azure: Option<AzureStorageSettings>,
	/// Local filesystem backend settings.
	#[cfg(feature = "local")]
	#[serde(default)]
	pub local: Option<LocalStorageSettings>,
}

/// Amazon S3 settings.
#[cfg(feature = "s3")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct S3StorageSettings {
	/// S3 bucket name.
	pub bucket: String,
	/// AWS region.
	#[serde(default)]
	pub region: Option<String>,
	/// Custom S3-compatible endpoint.
	#[serde(default)]
	pub endpoint: Option<String>,
	/// Object key prefix.
	#[serde(default)]
	pub prefix: Option<String>,
}

/// Google Cloud Storage settings.
#[cfg(feature = "gcs")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GcsStorageSettings {
	/// GCS bucket name.
	pub bucket: String,
	/// Object name prefix.
	#[serde(default)]
	pub prefix: Option<String>,
	/// Custom endpoint, primarily for fake-gcs-server.
	#[serde(default)]
	pub endpoint: Option<String>,
	/// Service account JSON for explicit credentials and signed URLs.
	#[serde(default)]
	pub service_account_json: Option<SecretString>,
}

/// Azure Blob Storage settings.
#[cfg(feature = "azure")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AzureStorageSettings {
	/// Storage account name.
	pub account: String,
	/// Blob container name.
	pub container: String,
	/// Blob name prefix.
	#[serde(default)]
	pub prefix: Option<String>,
	/// Custom blob endpoint, primarily for Azurite.
	#[serde(default)]
	pub endpoint: Option<String>,
	/// Account access key used for Shared Key and SAS signing.
	#[serde(default)]
	pub access_key: Option<SecretString>,
	/// Pre-generated SAS token.
	#[serde(default)]
	pub sas_token: Option<SecretString>,
	/// Azure Storage connection string.
	#[serde(default)]
	pub connection_string: Option<SecretString>,
}

/// Local filesystem settings.
#[cfg(feature = "local")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalStorageSettings {
	/// Base directory path for stored files.
	pub base_path: String,
}

#[cfg(feature = "local")]
impl Default for LocalStorageSettings {
	fn default() -> Self {
		Self {
			base_path: "media".to_string(),
		}
	}
}

impl Default for StorageSettings {
	fn default() -> Self {
		Self {
			backend: default_backend(),
			#[cfg(feature = "s3")]
			s3: None,
			#[cfg(feature = "gcs")]
			gcs: None,
			#[cfg(feature = "azure")]
			azure: None,
			#[cfg(feature = "local")]
			local: Some(LocalStorageSettings::default()),
		}
	}
}

impl SettingsValidation for StorageSettings {
	fn validate(&self, _profile: &Profile) -> ValidationResult {
		self.to_config().map(|_| ()).map_err(|err| {
			ValidationError::InvalidValue {
				key: "storage.backend".to_string(),
				message: err.to_string(),
			}
		})
	}
}

impl StorageSettings {
	/// Convert settings into the deprecated compatibility config.
	pub fn to_config(&self) -> Result<StorageConfig> {
		match self.backend {
			#[cfg(feature = "s3")]
			BackendType::S3 => self
				.s3
				.as_ref()
				.map(|settings| {
					StorageConfig::S3(crate::config::S3Config {
						bucket: settings.bucket.clone(),
						region: settings.region.clone(),
						endpoint: settings.endpoint.clone(),
						prefix: settings.prefix.clone(),
					})
				})
				.ok_or_else(|| missing_section("storage.s3")),
			#[cfg(feature = "gcs")]
			BackendType::Gcs => self
				.gcs
				.as_ref()
				.map(|settings| {
					StorageConfig::Gcs(crate::config::GcsConfig {
						bucket: settings.bucket.clone(),
						prefix: settings.prefix.clone(),
						endpoint: settings.endpoint.clone(),
						service_account_json: settings.service_account_json.clone(),
					})
				})
				.ok_or_else(|| missing_section("storage.gcs")),
			#[cfg(feature = "azure")]
			BackendType::Azure => self
				.azure
				.as_ref()
				.map(|settings| {
					StorageConfig::Azure(crate::config::AzureConfig {
						account: settings.account.clone(),
						container: settings.container.clone(),
						prefix: settings.prefix.clone(),
						endpoint: settings.endpoint.clone(),
						access_key: settings.access_key.clone(),
						sas_token: settings.sas_token.clone(),
						connection_string: settings.connection_string.clone(),
					})
				})
				.ok_or_else(|| missing_section("storage.azure")),
			#[cfg(feature = "local")]
			BackendType::Local => self
				.local
				.as_ref()
				.map(|settings| {
					StorageConfig::Local(crate::config::LocalConfig {
						base_path: settings.base_path.clone(),
					})
				})
				.ok_or_else(|| missing_section("storage.local")),
			#[allow(unreachable_patterns)]
			backend => Err(StorageError::ConfigError(format!(
				"Backend type not enabled: {backend:?}"
			))),
		}
	}
}

fn missing_section(section: &str) -> StorageError {
	StorageError::ConfigError(format!("Selected backend requires [{section}] settings"))
}
