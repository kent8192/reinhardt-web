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

	let path = Path::new(name);

	if path.is_absolute() || name.starts_with('\\') {
		return Err(StorageError::InvalidPath(format!(
			"absolute paths are not allowed: {name}"
		)));
	}

	// Reject Windows drive-letter absolute paths on all platforms for
	// consistent behavior (on Unix, `Path::is_absolute` misses these).
	if let Some(rest) = name.as_bytes().get(1..) {
		if name.as_bytes()[0].is_ascii_alphabetic()
			&& rest.first() == Some(&b':')
			&& rest.get(1).is_some_and(|&b| b == b'/' || b == b'\\')
		{
			return Err(StorageError::InvalidPath(format!(
				"absolute paths are not allowed: {name}"
			)));
		}
	}

	for component in path.components() {
		match component {
			Component::ParentDir => {
				return Err(StorageError::InvalidPath(format!(
					"parent directory references are not allowed: {name}"
				)));
			}
			Component::RootDir | Component::Prefix(_) => {
				return Err(StorageError::InvalidPath(format!(
					"absolute paths are not allowed: {name}"
				)));
			}
			_ => {}
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
	canonical_base: PathBuf,
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

		let canonical_base = base_path.canonicalize().map_err(|e| {
			StorageError::ConfigError(format!(
				"Failed to canonicalize base path {}: {e}",
				base_path.display()
			))
		})?;

		Ok(Self {
			base_path,
			canonical_base,
		})
	}

	/// Get the full file path after validating it does not escape the storage root.
	fn get_path(&self, name: &str) -> Result<PathBuf> {
		let validated = validate_path(name)?;
		Ok(self.base_path.join(validated))
	}

	/// Verify that a resolved path is contained within the canonical base.
	fn check_containment(&self, canonical_path: &Path) -> Result<()> {
		if !canonical_path.starts_with(&self.canonical_base) {
			return Err(StorageError::InvalidPath(
				"resolved path escapes storage root".to_string(),
			));
		}
		Ok(())
	}
}

#[async_trait]
impl StorageBackend for LocalStorage {
	async fn save(&self, name: &str, content: &[u8]) -> Result<String> {
		let path = self.get_path(name)?;

		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).await?;
			let canonical_parent = parent.canonicalize()?;
			self.check_containment(&canonical_parent)?;
		}

		fs::write(&path, content).await?;

		Ok(name.to_string())
	}

	async fn open(&self, name: &str) -> Result<Vec<u8>> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let canonical = path.canonicalize()?;
		self.check_containment(&canonical)?;

		let content = fs::read(&canonical).await?;
		Ok(content)
	}

	async fn delete(&self, name: &str) -> Result<()> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let canonical = path.canonicalize()?;
		self.check_containment(&canonical)?;

		fs::remove_file(&canonical).await?;
		Ok(())
	}

	async fn exists(&self, name: &str) -> Result<bool> {
		let path = self.get_path(name)?;

		if !path.exists() || !path.is_file() {
			return Ok(false);
		}

		let canonical = path.canonicalize()?;
		self.check_containment(&canonical)?;

		Ok(true)
	}

	async fn url(&self, name: &str, _expiry_secs: u64) -> Result<String> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let canonical = path.canonicalize()?;
		self.check_containment(&canonical)?;

		Ok(format!("file://{}", canonical.display()))
	}

	async fn size(&self, name: &str) -> Result<u64> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let canonical = path.canonicalize()?;
		self.check_containment(&canonical)?;

		let metadata = fs::metadata(&canonical).await?;
		Ok(metadata.len())
	}

	async fn get_modified_time(&self, name: &str) -> Result<DateTime<Utc>> {
		let path = self.get_path(name)?;

		if !path.exists() {
			return Err(StorageError::NotFound(name.to_string()));
		}

		let canonical = path.canonicalize()?;
		self.check_containment(&canonical)?;

		let metadata = fs::metadata(&canonical).await?;
		let modified = metadata.modified()?;

		let datetime: DateTime<Utc> = modified.into();
		Ok(datetime)
	}
}
