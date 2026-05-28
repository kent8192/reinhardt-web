//! Google Cloud Storage backend implementation.

#![allow(deprecated)] // Backend constructor keeps accepting legacy config during compatibility.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use google_cloud_storage::client::{Storage, StorageControl};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;

use crate::config::GcsConfig;
use crate::{Result, StorageBackend, StorageError};

#[derive(Debug, Deserialize)]
struct GcsMetadata {
	size: Option<serde_json::Value>,
	updated: Option<String>,
}

/// Google Cloud Storage backend.
#[derive(Debug, Clone)]
pub struct GcsStorage {
	config: GcsConfig,
	storage: Option<Storage>,
	control: Option<StorageControl>,
	http: reqwest::Client,
}

impl GcsStorage {
	/// Create a new GCS storage backend.
	pub async fn new(config: GcsConfig) -> Result<Self> {
		let (storage, control) = if config.endpoint.is_some() {
			(None, None)
		} else {
			let storage = Storage::builder()
				.build()
				.await
				.map_err(|err| StorageError::ConfigError(err.to_string()))?;
			let control = StorageControl::builder()
				.build()
				.await
				.map_err(|err| StorageError::ConfigError(err.to_string()))?;
			(Some(storage), Some(control))
		};

		Ok(Self {
			config,
			storage,
			control,
			http: reqwest::Client::new(),
		})
	}

	fn object_name(&self, name: &str) -> String {
		if let Some(prefix) = &self.config.prefix {
			if prefix.is_empty() {
				name.to_string()
			} else {
				format!("{}/{}", prefix.trim_end_matches('/'), name)
			}
		} else {
			name.to_string()
		}
	}

	fn bucket_resource(&self) -> String {
		format!("projects/_/buckets/{}", self.config.bucket)
	}

	fn encoded_object(object: &str) -> String {
		utf8_percent_encode(object, NON_ALPHANUMERIC).to_string()
	}

	fn endpoint(&self) -> Option<&str> {
		self.config.endpoint.as_deref()
	}

	fn metadata_url(&self, object: &str) -> Result<String> {
		let endpoint = self.endpoint().ok_or_else(|| {
			StorageError::ConfigError("GCS endpoint is not configured".to_string())
		})?;
		Ok(format!(
			"{}/storage/v1/b/{}/o/{}",
			endpoint.trim_end_matches('/'),
			self.config.bucket,
			Self::encoded_object(object)
		))
	}

	fn media_url(&self, object: &str) -> Result<String> {
		Ok(format!("{}?alt=media", self.metadata_url(object)?))
	}

	fn upload_url(&self, object: &str) -> Result<String> {
		let endpoint = self.endpoint().ok_or_else(|| {
			StorageError::ConfigError("GCS endpoint is not configured".to_string())
		})?;
		Ok(format!(
			"{}/upload/storage/v1/b/{}/o?uploadType=media&name={}",
			endpoint.trim_end_matches('/'),
			self.config.bucket,
			Self::encoded_object(object)
		))
	}

	fn map_status(status: reqwest::StatusCode, name: &str) -> StorageError {
		if status == reqwest::StatusCode::NOT_FOUND {
			StorageError::NotFound(format!("GCS object not found: {name}"))
		} else if status == reqwest::StatusCode::FORBIDDEN {
			StorageError::PermissionDenied(format!("GCS permission denied for object: {name}"))
		} else if status.is_server_error() {
			StorageError::NetworkError(format!("GCS service error {status} for object: {name}"))
		} else {
			StorageError::Other(format!("GCS request failed with status {status} for object: {name}"))
		}
	}

	fn map_sdk_error(err: google_cloud_storage::Error, name: &str) -> StorageError {
		let message = err.to_string();
		if message.contains("404") || message.to_ascii_lowercase().contains("not found") {
			StorageError::NotFound(format!("GCS object not found: {name}"))
		} else if message.contains("403") || message.to_ascii_lowercase().contains("permission") {
			StorageError::PermissionDenied(message)
		} else {
			StorageError::NetworkError(message)
		}
	}

	async fn endpoint_metadata(&self, object: &str) -> Result<GcsMetadata> {
		let url = self.metadata_url(object)?;
		let response = self
			.http
			.get(url)
			.send()
			.await
			.map_err(|err| StorageError::NetworkError(err.to_string()))?;
		let status = response.status();
		if !status.is_success() {
			return Err(Self::map_status(status, object));
		}
		response
			.json::<GcsMetadata>()
			.await
			.map_err(|err| StorageError::Other(err.to_string()))
	}

	async fn sdk_metadata(&self, object: &str) -> Result<google_cloud_storage::model::Object> {
		let control = self.control.as_ref().ok_or_else(|| {
			StorageError::ConfigError("GCS control client is not configured".to_string())
		})?;
		control
			.get_object()
			.set_bucket(self.bucket_resource())
			.set_object(object)
			.send()
			.await
			.map_err(|err| Self::map_sdk_error(err, object))
	}

	fn metadata_size(metadata: &GcsMetadata) -> Result<u64> {
		match metadata.size.as_ref() {
			Some(serde_json::Value::String(size)) => size
				.parse::<u64>()
				.map_err(|err| StorageError::Other(err.to_string())),
			Some(serde_json::Value::Number(size)) => size
				.as_u64()
				.ok_or_else(|| StorageError::Other("GCS object size is not unsigned".to_string())),
			_ => Err(StorageError::Other(
				"GCS object metadata did not include size".to_string(),
			)),
		}
	}

	fn metadata_updated(metadata: &GcsMetadata) -> Result<DateTime<Utc>> {
		let updated = metadata.updated.as_ref().ok_or_else(|| {
			StorageError::Other("GCS object metadata did not include updated time".to_string())
		})?;
		DateTime::parse_from_rfc3339(updated)
			.map(|time| time.with_timezone(&Utc))
			.map_err(|err| StorageError::Other(err.to_string()))
	}
}

