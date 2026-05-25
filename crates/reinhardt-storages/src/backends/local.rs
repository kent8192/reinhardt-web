//! Local file system storage backend implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::{Component, Path, PathBuf};
use tokio::fs;

use crate::config::LocalConfig;
use crate::{Result, StorageBackend, StorageError};

/// Validate that the given name does not escape the storage root.
///
/// Rejects empty strings, absolute paths, parent directory references (`..`),
/// and degenerate directory-only references (`.` and `..`).
fn validate_path(name: &str) -> Result<&str> {
	if name.is_empty() {
		return Err(StorageError::InvalidPath(
			"path must not be empty".to_string(),
		));
	}

	if name.starts_with('/') || name.starts_with('\\') {
		return Err(StorageError::InvalidPath(format!(
			"absolute paths are not allowed: {name}"
		)));
	}

	for component in Path::new(name).components() {
		if component == Component::ParentDir {
			return Err(StorageError::InvalidPath(format!(
				"parent directory references are not allowed: {name}"
			)));
		}
	}

	if name == "." || name == ".." {
		return Err(StorageError::InvalidPath(format!(
			"path must refer to a file, not a directory reference: {name}"
		)));
	}

	Ok(name)
}

/// Local file system storage backend.
#[derive(Debug, Clone)]
pub struct LocalStorage {
	base_path: PathBuf,
}

impl LocalStorage {
	/// Create a new local storage backend.
	///
	/// # Arguments
	///
	/// * `config` - Local storage configuration
	///
	/// # Errors
	///
	/// Returns `` `StorageError::ConfigError` `` if the base path is invalid.
	pub fn new(config: LocalConfig) -> Result<Self> {
		let base_path = PathBuf::from(config.base_path);

		if !base_path.exists() {
			return Err(StorageError::ConfigError(format!(
				"Base path does not exist: {}",
				base_path.display()
			)));
		}

		if !base_path.is_dir() {
			return Err(StorageError::ConfigError(format!(
				"Base path is not a directory: {}",
				base_path.display()
			)));
		}

		Ok(Self { base_path })
	}

	/// Get the full file path after validating it does not escape the storage root.
	fn get_path(&self, name: &str) -> Result<PathBuf> {
		let validated = validate_path(name)?;
		Ok(self.base_path.join(validated))
	}
}

#[async_trait]
impl StorageBackend for LocalStorage {
	async fn save(&self, name: &str, content: &[u8]) -> Result<String> {
		let path = self.get_path(name)?;

		// Create parent directories if they don't exist
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).await?;
		}

		fs::write(&path, content).await?;

		Ok(name.to_string())
	}

	async fn open(&self, name: &str) -> Result<Vec<u8>> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let content = fs::read(&path).await?;
		Ok(content)
	}

	async fn delete(&self, name: &str) -> Result<()> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		fs::remove_file(&path).await?;
		Ok(())
	}

	async fn exists(&self, name: &str) -> Result<bool> {
		let path = self.get_path(name)?;
		Ok(path.exists() && path.is_file())
	}

	async fn url(&self, name: &str, _expiry_secs: u64) -> Result<String> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		// Convert to absolute path
		let abs_path = path.canonicalize()?;

		// Return as file:// URL
		Ok(format!("file://{}", abs_path.display()))
	}

	async fn size(&self, name: &str) -> Result<u64> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let metadata = fs::metadata(&path).await?;
		Ok(metadata.len())
	}

	async fn get_modified_time(&self, name: &str) -> Result<DateTime<Utc>> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let metadata = fs::metadata(&path).await?;
		let modified = metadata.modified()?;

		let datetime: DateTime<Utc> = modified.into();
		Ok(datetime)
	}
}
