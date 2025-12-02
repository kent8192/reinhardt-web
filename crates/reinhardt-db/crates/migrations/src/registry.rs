//! Migration Registry Module
//!
//! Provides both global (production) and local (testing) migration registries
//! with a unified `MigrationRegistry` trait interface.
//!
//! # Architecture
//!
//! This module implements a hybrid registry architecture:
//!
//! - **GlobalRegistry**: Production registry using `linkme::distributed_slice` for
//!   compile-time collection of migrations. Supports runtime registration for
//!   dynamic scenarios.
//!
//! - **LocalRegistry**: Test-specific registry using pure runtime registration.
//!   Provides complete isolation between test cases without linkme's global state.
//!
//! - **MigrationRegistry** trait: Unified interface for both implementations,
//!   enabling polymorphic usage in production and test code.
//!
//! # Usage in Production
//!
//! Use the global registry for production code:
//!
//! ```rust,ignore
//! use reinhardt_migrations::registry::{global_registry, MigrationRegistry};
//!
//! let registry = global_registry();
//! let all_migrations = registry.all_migrations();
//! let polls_migrations = registry.migrations_for_app("polls");
//! ```
//!
//! Migrations are automatically registered via the `collect_migrations!` macro:
//!
//! ```rust,ignore
//! // In your app's migrations.rs
//! pub mod _0001_initial;
//! pub mod _0002_add_fields;
//!
//! reinhardt::collect_migrations!(
//!     app_label = "polls",
//!     _0001_initial,
//!     _0002_add_fields,
//! );
//! ```
//!
//! # Usage in Tests
//!
//! Use local registries for isolated unit tests:
//!
//! ```rust,ignore
//! use reinhardt_migrations::registry::{LocalRegistry, MigrationRegistry};
//!
//! #[test]
//! fn test_migration_operations() {
//!     let registry = LocalRegistry::new();
//!
//!     registry.register(Migration {
//!         app_label: "polls",
//!         name: "0001_initial",
//!         operations: vec![],
//!         dependencies: vec![],
//!     }).unwrap();
//!
//!     let migrations = registry.all_migrations();
//!     assert_eq!(migrations.len(), 1);
//! }
//! ```
//!
//! For convenience, use the `reinhardt-test` fixtures:
//!
//! ```rust,ignore
//! use reinhardt_test::fixtures::*;
//! use rstest::*;
//!
//! #[rstest]
//! fn test_with_fixture(migration_registry: LocalRegistry) {
//!     // Registry is empty and isolated
//!     assert!(migration_registry.all_migrations().is_empty());
//! }
//! ```

// Module declarations (Rust 2024 Edition)
pub mod global;
pub mod local;
pub mod traits;

// Re-export commonly used items
pub use global::{GlobalRegistry, MIGRATION_PROVIDERS, global_registry};
pub use local::LocalRegistry;
pub use traits::MigrationRegistry;

// Deprecated compatibility exports (to be removed in next major version)
#[deprecated(
	since = "0.1.0",
	note = "Use `global_registry().all_migrations()` instead"
)]
pub fn all_migrations() -> Vec<crate::Migration> {
	global_registry().all_migrations()
}

#[deprecated(
	since = "0.1.0",
	note = "Use `global_registry().migrations_for_app(app_label)` instead"
)]
pub fn migrations_for_app(app_label: &str) -> Vec<crate::Migration> {
	global_registry().migrations_for_app(app_label)
}

#[deprecated(
	since = "0.1.0",
	note = "Use `global_registry().registered_app_labels()` instead"
)]
pub fn registered_app_labels() -> Vec<String> {
	global_registry().registered_app_labels()
}

#[deprecated(
	since = "0.1.0",
	note = "Use `global_registry().all_migrations().len()` instead"
)]
pub fn migration_count() -> usize {
	global_registry().all_migrations().len()
}
