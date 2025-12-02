//! MigrationExecutor Integration Tests with Real Database
//!
//! These tests verify MigrationExecutor with real PostgreSQL database operations.
//! Extracted from test_executor.rs to use reinhardt-test fixtures.
//!
//! **Test Coverage:**
//! - Migration execution with real database
//! - DatabaseMigrationRecorder integration
//! - Rollback operations
//! - Already applied migration detection
//! - Migration dependencies
//! - Column addition migrations
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_backends::DatabaseConnection;
use reinhardt_migrations::{
	ColumnDefinition, Migration, Operation, executor::DatabaseMigrationExecutor,
	recorder::DatabaseMigrationRecorder,
};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::{slice, sync::Arc};
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Helper Functions
// ============================================================================

fn leak_str(s: impl Into<String>) -> &'static str {
	Box::leak(s.into().into_boxed_str())
}

/// Create a simple migration for testing
fn create_test_migration(
	app: &'static str,
	name: &'static str,
	operations: Vec<Operation>,
) -> Migration {
	Migration {
		app_label: app,
		name,
		operations,
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: None,
	}
}

/// Create a basic column definition
fn create_basic_column(name: &'static str, type_def: &'static str) -> ColumnDefinition {
	ColumnDefinition {
		name,
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
		max_length: None,
	}
}

// ============================================================================
// Basic MigrationExecutor Integration Tests
// ============================================================================

/// Test running a simple set of migrations
///
/// **Test Intent**: Verify MigrationExecutor can apply multiple migrations in order
/// with real PostgreSQL database
///
/// **Integration Point**: MigrationExecutor → PostgreSQL CREATE TABLE operations
///
/// **Not Intent**: Migration planning, dependency resolution
#[rstest]
#[tokio::test]
async fn test_executor_basic_run(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to database");
	let mut executor = DatabaseMigrationExecutor::new(
		connection,
		reinhardt_backends::types::DatabaseType::Postgres,
	);

	// Create test migrations
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: leak_str("test_author"),
			columns: vec![
				create_basic_column("id", "SERIAL PRIMARY KEY"),
				create_basic_column("name", "TEXT NOT NULL"),
			],
			constraints: vec![],
		}],
	);

	let migration2 = create_test_migration(
		"testapp",
		"0002_add_book",
		vec![Operation::CreateTable {
			name: leak_str("test_book"),
			columns: vec![
				create_basic_column("id", "SERIAL PRIMARY KEY"),
				create_basic_column("title", "TEXT NOT NULL"),
				create_basic_column("author_id", "INTEGER"),
			],
			constraints: vec![],
		}],
	);

	// Build migration plan
	let plan = vec![migration1, migration2];

	// Execute migrations
	let result = executor.apply_migrations(&plan).await;
	assert!(result.is_ok(), "Migration should succeed");

	let execution_result = result.unwrap();
	assert_eq!(execution_result.applied.len(), 2);
	assert!(execution_result.failed.is_none());

	// Verify tables were created (PostgreSQL)
	let tables_query = sqlx::query(
		"SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
	)
	.fetch_all(_pool.as_ref())
	.await
	.unwrap();

	let table_names: Vec<String> = tables_query
		.iter()
		.map(|row| row.get::<String, _>("table_name"))
		.collect();

	assert!(table_names.contains(&"test_author".to_string()));
	assert!(table_names.contains(&"test_book".to_string()));
}

/// Test rolling back migrations
///
/// **Test Intent**: Verify MigrationExecutor can execute DROP TABLE operations
/// to rollback previously applied migrations
///
/// **Integration Point**: MigrationExecutor → PostgreSQL DROP TABLE
///
/// **Not Intent**: Transaction rollback, migration dependency rollback
#[rstest]
#[tokio::test]
async fn test_executor_rollback(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to database");
	let mut executor = DatabaseMigrationExecutor::new(
		connection,
		reinhardt_backends::types::DatabaseType::Postgres,
	);

	// Create and apply migrations
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: leak_str("rollback_test"),
			columns: vec![create_basic_column("id", "SERIAL PRIMARY KEY")],
			constraints: vec![],
		}],
	);

	executor
		.apply_migrations(slice::from_ref(&migration1))
		.await
		.unwrap();

	// Verify table exists
	let exists_before = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'rollback_test')",
	)
	.fetch_one(_pool.as_ref())
	.await
	.unwrap()
	.get::<bool, _>(0);
	assert!(exists_before, "Table should exist before rollback");

	// Now rollback
	let rollback_ops = vec![Operation::DropTable {
		name: leak_str("rollback_test"),
	}];

	let rollback_migration = create_test_migration("testapp", "0001_rollback", rollback_ops);

	let result = executor.apply_migrations(&[rollback_migration]).await;
	assert!(result.is_ok());

	// Verify table was dropped
	let exists_after = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'rollback_test')",
	)
	.fetch_one(_pool.as_ref())
	.await
	.unwrap()
	.get::<bool, _>(0);
	assert!(!exists_after, "Table should be dropped after rollback");
}

