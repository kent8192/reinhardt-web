//! Local storage backend test fixtures.

use reinhardt_storages::{StorageBackend, StorageConfig};
use rstest::fixture;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Test directory wrapper for local storage.
pub struct LocalTestDir {
	temp_dir: TempDir,
	backend: Arc<dyn StorageBackend>,
}

impl LocalTestDir {
	/// Create a new test directory with storage backend.
	pub fn new() -> Self {
		let temp_dir = TempDir::new().expect("Failed to create temp dir");
		let base_path = temp_dir.path().to_str().unwrap().to_string();

		let config = StorageConfig::Local(reinhardt_storages::config::LocalConfig { base_path });

		let backend = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(async { reinhardt_storages::create_storage(config).await })
			.expect("Failed to create local backend");

		Self { temp_dir, backend }
	}

	/// Get the storage backend.
	pub fn backend(&self) -> Arc<dyn StorageBackend> {
		Arc::clone(&self.backend)
	}

	/// Get the temp directory path.
	pub fn path(&self) -> &Path {
		self.temp_dir.path()
	}
}

/// Local storage backend fixture.
#[fixture]
pub fn local_backend() -> Arc<dyn StorageBackend> {
	let temp_dir = TempDir::new().expect("Failed to create temp dir");
	let base_path = temp_dir.path().to_str().unwrap().to_string();

	let config = StorageConfig::Local(reinhardt_storages::config::LocalConfig { base_path });

	tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(async { reinhardt_storages::create_storage(config).await })
		.expect("Failed to create local backend")
}

/// Local test directory fixture.
#[fixture]
pub fn local_temp_dir() -> LocalTestDir {
	LocalTestDir::new()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_local_test_dir_creation() {
		let test_dir = LocalTestDir::new();
		assert!(test_dir.path().exists());
		assert!(test_dir.path().is_dir());
	}

	#[test]
	fn test_local_backend_fixture() {
		let backend = local_backend();
		// Just verify it creates successfully
		drop(backend);
	}

	#[tokio::test]
	async fn test_local_backend_save_and_read() {
		let backend = local_backend();

		backend
			.save("test.txt", b"Hello, world!")
			.await
			.expect("Failed to save");

		let content = backend.open("test.txt").await.expect("Failed to read");

		assert_eq!(content, b"Hello, world!");
	}
}
