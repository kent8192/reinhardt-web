//! Migration Registry Test Fixtures
//!
//! Provides test-friendly migration registry helpers using `LocalRegistry`
//! for isolated unit testing without global state interference.
//!
//! # Usage
//!
//! In your tests, use the `migration_registry` fixture for an empty isolated registry:
//!
//! ```rust,ignore
//! use reinhardt_test::fixtures::*;
//! use reinhardt_migrations::Migration;
//! use rstest::*;
//!
//! #[rstest]
//! fn test_migration_registration(migration_registry: LocalRegistry) {
//!     let migration = Migration {
//!         app_label: "polls".to_string(),
//!         name: "0001_initial".to_string(),
//!         operations: vec![],
//!         dependencies: vec![],
//!     };
//!
//!     migration_registry.register(migration).unwrap();
//!     assert_eq!(migration_registry.all_migrations().len(), 1);
//! }
//! ```
//!
//! For production code or examples, use the `collect_migrations!` macro to register
//! migrations with the global registry.

use async_trait::async_trait;
use reinhardt_migrations::registry::LocalRegistry;
use reinhardt_migrations::{Migration, MigrationRepository, MigrationSource, Result};
use rstest::*;
use std::collections::HashMap;

/// Creates a new isolated migration registry for testing
///
/// Each test gets its own empty LocalRegistry instance, ensuring complete
/// isolation between test cases. This avoids the "duplicate distributed_slice"
/// errors that occur with linkme's global registry in test environments.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_test::fixtures::*;
/// use reinhardt_migrations::Migration;
/// use rstest::*;
///
/// #[rstest]
/// fn test_migration_operations(migration_registry: LocalRegistry) {
///     // Registry starts empty
///     assert!(migration_registry.all_migrations().is_empty());
///
///     // Register a migration
///     migration_registry.register(Migration {
///         app_label: "polls".to_string(),
///         name: "0001_initial".to_string(),
///         operations: vec![],
///         dependencies: vec![],
///     }).unwrap();
///
///     // Verify registration
///     assert_eq!(migration_registry.all_migrations().len(), 1);
/// }
/// ```
#[fixture]
pub fn migration_registry() -> LocalRegistry {
	LocalRegistry::new()
}

/// In-memory migration source for testing
///
/// Provides a simple implementation of `MigrationSource` that stores migrations
/// in memory. Useful for testing migration-related functionality without
/// filesystem or database dependencies.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_test::fixtures::TestMigrationSource;
/// use reinhardt_migrations::{Migration, MigrationSource};
///
/// #[tokio::test]
/// async fn test_source() {
///     let mut source = TestMigrationSource::new();
///     source.add_migration(Migration {
///         app_label: "polls".to_string(),
///         name: "0001_initial".to_string(),
///         operations: vec![],
///         dependencies: vec![],
///     });
///
///     let migrations = source.all_migrations().await.unwrap();
///     assert_eq!(migrations.len(), 1);
/// }
/// ```
pub struct TestMigrationSource {
	migrations: Vec<Migration>,
}

impl TestMigrationSource {
	/// Create a new empty TestMigrationSource
	pub fn new() -> Self {
		Self {
			migrations: Vec::new(),
		}
	}

	/// Create a TestMigrationSource with initial migrations
	pub fn with_migrations(migrations: Vec<Migration>) -> Self {
		Self { migrations }
	}

	/// Add a migration to the source
	pub fn add_migration(&mut self, migration: Migration) {
		self.migrations.push(migration);
	}

	/// Clear all migrations from the source
	pub fn clear(&mut self) {
		self.migrations.clear();
	}

	/// Get the number of migrations
	pub fn len(&self) -> usize {
		self.migrations.len()
	}

	/// Check if the source is empty
	pub fn is_empty(&self) -> bool {
		self.migrations.is_empty()
	}
}

impl Default for TestMigrationSource {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl MigrationSource for TestMigrationSource {
	async fn all_migrations(&self) -> Result<Vec<Migration>> {
		Ok(self.migrations.clone())
	}
}

/// In-memory migration repository for testing
///
/// Provides a simple implementation of `MigrationRepository` that stores migrations
/// in memory using a HashMap. Useful for testing migration persistence without
/// actual file I/O.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_test::fixtures::InMemoryRepository;
/// use reinhardt_migrations::{Migration, MigrationRepository};
///
/// #[tokio::test]
/// async fn test_repository() {
///     let mut repo = InMemoryRepository::new();
///
///     let migration = Migration {
///         app_label: "polls".to_string(),
///         name: "0001_initial".to_string(),
///         operations: vec![],
///         dependencies: vec![],
///     };
///
///     repo.save(&migration).await.unwrap();
///     let retrieved = repo.get("polls", "0001_initial").await.unwrap();
///     assert_eq!(retrieved.name, "0001_initial");
/// }
/// ```
pub struct InMemoryRepository {
	migrations: HashMap<(String, String), Migration>,
}

impl InMemoryRepository {
	/// Create a new empty InMemoryRepository
	pub fn new() -> Self {
		Self {
			migrations: HashMap::new(),
		}
	}

