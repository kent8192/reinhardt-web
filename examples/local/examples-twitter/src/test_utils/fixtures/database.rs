//! Database fixture for tests.
//!
//! Provides PostgreSQL container setup with migrations applied.

use crate::migrations::TwitterMigrations;
use reinhardt::db::DatabaseConnection;
use reinhardt::test::fixtures::postgres_with_migrations_from;
use reinhardt::test::testcontainers::{ContainerAsync, GenericImage};
use rstest::*;
use std::sync::Arc;

/// Type alias for test database tuple (container, connection)
pub type TestDatabase = (ContainerAsync<GenericImage>, Arc<DatabaseConnection>);

/// Database fixture with migrations applied.
///
/// Creates a PostgreSQL container and applies all Twitter migrations.
/// The container is automatically cleaned up when dropped.
///
/// # Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// use crate::test_utils::fixtures::test_database;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn my_test(#[future] test_database: TestDatabase) {
///     let (_container, db) = test_database.await;
///     // Use db connection
/// }
/// ```
#[fixture]
pub async fn test_database() -> TestDatabase {
	postgres_with_migrations_from::<TwitterMigrations>()
		.await
		.expect("Failed to create test database with migrations")
}
