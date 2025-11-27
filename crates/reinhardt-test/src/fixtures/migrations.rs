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

use reinhardt_migrations::registry::LocalRegistry;
use rstest::*;

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

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_migrations::{Migration, MigrationRegistry};

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
				app_label: "test_app".to_string(),
				name: "0001_initial".to_string(),
				operations: vec![],
				dependencies: vec![],
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
