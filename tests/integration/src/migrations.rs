//! Test migration registry
//!
//! Provides helper functions to apply test migrations using DatabaseMigrationExecutor.
//!
//! # Architecture
//!
//! Each migration module exposes a `migration()` function that returns a `Migration` instance.
//! The registry provides utility functions to apply specific sets of migrations:
//!
//! - `apply_basic_test_migrations()`: For validator_test_common.rs tests
//! - `apply_constraint_test_migrations()`: For validator_orm_constraints.rs tests
//! - `apply_async_query_test_migrations()`: For async_query_integration.rs tests
//!
//! # Usage
//!
//! In test fixtures, call the appropriate apply function with a DatabaseConnection:
//!
//! ```rust,ignore
//! use reinhardt_backends::DatabaseConnection;
//! use reinhardt_integration_tests::migrations;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let connection = DatabaseConnection::connect_postgres("postgres://localhost/test").await?;
//! migrations::apply_basic_test_migrations(&connection).await?;
//! # Ok(())
//! # }
//! ```

use reinhardt_backends::{DatabaseConnection, DatabaseType};
use reinhardt_migrations::MigrationError;
use reinhardt_migrations::executor::DatabaseMigrationExecutor;

mod create_async_query_test_tables;
mod create_constraint_test_tables;
mod create_test_tables;

use create_async_query_test_tables::migration as migration_0003;
use create_constraint_test_tables::migration as migration_0002;
use create_test_tables::migration as migration_0001;

/// Apply all test migrations to a database connection
///
/// Applies all three test migrations in order:
/// 1. Basic test tables (test_users, test_products, test_orders)
/// 2. Constraint test tables (test_posts, test_comments)
/// 3. Async query test tables (test_models)
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_backends::DatabaseConnection;
/// use reinhardt_integration_tests::migrations;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = DatabaseConnection::connect_postgres("postgres://localhost/test").await?;
/// migrations::apply_test_migrations(&connection).await?;
/// # Ok(())
/// # }
/// ```
pub async fn apply_test_migrations(connection: &DatabaseConnection) -> Result<(), MigrationError> {
	let db_type = detect_database_type(connection);
	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), db_type);

	// Apply migrations in dependency order
	let migrations = vec![migration_0001(), migration_0002(), migration_0003()];

	executor.apply_migrations(&migrations).await?;

	Ok(())
}

/// Apply only basic test tables (for validator_test_common.rs tests)
///
/// Applies migration 0001 which creates:
/// - test_users
/// - test_products
/// - test_orders
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_backends::DatabaseConnection;
/// use reinhardt_integration_tests::migrations;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = DatabaseConnection::connect_postgres("postgres://localhost/test").await?;
/// migrations::apply_basic_test_migrations(&connection).await?;
/// # Ok(())
/// # }
/// ```
pub async fn apply_basic_test_migrations(
	connection: &DatabaseConnection,
) -> Result<(), MigrationError> {
	let db_type = detect_database_type(connection);
	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), db_type);

	executor.apply_migrations(&[migration_0001()]).await?;

	Ok(())
}

/// Apply constraint test tables (for validator_orm_constraints.rs tests)
///
/// Applies migrations 0001 and 0002 which create:
/// - test_users, test_products, test_orders (from 0001)
/// - test_posts, test_comments (from 0002)
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_backends::DatabaseConnection;
/// use reinhardt_integration_tests::migrations;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = DatabaseConnection::connect_postgres("postgres://localhost/test").await?;
/// migrations::apply_constraint_test_migrations(&connection).await?;
/// # Ok(())
/// # }
/// ```
pub async fn apply_constraint_test_migrations(
	connection: &DatabaseConnection,
) -> Result<(), MigrationError> {
	let db_type = detect_database_type(connection);
	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), db_type);

	// 0002 depends on 0001, so apply both in order
	executor
		.apply_migrations(&[migration_0001(), migration_0002()])
		.await?;

	Ok(())
}

/// Apply async query test tables (for async_query_integration.rs tests)
///
/// Applies migration 0003 which creates:
/// - test_models
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_backends::DatabaseConnection;
/// use reinhardt_integration_tests::migrations;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = DatabaseConnection::connect_postgres("postgres://localhost/test").await?;
/// migrations::apply_async_query_test_migrations(&connection).await?;
/// # Ok(())
/// # }
/// ```
pub async fn apply_async_query_test_migrations(
	connection: &DatabaseConnection,
) -> Result<(), MigrationError> {
	let db_type = detect_database_type(connection);
	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), db_type);

	executor.apply_migrations(&[migration_0003()]).await?;

	Ok(())
}

/// Detect database type from connection
///
/// Currently only PostgreSQL is supported for test migrations.
/// Returns DatabaseType::Postgres.
fn detect_database_type(_connection: &DatabaseConnection) -> DatabaseType {
	// TODO: Detect database type from connection
	// For now, assume PostgreSQL as that's what TestContainers uses
	DatabaseType::Postgres
}