	/// Create an InMemoryRepository with initial migrations
	pub fn with_migrations(migrations: Vec<Migration>) -> Self {
		let mut repo = Self::new();
		for migration in migrations {
			let key = (migration.app_label.to_string(), migration.name.to_string());
			repo.migrations.insert(key, migration);
		}
		repo
	}

	/// Clear all migrations from the repository
	pub fn clear(&mut self) {
		self.migrations.clear();
	}

	/// Get the number of migrations in the repository
	pub fn len(&self) -> usize {
		self.migrations.len()
	}

	/// Check if the repository is empty
	pub fn is_empty(&self) -> bool {
		self.migrations.is_empty()
	}
}

impl Default for InMemoryRepository {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl MigrationRepository for InMemoryRepository {
	async fn save(&mut self, migration: &Migration) -> Result<()> {
		let key = (migration.app_label.to_string(), migration.name.to_string());
		self.migrations.insert(key, migration.clone());
		Ok(())
	}

	async fn get(&self, app_label: &str, name: &str) -> Result<Migration> {
		let key = (app_label.to_string(), name.to_string());
		self.migrations.get(&key).cloned().ok_or_else(|| {
			reinhardt_migrations::MigrationError::NotFound(format!("{}.{}", app_label, name))
		})
	}

	async fn list(&self, app_label: &str) -> Result<Vec<Migration>> {
		Ok(self
			.migrations
			.values()
			.filter(|m| m.app_label == app_label)
			.cloned()
			.collect())
	}

	async fn delete(&mut self, app_label: &str, name: &str) -> Result<()> {
		let key = (app_label.to_string(), name.to_string());
		self.migrations.remove(&key).ok_or_else(|| {
			reinhardt_migrations::MigrationError::NotFound(format!("{}.{}", app_label, name))
		})?;
		Ok(())
	}
}

/// Creates a new TestMigrationSource for testing
///
/// Provides an empty migration source that can be populated with test migrations.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_test::fixtures::*;
/// use reinhardt_migrations::{Migration, MigrationSource};
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_source(mut test_migration_source: TestMigrationSource) {
///     test_migration_source.add_migration(Migration {
///         app_label: "polls".to_string(),
///         name: "0001_initial".to_string(),
///         operations: vec![],
///         dependencies: vec![],
///     });
///
///     let migrations = test_migration_source.all_migrations().await.unwrap();
///     assert_eq!(migrations.len(), 1);
/// }
/// ```
#[fixture]
pub fn test_migration_source() -> TestMigrationSource {
	TestMigrationSource::new()
}

/// Creates a new InMemoryRepository for testing
///
/// Provides an empty migration repository that stores migrations in memory.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_test::fixtures::*;
/// use reinhardt_migrations::{Migration, MigrationRepository};
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_repository(mut in_memory_repository: InMemoryRepository) {
///     let migration = Migration {
///         app_label: "polls".to_string(),
///         name: "0001_initial".to_string(),
///         operations: vec![],
///         dependencies: vec![],
///     };
///
///     in_memory_repository.save(&migration).await.unwrap();
///     let retrieved = in_memory_repository.get("polls", "0001_initial").await.unwrap();
///     assert_eq!(retrieved.name, "0001_initial");
/// }
/// ```
#[fixture]
pub fn in_memory_repository() -> InMemoryRepository {
	InMemoryRepository::new()
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_migrations::Migration;
	use reinhardt_migrations::registry::MigrationRegistry;

	#[rstest]
	fn test_migration_registry_fixture(migration_registry: LocalRegistry) {
		assert!(migration_registry.all_migrations().is_empty());
	}

	#[rstest]
	fn test_registry_isolation_between_tests(migration_registry: LocalRegistry) {
		// This test runs independently - registry should be empty
		assert_eq!(migration_registry.all_migrations().len(), 0);

		migration_registry
			.register(Migration {
				initial: None,
				app_label: "test_app",
				name: "0001_initial",
				operations: vec![],
				dependencies: vec![],
				atomic: true,
				replaces: vec![],
			})
			.unwrap();

		assert_eq!(migration_registry.all_migrations().len(), 1);
	}

	#[rstest]
	fn test_another_isolated_test(migration_registry: LocalRegistry) {
		// Even though previous test registered a migration,
		// this new fixture instance should be empty
		assert_eq!(migration_registry.all_migrations().len(), 0);
	}
}
