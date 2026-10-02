//! Type-safe extensions for Request
//!
//! Provides a simple type-safe storage mechanism for arbitrary data
//! that can be attached to requests.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

type ExtensionValue = Box<dyn Any + Send + Sync>;
type ExtensionMap = HashMap<TypeId, ExtensionValue>;
type SharedExtensionMap = Arc<Mutex<ExtensionMap>>;

/// Whether the current user is authenticated.
/// Newtype wrapper to avoid `TypeId` collision with other bool values in extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsAuthenticated(pub bool);

/// Whether the current user has admin privileges (staff or superuser).
/// Newtype wrapper to avoid `TypeId` collision with other bool values in extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsAdmin(pub bool);

/// Whether the current user account is active.
/// Newtype wrapper to avoid `TypeId` collision with other bool values in extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsActive(pub bool);

/// Type-safe extension storage
///
/// # Clone semantics
///
/// `Extensions` lazily initializes an `Arc<Mutex<HashMap>>` backing store.
/// Cloning an `Extensions` creates a **shared** reference to the same backing
/// store — it does NOT deep-copy the stored values. Mutations through one
/// clone are visible through all other clones.
#[derive(Default)]
pub struct Extensions {
	map: OnceLock<SharedExtensionMap>,
}

impl Clone for Extensions {
	fn clone(&self) -> Self {
		let map = self.shared_map();
		let cloned = Self::new();
		let _ = cloned.map.set(Arc::clone(map));
		cloned
	}
}

impl Extensions {
	/// Create a new Extensions instance
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Extensions;
	///
	/// let extensions = Extensions::new();
	/// assert!(!extensions.contains::<String>());
	/// ```
	pub fn new() -> Self {
		Self {
			map: OnceLock::new(),
		}
	}

	fn shared_map(&self) -> &SharedExtensionMap {
		self.map
			.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
	}

	/// Clone only the initialized backing store.
	pub(crate) fn clone_if_initialized(&self) -> Self {
		let cloned = Self::new();
		if let Some(map) = self.map.get() {
			let _ = cloned.map.set(Arc::clone(map));
		}
		cloned
	}