#[async_trait]
impl StorageBackend for GcsStorage {
	async fn save(&self, name: &str, content: &[u8]) -> Result<String> {
		let object = self.object_name(name);

		if self.endpoint().is_some() {
			let url = self.upload_url(&object)?;
			let response = self
				.http
				.post(url)
				.header("content-type", "application/octet-stream")
				.body(content.to_vec())
				.send()
				.await
				.map_err(|err| StorageError::NetworkError(err.to_string()))?;
			let status = response.status();
			if !status.is_success() {
				return Err(Self::map_status(status, &object));
			}
			return Ok(object);
		}

		let storage = self.storage.as_ref().ok_or_else(|| {
			StorageError::ConfigError("GCS storage client is not configured".to_string())
		})?;
		storage
			.write_object(
				self.bucket_resource(),
				object.clone(),
				bytes::Bytes::copy_from_slice(content),
			)
			.send_buffered()
			.await
			.map_err(|err| Self::map_sdk_error(err, &object))?;
		Ok(object)
	}

	async fn open(&self, name: &str) -> Result<Vec<u8>> {
		let object = self.object_name(name);

		if self.endpoint().is_some() {
			let url = self.media_url(&object)?;
			let response = self
				.http
				.get(url)
				.send()
				.await
				.map_err(|err| StorageError::NetworkError(err.to_string()))?;
			let status = response.status();
			if !status.is_success() {
				return Err(Self::map_status(status, &object));
			}
			return response
				.bytes()
				.await
				.map(|bytes| bytes.to_vec())
				.map_err(|err| StorageError::NetworkError(err.to_string()));
		}

		let storage = self.storage.as_ref().ok_or_else(|| {
			StorageError::ConfigError("GCS storage client is not configured".to_string())
		})?;
		let mut response = storage
			.read_object(self.bucket_resource(), object.clone())
			.send()
			.await
			.map_err(|err| Self::map_sdk_error(err, &object))?;
		let mut content = Vec::new();
		while let Some(chunk) = response.next().await {
			content.extend_from_slice(&chunk.map_err(|err| Self::map_sdk_error(err, &object))?);
		}
		Ok(content)
	}

	async fn delete(&self, name: &str) -> Result<()> {
		let object = self.object_name(name);
		if !self.exists(name).await? {
			return Err(StorageError::NotFound(format!(
				"GCS object not found: {object}"
			)));
		}

		if self.endpoint().is_some() {
			let url = self.metadata_url(&object)?;
			let response = self
				.http
				.delete(url)
				.send()
				.await
				.map_err(|err| StorageError::NetworkError(err.to_string()))?;
			let status = response.status();
			if !status.is_success() {
				return Err(Self::map_status(status, &object));
			}
			return Ok(());
		}

		let control = self.control.as_ref().ok_or_else(|| {
			StorageError::ConfigError("GCS control client is not configured".to_string())
		})?;
		control
			.delete_object()
			.set_bucket(self.bucket_resource())
			.set_object(object.clone())
			.send()
			.await
			.map_err(|err| Self::map_sdk_error(err, &object))
	}

	async fn exists(&self, name: &str) -> Result<bool> {
		let object = self.object_name(name);

		let result = if self.endpoint().is_some() {
			self.endpoint_metadata(&object).await.map(|_| ())
		} else {
			self.sdk_metadata(&object).await.map(|_| ())
		};

		match result {
			Ok(()) => Ok(true),
			Err(StorageError::NotFound(_)) => Ok(false),
			Err(err) => Err(err),
		}
	}

	async fn url(&self, name: &str, expiry_secs: u64) -> Result<String> {
		let object = self.object_name(name);
		if !self.exists(name).await? {
			return Err(StorageError::NotFound(format!(
				"GCS object not found: {object}"
			)));
		}

		if let Some(endpoint) = self.endpoint() {
			return Ok(format!(
				"{}/storage/v1/b/{}/o/{}?alt=media&X-Goog-Expires={}",
				endpoint.trim_end_matches('/'),
				self.config.bucket,
				Self::encoded_object(&object),
				expiry_secs
			));
		}

		Err(StorageError::ConfigError(
			"GCS signed URL generation requires explicit signing credentials".to_string(),
		))
	}

	async fn size(&self, name: &str) -> Result<u64> {
		let object = self.object_name(name);
		if self.endpoint().is_some() {
			let metadata = self.endpoint_metadata(&object).await?;
			return Self::metadata_size(&metadata);
		}
		let metadata = self.sdk_metadata(&object).await?;
		u64::try_from(metadata.size)
			.map_err(|err| StorageError::Other(format!("Invalid GCS object size: {err}")))
	}

	async fn get_modified_time(&self, name: &str) -> Result<DateTime<Utc>> {
		let object = self.object_name(name);
		if self.endpoint().is_some() {
			let metadata = self.endpoint_metadata(&object).await?;
			return Self::metadata_updated(&metadata);
		}
		let metadata = self.sdk_metadata(&object).await?;
		let timestamp = metadata.finalize_time.or(metadata.create_time).ok_or_else(|| {
			StorageError::Other("GCS object metadata did not include timestamp".to_string())
		})?;
		let nanos = u32::try_from(timestamp.nanos()).map_err(|err| {
			StorageError::Other(format!("Invalid GCS timestamp nanoseconds: {err}"))
		})?;
		DateTime::from_timestamp(timestamp.seconds(), nanos).ok_or_else(|| {
			StorageError::Other("Invalid GCS object metadata timestamp".to_string())
		})
	}
}
