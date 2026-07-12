//! Test fixtures and utilities for Reinhardt framework testing
//!
//! This module re-exports all fixtures from `reinhardt-testkit` and adds
//! additional fixtures that depend on functional crates (reinhardt-auth,
//! reinhardt-admin).

// Re-export all submodules from testkit for path compatibility
#[cfg(native)]
pub use reinhardt_testkit::fixtures::{client, dcl, di, loader, mock, server};

#[cfg(all(native, feature = "testcontainers"))]
pub use reinhardt_testkit::fixtures::{
	resources, schema, shared_postgres, testcontainers, validator,
};

#[cfg(native)]
pub use reinhardt_testkit::fixtures::migrations;

// Admin settings fixtures (re-exported from testkit, requires admin feature)
#[cfg(all(native, feature = "admin"))]
pub use reinhardt_testkit::fixtures::admin;

// Server function fixtures (re-exported from testkit)
#[cfg(all(native, feature = "server-fn-test"))]
pub use reinhardt_testkit::fixtures::server_fn;

// Re-export all public items from testkit fixtures
#[cfg(native)]
pub use reinhardt_testkit::fixtures::*;

/// Load a Django-compatible model fixture file into the active test database.
#[cfg(native)]
pub async fn load_model_fixture_file(
	path: impl AsRef<std::path::Path>,
) -> reinhardt_db::orm::fixtures::FixtureResult<usize> {
	let content = std::fs::read_to_string(path)
		.map_err(|error| reinhardt_db::orm::fixtures::FixtureError::Database(error.to_string()))?;
	let records: Vec<reinhardt_db::orm::fixtures::FixtureRecord> = serde_json::from_str(&content)?;
	reinhardt_db::orm::fixtures::load_fixture_records(&records).await
}

// ============================================================================
// Modules specific to reinhardt-test (depend on functional crates)
// ============================================================================

// Authentication fixtures (depends on reinhardt-auth)
#[cfg(native)]
pub mod auth;

// Admin panel fixtures (depends on reinhardt-admin)
#[cfg(all(native, feature = "admin", feature = "testcontainers"))]
pub mod admin_panel;

// Admin migration fixtures (depends on reinhardt-admin)
#[cfg(all(native, feature = "admin", feature = "testcontainers"))]
pub mod admin_migrations;

// WASM frontend test fixtures and E2E browser testing fixtures
#[cfg(any(
	all(wasm, feature = "wasm"),
	all(feature = "e2e", native),
	all(feature = "e2e-cdp", native)
))]
pub mod wasm;

// Admin integration fixtures (conditional on admin + testcontainers features)
#[cfg(all(native, feature = "admin", feature = "testcontainers"))]
pub use admin_migrations::{AdminTableCreator, admin_table_creator};

#[cfg(all(test, native))]
mod database_reexport_tests {
	use super::{TestDatabase, TestDatabaseBackend, test_database};

	struct EmptyProvider;

	impl reinhardt_db::migrations::MigrationProvider for EmptyProvider {
		fn migrations() -> Vec<reinhardt_db::migrations::Migration> {
			Vec::new()
		}
	}

	#[test]
	fn database_types_are_reexported() {
		let builder = TestDatabase::builder();
		let _macro_name_is_importable = stringify!(test_database);
		let _database_future = test_database!(migrations = EmptyProvider);

		assert_eq!(builder.backend(), TestDatabaseBackend::SqliteFile);
	}
}