/// Test that already applied migrations are skipped
///
/// **Test Intent**: Verify MigrationExecutor detects and skips migrations
/// that have already been applied (recorded in migration history table)
///
/// **Integration Point**: MigrationExecutor → DatabaseMigrationRecorder
///
/// **Not Intent**: Migration dependency checking, migration plan optimization
#[rstest]
#[tokio::test]
async fn test_executor_already_applied(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to database");
	let mut executor = DatabaseMigrationExecutor::new(
		connection,
		reinhardt_backends::types::DatabaseType::Postgres,
	);

	let migration = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: leak_str("skip_test"),
			columns: vec![create_basic_column("id", "SERIAL PRIMARY KEY")],
			constraints: vec![],
		}],
	);

	// Apply once
	let result1 = executor
		.apply_migrations(slice::from_ref(&migration))
		.await
		.unwrap();
	assert_eq!(result1.applied.len(), 1, "First apply should succeed");

	// Apply again - should be skipped
	let result2 = executor
		.apply_migrations(slice::from_ref(&migration))
		.await
		.unwrap();

	// Should show 0 newly applied (already applied)
	assert_eq!(
		result2.applied.len(),
		0,
		"Already applied migration should be skipped"
	);

	// Verify table still exists (not recreated)
	let exists = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = 'skip_test')",
	)
	.fetch_one(_pool.as_ref())
	.await
	.unwrap()
	.get::<bool, _>(0);
	assert!(exists, "Table should still exist");
}

// ============================================================================
// Migration Dependencies Integration Tests
// ============================================================================

/// Test migrations with dependencies
///
/// **Test Intent**: Verify MigrationExecutor respects migration dependency order
/// when applying migrations across multiple apps
///
/// **Integration Point**: MigrationExecutor → Migration dependency resolution → Database
///
/// **Not Intent**: Circular dependency detection, dependency DAG validation
#[rstest]
#[tokio::test]
async fn test_executor_with_dependencies(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to database");
	let mut executor = DatabaseMigrationExecutor::new(
		connection,
		reinhardt_backends::types::DatabaseType::Postgres,
	);

	let migration1 = Migration {
		app_label: "app1",
		name: leak_str("0001_initial"),
		operations: vec![Operation::CreateTable {
			name: leak_str("dep_table1"),
			columns: vec![create_basic_column("id", "SERIAL PRIMARY KEY")],
			constraints: vec![],
		}],
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: None,
	};

	let migration2 = Migration {
		app_label: "app2",
		name: leak_str("0001_initial"),
		operations: vec![Operation::CreateTable {
			name: leak_str("dep_table2"),
			columns: vec![create_basic_column("id", "SERIAL PRIMARY KEY")],
			constraints: vec![],
		}],
		dependencies: vec![("app1", "0001_initial")],
		replaces: vec![],
		atomic: true,
		initial: None,
	};

	// Apply in correct order
	let result = executor.apply_migrations(&[migration1, migration2]).await;
	let execution_result = result.unwrap();
	assert_eq!(execution_result.applied.len(), 2);

	// Verify both tables were created
	let tables_query = sqlx::query(
		"SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'dep_table%'",
	)
	.fetch_all(_pool.as_ref())
	.await
	.unwrap();

	assert_eq!(tables_query.len(), 2, "Both tables should be created");
}

// ============================================================================
// DatabaseMigrationRecorder Integration Tests
// ============================================================================

/// Test that DatabaseMigrationRecorder properly records migrations to the database
///
/// **Test Intent**: Verify DatabaseMigrationRecorder can record and query
/// migration history in real PostgreSQL database
///
/// **Integration Point**: DatabaseMigrationRecorder → PostgreSQL migrations table
///
/// **Not Intent**: Migration execution, migration rollback
#[rstest]
#[tokio::test]
async fn test_executor_migration_recording(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to database");

	let recorder = DatabaseMigrationRecorder::new(connection);
	recorder.ensure_schema_table().await.unwrap();

	// Manually record a migration
	recorder
		.record_applied("testapp", "0001_initial")
		.await
		.unwrap();

	// Check if migration was recorded in the database
	let is_applied = recorder
		.is_applied("testapp", "0001_initial")
		.await
		.unwrap();
	assert!(is_applied, "Migration should be recorded as applied");

	// Check that non-existent migration is not recorded
	let is_not_applied = recorder.is_applied("testapp", "0002_second").await.unwrap();
	assert!(
		!is_not_applied,
		"Non-existent migration should not be recorded"
	);

	// Test recording multiple migrations
	recorder
		.record_applied("testapp", "0002_second")
		.await
		.unwrap();
	recorder
		.record_applied("otherapp", "0001_initial")
		.await
		.unwrap();

	// Verify all recorded
	assert!(
		recorder
			.is_applied("testapp", "0001_initial")
			.await
			.unwrap()
	);
	assert!(recorder.is_applied("testapp", "0002_second").await.unwrap());
	assert!(
		recorder
			.is_applied("otherapp", "0001_initial")
			.await
			.unwrap()
	);
}

