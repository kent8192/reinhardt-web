//! Storage backend registry
//!
//! Provides a registry system for custom storage backends

use crate::storage::Storage;
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, RwLock};

/// Storage backend factory function type
pub type StorageFactory = Box<dyn Fn() -> Arc<dyn Storage> + Send + Sync>;

/// Storage backend registry
///
/// Allows registration and retrieval of custom storage backends
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_static::storage::{StorageRegistry, FileSystemStorage};
/// use std::sync::Arc;
/// use std::path::PathBuf;
///
/// let mut registry = StorageRegistry::new();
///
/// // Register a custom storage backend
/// registry.register(
///     "custom",
///     Box::new(|| {
///         Arc::new(FileSystemStorage::new(
///             PathBuf::from("/tmp/static"),
///             "/static/"
///         ))
///     })
/// );
///
/// // Get the storage backend
/// let storage = registry.get("custom").unwrap();
/// ```
pub struct StorageRegistry {
	backends: Arc<RwLock<HashMap<String, StorageFactory>>>,
}

impl StorageRegistry {
	/// Create a new empty storage registry
	pub fn new() -> Self {
		Self {
			backends: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Register a storage backend
	///
	/// # Arguments
	///
	/// * `name` - Unique name for the storage backend
	/// * `factory` - Factory function that creates the storage backend
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// registry.register(
	///     "my-storage",
	///     Box::new(|| Arc::new(MyCustomStorage::new()))
	/// );
	/// ```
	pub fn register(&mut self, name: &str, factory: StorageFactory) -> io::Result<()> {
		let mut backends = self
			.backends
			.write()
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

		if backends.contains_key(name) {
			return Err(io::Error::new(
				io::ErrorKind::AlreadyExists,
				format!("Storage backend '{}' is already registered", name),
			));
		}

		backends.insert(name.to_string(), factory);
		Ok(())
	}

	/// Unregister a storage backend
	///
	/// # Arguments
	///
	/// * `name` - Name of the storage backend to unregister
	pub fn unregister(&mut self, name: &str) -> io::Result<()> {
		let mut backends = self
			.backends
			.write()
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

		if backends.remove(name).is_none() {
			return Err(io::Error::new(
				io::ErrorKind::NotFound,
				format!("Storage backend '{}' not found", name),
			));
		}

		Ok(())
	}

	/// Get a storage backend by name
	///
	/// # Arguments
	///
	/// * `name` - Name of the storage backend
	///
	/// # Returns
	///
	/// Returns `Some(Arc<dyn Storage>)` if found, `None` otherwise
	pub fn get(&self, name: &str) -> Option<Arc<dyn Storage>> {
		let backends = self.backends.read().ok()?;
		let factory = backends.get(name)?;
		Some(factory())
	}

	/// Check if a storage backend is registered
	///
	/// # Arguments
	///
	/// * `name` - Name of the storage backend
	pub fn contains(&self, name: &str) -> bool {
		self.backends
			.read()
			.map(|backends| backends.contains_key(name))
			.unwrap_or(false)
	}

	/// Get list of registered storage backend names
	pub fn list(&self) -> Vec<String> {
		self.backends
			.read()
			.map(|backends| backends.keys().cloned().collect())
			.unwrap_or_default()
	}

	/// Clear all registered storage backends
	pub fn clear(&mut self) -> io::Result<()> {
		let mut backends = self
			.backends
			.write()
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

		backends.clear();
		Ok(())
	}
}

impl Default for StorageRegistry {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::storage::{FileSystemStorage, MemoryStorage};
	use std::path::PathBuf;

	#[test]
	fn test_registry_creation() {
		let registry = StorageRegistry::new();
		assert_eq!(registry.list().len(), 0);
	}

	#[test]
	fn test_register_storage_backend() {
		let mut registry = StorageRegistry::new();

		let result = registry.register(
			"test",
			Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
		);

		assert!(result.is_ok());
		assert!(registry.contains("test"));
	}

	#[test]
	fn test_register_duplicate_fails() {
		let mut registry = StorageRegistry::new();

		registry
			.register(
				"test",
				Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
			)
			.unwrap();

		let result = registry.register(
			"test",
			Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
		);

		assert!(result.is_err());
		assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
	}

	#[test]
	fn test_get_storage_backend() {
		let mut registry = StorageRegistry::new();

		registry
			.register(
				"memory",
				Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
			)
			.unwrap();

		let storage = registry.get("memory");
		assert!(storage.is_some());

		// Verify it's the correct type by checking URL generation
		let url = storage.unwrap().url("test.txt");
		assert_eq!(url, "/static/test.txt");
	}

	#[test]
	fn test_get_nonexistent_backend() {
		let registry = StorageRegistry::new();
		let storage = registry.get("nonexistent");
		assert!(storage.is_none());
	}

	#[test]
	fn test_unregister_storage_backend() {
		let mut registry = StorageRegistry::new();

		registry
			.register(
				"test",
				Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
			)
			.unwrap();

		assert!(registry.contains("test"));

		let result = registry.unregister("test");
		assert!(result.is_ok());
		assert!(!registry.contains("test"));
	}

	#[test]
	fn test_unregister_nonexistent_fails() {
		let mut registry = StorageRegistry::new();
		let result = registry.unregister("nonexistent");

		assert!(result.is_err());
		assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
	}

	#[test]
	fn test_list_storage_backends() {
		let mut registry = StorageRegistry::new();

		registry
			.register(
				"memory",
				Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
			)
			.unwrap();

		registry
			.register(
				"filesystem",
				Box::new(|| {
					Arc::new(FileSystemStorage::new(
						PathBuf::from("/tmp/static"),
						"/static/",
					))
				}),
			)
			.unwrap();

		let backends = registry.list();
		assert_eq!(backends.len(), 2);
		assert!(backends.contains(&"memory".to_string()));
		assert!(backends.contains(&"filesystem".to_string()));
	}

	#[test]
	fn test_clear_registry() {
		let mut registry = StorageRegistry::new();

		registry
			.register(
				"test1",
				Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
			)
			.unwrap();

		registry
			.register(
				"test2",
				Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
			)
			.unwrap();

		assert_eq!(registry.list().len(), 2);

		let result = registry.clear();
		assert!(result.is_ok());
		assert_eq!(registry.list().len(), 0);
	}

	#[test]
	fn test_multiple_gets_create_different_instances() {
		let mut registry = StorageRegistry::new();

		registry
			.register(
				"test",
				Box::new(|| Arc::new(MemoryStorage::new("/static/"))),
			)
			.unwrap();

		let storage1 = registry.get("test").unwrap();
		let storage2 = registry.get("test").unwrap();

		// Each call to get() should create a new instance
		// We can verify this by checking that the Arc pointers are different
		assert_ne!(Arc::as_ptr(&storage1), Arc::as_ptr(&storage2));
	}
}
