//! Storage backend trait

use super::errors::StorageResult;
use super::file::{FileMetadata, StoredFile};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Trait for file storage backends
#[async_trait]
pub trait Storage: Send + Sync {
	/// Save a file to storage
	async fn save(&self, path: &str, content: &[u8]) -> StorageResult<FileMetadata>;

	/// Read a file from storage
	async fn read(&self, path: &str) -> StorageResult<StoredFile>;

	/// Delete a file from storage
	async fn delete(&self, path: &str) -> StorageResult<()>;

	/// Check if a file exists
	async fn exists(&self, path: &str) -> StorageResult<bool>;

	/// Get file metadata
	async fn metadata(&self, path: &str) -> StorageResult<FileMetadata>;

	/// List files in a directory
	async fn list(&self, path: &str) -> StorageResult<Vec<FileMetadata>>;

	/// Get the URL for accessing a file
	fn url(&self, path: &str) -> String;

	/// Get the full path for a file
	fn path(&self, name: &str) -> String;

	/// Get file accessed time
	async fn get_accessed_time(&self, path: &str) -> StorageResult<DateTime<Utc>> {
		Ok(self.metadata(path).await?.modified_at)
	}

	/// Get file created time
	async fn get_created_time(&self, path: &str) -> StorageResult<DateTime<Utc>> {
		Ok(self.metadata(path).await?.created_at)
	}

	/// Get file modified time
	async fn get_modified_time(&self, path: &str) -> StorageResult<DateTime<Utc>> {
		Ok(self.metadata(path).await?.modified_at)
	}
}