// ============================================================================
// Schema Evolution Integration Tests
// ============================================================================

/// Test adding a column to existing table
///
/// **Test Intent**: Verify MigrationExecutor can execute ALTER TABLE ADD COLUMN
/// operations on real PostgreSQL database
///
/// **Integration Point**: MigrationExecutor → PostgreSQL ALTER TABLE
///
/// **Not Intent**: Column type changes, column removal, constraint changes
#[rstest]
#[tokio::test]
async fn test_executor_add_column_migration(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to database");
	let mut executor = DatabaseMigrationExecutor::new(
		connection,
		reinhardt_backends::types::DatabaseType::Postgres,
	);

	// First create a table
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "evolving_table",
			columns: vec![
				create_basic_column("id", "SERIAL PRIMARY KEY"),
				create_basic_column("name", "TEXT"),
			],
			constraints: vec![],
		}],
	);

	executor.apply_migrations(&[migration1]).await.unwrap();

	// Verify initial columns
	let initial_columns = sqlx::query(
		"SELECT column_name FROM information_schema.columns WHERE table_name = 'evolving_table'",
	)
	.fetch_all(_pool.as_ref())
	.await
	.unwrap();

	assert_eq!(initial_columns.len(), 2, "Should have 2 columns initially");

	// Then add a column
	let migration2 = create_test_migration(
		"testapp",
		"0002_add_email",
		vec![Operation::AddColumn {
			table: "evolving_table",
			column: create_basic_column("email", "TEXT"),
		}],
	);

	let result = executor.apply_migrations(&[migration2]).await;
	assert!(result.is_ok(), "Adding column should succeed");

	// Verify column was added
	let columns_query = sqlx::query(
		"SELECT column_name FROM information_schema.columns WHERE table_name = 'evolving_table'",
	)
	.fetch_all(_pool.as_ref())
	.await
	.unwrap();

	let column_names: Vec<String> = columns_query
		.iter()
		.map(|row| row.get::<String, _>("column_name"))
		.collect();

	assert!(
		column_names.contains(&"email".to_string()),
		"New column should exist"
	);
	assert_eq!(
		columns_query.len(),
		3,
		"Should have 3 columns after migration"
	);
}

/// Test complex migration with multiple operations
///
/// **Test Intent**: Verify MigrationExecutor can apply multiple operations
/// (CREATE TABLE, ADD COLUMN, CREATE INDEX) in a single migration
///
/// **Integration Point**: MigrationExecutor → Multiple PostgreSQL DDL operations
///
/// **Not Intent**: Transaction atomicity, rollback on partial failure
#[rstest]
#[tokio::test]
async fn test_executor_complex_migration(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to database");
	let mut executor = DatabaseMigrationExecutor::new(
		connection,
		reinhardt_backends::types::DatabaseType::Postgres,
	);

	// Create complex migration with multiple operations
	let migration = create_test_migration(
		"testapp",
		"0001_complex",
		vec![
			Operation::CreateTable {
				name: "complex_table",
				columns: vec![
					create_basic_column("id", "SERIAL PRIMARY KEY"),
					create_basic_column("username", "TEXT NOT NULL"),
				],
				constraints: vec![],
			},
			Operation::AddColumn {
				table: "complex_table",
				column: create_basic_column("email", "TEXT"),
			},
		],
	);

	let result = executor.apply_migrations(&[migration]).await;
	assert!(result.is_ok(), "Complex migration should succeed");

	// Verify table and columns were created
	let columns = sqlx::query(
		"SELECT column_name FROM information_schema.columns WHERE table_name = 'complex_table'",
	)
	.fetch_all(_pool.as_ref())
	.await
	.unwrap();

	let column_names: Vec<String> = columns
		.iter()
		.map(|row| row.get::<String, _>("column_name"))
		.collect();

	assert!(column_names.contains(&"id".to_string()));
	assert!(column_names.contains(&"username".to_string()));
	assert!(column_names.contains(&"email".to_string()));
}
