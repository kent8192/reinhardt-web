//! Compatibility configuration types for storage backends.

#![allow(deprecated)] // This module defines and populates legacy compatibility types.

use crate::{Result, StorageError};
#[cfg(any(feature = "azure", feature = "gcs"))]
use reinhardt_conf::settings::secret_types::SecretString;
use serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;

/// Storage backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
	/// Amazon S3 storage.
	S3,
	/// Google Cloud Storage.
	Gcs,
	/// Azure Blob Storage.
	Azure,
	/// Local file system.
	Local,
}

impl std::fmt::Display for BackendType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BackendType::S3 => write!(f, "S3"),
			BackendType::Gcs => write!(f, "GCS"),
			BackendType::Azure => write!(f, "Azure"),
			BackendType::Local => write!(f, "Local"),
		}
	}
}

impl FromStr for BackendType {
	type Err = StorageError;

	fn from_str(s: &str) -> Result<Self> {
		match s.to_lowercase().as_str() {
			"s3" => Ok(BackendType::S3),
			"gcs" => Ok(BackendType::Gcs),
			"azure" => Ok(BackendType::Azure),
			"local" => Ok(BackendType::Local),
			_ => Err(StorageError::ConfigError(format!(
				"Invalid backend type: {}",
				s
			))),
		}
	}
}

/// Configuration for S3 storage backend.
#[cfg(feature = "s3")]
#[deprecated(
	since = "0.2.0",
	note = "Use `StorageSettings` with the `#[settings]` macro instead."
)]
#[derive(Debug, Clone)]
pub struct S3Config {
	/// S3 bucket name.
	pub bucket: String,
	/// AWS region (for example, "us-east-1").
	pub region: Option<String>,
	/// Custom endpoint URL for S3-compatible services.
	pub endpoint: Option<String>,
	/// Path prefix for all files.
	pub prefix: Option<String>,
}

/// Configuration for Google Cloud Storage backend.
#[cfg(feature = "gcs")]
#[deprecated(
	since = "0.2.0",
	note = "Use `StorageSettings` with the `#[settings]` macro instead."
)]
#[derive(Debug, Clone)]
pub struct GcsConfig {
	/// GCS bucket name.
	pub bucket: String,
	/// Path prefix for all files.
	pub prefix: Option<String>,
	/// Custom endpoint URL for emulators.
	pub endpoint: Option<String>,
	/// Service account JSON used for signed URLs and explicit credentials.
	pub service_account_json: Option<SecretString>,
}

/// Configuration for Azure Blob Storage backend.
#[cfg(feature = "azure")]
#[deprecated(
	since = "0.2.0",
	note = "Use `StorageSettings` with the `#[settings]` macro instead."
)]
#[derive(Debug, Clone)]
pub struct AzureConfig {
	/// Storage account name.
	pub account: String,
	/// Container name.
	pub container: String,
	/// Path prefix for all blobs.
	pub prefix: Option<String>,
	/// Custom endpoint URL for emulators or sovereign clouds.
	pub endpoint: Option<String>,
	/// Account access key used for Shared Key and SAS signing.
	pub access_key: Option<SecretString>,
	/// Pre-generated SAS token used only for backend operations.
	pub sas_token: Option<SecretString>,
	/// Azure Storage connection string.
	pub connection_string: Option<SecretString>,
}

/// Configuration for local file system backend.
#[cfg(feature = "local")]
#[deprecated(
	since = "0.2.0",
	note = "Use `StorageSettings` with the `#[settings]` macro instead."
)]
#[derive(Debug, Clone)]
pub struct LocalConfig {
	/// Base directory path for file storage.
	pub base_path: String,
}

/// Compatibility storage configuration.
#[deprecated(
	since = "0.2.0",
	note = "Use `StorageSettings` with the `#[settings]` macro instead."
)]
#[derive(Debug, Clone)]
pub enum StorageConfig {
	/// Amazon S3 storage.
	#[cfg(feature = "s3")]
	S3(S3Config),
	/// Google Cloud Storage.
	#[cfg(feature = "gcs")]
	Gcs(GcsConfig),
	/// Azure Blob Storage.
	#[cfg(feature = "azure")]
	Azure(AzureConfig),
	/// Local file system.
	#[cfg(feature = "local")]
	Local(LocalConfig),
}