	/// Insert a value into extensions
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Extensions;
	///
	/// let extensions = Extensions::new();
	/// extensions.insert(42u32);
	/// extensions.insert("hello".to_string());
	///
	/// assert!(extensions.contains::<u32>());
	/// assert!(extensions.contains::<String>());
	/// ```
	pub fn insert<T: Send + Sync + 'static>(&self, value: T) {
		let mut map = self.shared_map().lock().unwrap_or_else(|e| e.into_inner());
		map.insert(TypeId::of::<T>(), Box::new(value));
	}
	/// Get a cloned value from extensions
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Extensions;
	///
	/// let extensions = Extensions::new();
	/// extensions.insert(42u32);
	///
	/// assert_eq!(extensions.get::<u32>(), Some(42));
	/// assert_eq!(extensions.get::<String>(), None);
	/// ```
	pub fn get<T>(&self) -> Option<T>
	where
		T: Clone + Send + Sync + 'static,
	{
		let map = self.map.get()?;
		let map = map.lock().unwrap_or_else(|e| e.into_inner());
		map.get(&TypeId::of::<T>())
			.and_then(|boxed| boxed.downcast_ref::<T>())
			.cloned()
	}
	/// Check if a value of the given type exists
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Extensions;
	///
	/// let extensions = Extensions::new();
	/// extensions.insert("hello".to_string());
	///
	/// assert!(extensions.contains::<String>());
	/// assert!(!extensions.contains::<u32>());
	/// ```
	pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
		let Some(map) = self.map.get() else {
			return false;
		};
		let map = map.lock().unwrap_or_else(|e| e.into_inner());
		map.contains_key(&TypeId::of::<T>())
	}
	/// Remove a value from extensions and return it
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Extensions;
	///
	/// let extensions = Extensions::new();
	/// extensions.insert(42u32);
	///
	/// assert_eq!(extensions.remove::<u32>(), Some(42));
	/// assert!(!extensions.contains::<u32>());
	/// assert_eq!(extensions.remove::<u32>(), None);
	/// ```
	pub fn remove<T>(&self) -> Option<T>
	where
		T: Send + Sync + 'static,
	{
		let map = self.map.get()?;
		let mut map = map.lock().unwrap_or_else(|e| e.into_inner());
		let boxed = map.remove(&TypeId::of::<T>())?;
		match boxed.downcast::<T>() {
			Ok(val) => Some(*val),
			Err(boxed) => {
				// Re-insert to prevent value loss on type mismatch
				map.insert(TypeId::of::<T>(), boxed);
				None
			}
		}
	}
	/// Clear all extensions
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Extensions;
	///
	/// let extensions = Extensions::new();
	/// extensions.insert(42u32);
	/// extensions.insert("hello".to_string());
	///
	/// assert!(extensions.contains::<u32>());
	/// assert!(extensions.contains::<String>());
	///
	/// extensions.clear();
	///
	/// assert!(!extensions.contains::<u32>());
	/// assert!(!extensions.contains::<String>());
	/// ```
	pub fn clear(&self) {
		if let Some(map) = self.map.get() {
			let mut map = map.lock().unwrap_or_else(|e| e.into_inner());
			map.clear();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[derive(Clone, Debug, PartialEq)]
	struct TestData {
		value: String,
	}

	#[rstest]
	fn test_newtype_bools_coexist_in_extensions() {
		// Arrange
		let extensions = Extensions::new();

		// Act
		extensions.insert(IsAuthenticated(true));
		extensions.insert(IsAdmin(false));
		extensions.insert(IsActive(true));

		// Assert
		assert_eq!(
			extensions.get::<IsAuthenticated>(),
			Some(IsAuthenticated(true))
		);
		assert_eq!(extensions.get::<IsAdmin>(), Some(IsAdmin(false)));
		assert_eq!(extensions.get::<IsActive>(), Some(IsActive(true)));
	}

	#[test]
	fn test_insert_and_get() {
		let extensions = Extensions::new();
		let data = TestData {
			value: "test".to_string(),
		};

		extensions.insert(data.clone());
		let retrieved = extensions.get::<TestData>();

		assert_eq!(retrieved, Some(data));
	}

	#[test]
	fn test_get_nonexistent() {
		let extensions = Extensions::new();
		let retrieved = extensions.get::<TestData>();

		assert_eq!(retrieved, None);
	}

	#[test]
	fn test_empty_read_methods_do_not_initialize_backing_store() {
		let extensions = Extensions::new();

		assert!(!extensions.contains::<TestData>());
		assert_eq!(extensions.get::<TestData>(), None);
		assert_eq!(extensions.remove::<TestData>(), None);
		extensions.clear();

		assert!(extensions.map.get().is_none());
	}

	#[test]
	fn test_contains() {
		let extensions = Extensions::new();
		extensions.insert(TestData {
			value: "test".to_string(),
		});

		assert!(extensions.contains::<TestData>());
		assert!(!extensions.contains::<String>());
	}

	#[test]
	fn test_remove() {
		let extensions = Extensions::new();
		let data = TestData {
			value: "test".to_string(),
		};

		extensions.insert(data.clone());
		let removed = extensions.remove::<TestData>();

		assert_eq!(removed, Some(data));
		assert!(!extensions.contains::<TestData>());
	}

	#[test]
	fn test_clear() {
		let extensions = Extensions::new();
		extensions.insert(TestData {
			value: "test".to_string(),
		});
		extensions.insert("another value".to_string());

		extensions.clear();

		assert!(!extensions.contains::<TestData>());
		assert!(!extensions.contains::<String>());
	}

	#[test]
	fn test_remove_wrong_type_preserves_value() {
		// Arrange
		let extensions = Extensions::new();
		extensions.insert(42u32);

		// Act - try to remove as wrong type
		let removed = extensions.remove::<String>();

		// Assert - removal fails and original value is preserved
		assert_eq!(removed, None);
		assert!(extensions.contains::<u32>());
		assert_eq!(extensions.get::<u32>(), Some(42));
	}

	#[test]
	fn test_multiple_types() {
		let extensions = Extensions::new();
		extensions.insert(TestData {
			value: "test".to_string(),
		});
		extensions.insert(42u32);
		extensions.insert("string value".to_string());

		assert_eq!(
			extensions.get::<TestData>(),
			Some(TestData {
				value: "test".to_string()
			})
		);
		assert_eq!(extensions.get::<u32>(), Some(42));
		assert_eq!(extensions.get::<String>(), Some("string value".to_string()));
	}

	#[test]
	fn test_clone_shares_backing_store() {
		// Arrange
		let original = Extensions::new();
		let cloned = original.clone();

		// Act - insert via clone
		cloned.insert(42u32);

		// Assert - original sees the value
		assert_eq!(original.get::<u32>(), Some(42));

		// Act - remove via original
		let removed = original.remove::<u32>();

		// Assert - clone no longer sees it
		assert_eq!(removed, Some(42));
		assert!(!cloned.contains::<u32>());
	}

	#[test]
	fn test_clone_initializes_shared_backing_store() {
		// Arrange
		let original = Extensions::new();

		// Act
		let cloned = original.clone();

		// Assert
		assert!(original.map.get().is_some());
		assert!(cloned.map.get().is_some());

		// Act - insert via original
		original.insert(42u32);

		// Assert - clone sees the value
		assert_eq!(cloned.get::<u32>(), Some(42));
	}
}