impl StorageConfig {
	/// Load compatibility configuration from environment variables.
	///
	/// Prefer `StorageSettings` composed through the `#[settings]` macro for new
	/// applications.
	///
	/// # Environment Variables
	///
	/// - `STORAGE_BACKEND`: Backend type ("s3", "gcs", "azure", "local")
	///
	/// ## S3 Backend
	/// - `S3_BUCKET`: Bucket name (required)
	/// - `S3_REGION`: AWS region (optional)
	/// - `S3_ENDPOINT`: Custom endpoint URL (optional)
	/// - `S3_PREFIX`: Path prefix (optional)
	///
	/// ## GCS Backend
	/// - `GCS_BUCKET`: Bucket name (required)
	/// - `GCS_PREFIX`: Path prefix (optional)
	/// - `GCS_ENDPOINT`: Custom endpoint URL (optional)
	/// - `GCS_SERVICE_ACCOUNT_JSON`: Service account JSON (optional)
	///
	/// ## Azure Backend
	/// - `AZURE_ACCOUNT`: Storage account name (required)
	/// - `AZURE_CONTAINER`: Container name (required)
	/// - `AZURE_PREFIX`: Path prefix (optional)
	/// - `AZURE_ENDPOINT`: Custom endpoint URL (optional)
	/// - `AZURE_ACCESS_KEY`: Storage account key (optional)
	/// - `AZURE_SAS_TOKEN`: Pre-generated SAS token (optional)
	/// - `AZURE_CONNECTION_STRING`: Storage connection string (optional)
	///
	/// ## Local Backend
	/// - `LOCAL_BASE_PATH`: Base directory path (required)
	#[deprecated(
		since = "0.2.0",
		note = "Use `StorageSettings` with the `#[settings]` macro instead."
	)]
	pub fn from_env() -> Result<Self> {
		let backend_type = env::var("STORAGE_BACKEND").map_err(|_| {
			StorageError::ConfigError("STORAGE_BACKEND environment variable not set".to_string())
		})?;

		let backend_type = backend_type.parse::<BackendType>()?;

		match backend_type {
			#[cfg(feature = "s3")]
			BackendType::S3 => {
				let bucket = env::var("S3_BUCKET").map_err(|_| {
					StorageError::ConfigError("S3_BUCKET environment variable not set".to_string())
				})?;
				let region = env::var("S3_REGION").ok();
				let endpoint = env::var("S3_ENDPOINT").ok();
				let prefix = env::var("S3_PREFIX").ok();

				Ok(StorageConfig::S3(S3Config {
					bucket,
					region,
					endpoint,
					prefix,
				}))
			}
			#[cfg(feature = "gcs")]
			BackendType::Gcs => {
				let bucket = env::var("GCS_BUCKET").map_err(|_| {
					StorageError::ConfigError("GCS_BUCKET environment variable not set".to_string())
				})?;
				let prefix = env::var("GCS_PREFIX").ok();
				let endpoint = env::var("GCS_ENDPOINT").ok();
				let service_account_json = env::var("GCS_SERVICE_ACCOUNT_JSON")
					.ok()
					.map(SecretString::new);

				Ok(StorageConfig::Gcs(GcsConfig {
					bucket,
					prefix,
					endpoint,
					service_account_json,
				}))
			}
			#[cfg(feature = "azure")]
			BackendType::Azure => {
				let account = env::var("AZURE_ACCOUNT").map_err(|_| {
					StorageError::ConfigError(
						"AZURE_ACCOUNT environment variable not set".to_string(),
					)
				})?;
				let container = env::var("AZURE_CONTAINER").map_err(|_| {
					StorageError::ConfigError(
						"AZURE_CONTAINER environment variable not set".to_string(),
					)
				})?;
				let prefix = env::var("AZURE_PREFIX").ok();
				let endpoint = env::var("AZURE_ENDPOINT").ok();
				let access_key = env::var("AZURE_ACCESS_KEY").ok().map(SecretString::new);
				let sas_token = env::var("AZURE_SAS_TOKEN").ok().map(SecretString::new);
				let connection_string = env::var("AZURE_CONNECTION_STRING")
					.ok()
					.map(SecretString::new);

				Ok(StorageConfig::Azure(AzureConfig {
					account,
					container,
					prefix,
					endpoint,
					access_key,
					sas_token,
					connection_string,
				}))
			}
			#[cfg(feature = "local")]
			BackendType::Local => {
				let base_path = env::var("LOCAL_BASE_PATH").map_err(|_| {
					StorageError::ConfigError(
						"LOCAL_BASE_PATH environment variable not set".to_string(),
					)
				})?;

				Ok(StorageConfig::Local(LocalConfig { base_path }))
			}
			#[allow(unreachable_patterns)]
			_ => Err(StorageError::ConfigError(format!(
				"Backend type not enabled: {:?}",
				backend_type
			))),
		}
	}
}
