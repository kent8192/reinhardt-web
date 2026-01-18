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

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, Migration, Operation, executor::DatabaseMigrationExecutor,
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

/// Create a simple migration for testing
fn create_test_migration(app: &str, name: &str, operations: Vec<Operation>) -> Migration {
	let mut migration = Migration::new(name, app);
	for op in operations {
		migration = migration.add_operation(op);
	}
	migration
}

/// Create a basic column definition
fn create_basic_column(name: &str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name: name.to_string(),
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
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
	let mut executor = DatabaseMigrationExecutor::new(connection);

	// Create test migrations
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "test_author".to_string(),
			columns: vec![
				create_basic_column("id", FieldType::Custom("SERIAL PRIMARY KEY".to_string())),
				create_basic_column("name", FieldType::Custom("TEXT NOT NULL".to_string())),
			],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		}],
	);

	let migration2 = create_test_migration(
		"testapp",
		"0002_add_book",
		vec![Operation::CreateTable {
			name: "test_book".to_string(),
			columns: vec![
				create_basic_column("id", FieldType::Custom("SERIAL PRIMARY KEY".to_string())),
				create_basic_column("title", FieldType::Custom("TEXT NOT NULL".to_string())),
				create_basic_column("author_id", FieldType::Custom("INTEGER".to_string())),
			],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
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
	let mut executor = DatabaseMigrationExecutor::new(connection);

	// Create and apply migrations
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "rollback_test".to_string(),
			columns: vec![create_basic_column(
				"id",
				FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
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
		name: "rollback_test".to_string(),
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
	let mut executor = DatabaseMigrationExecutor::new(connection);

	let migration = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "skip_test".to_string(),
			columns: vec![create_basic_column(
				"id",
				FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
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
	let mut executor = DatabaseMigrationExecutor::new(connection);

	let migration1 = Migration::new("0001_initial", "app1").add_operation(Operation::CreateTable {
		name: "dep_table1".to_string(),
		columns: vec![create_basic_column(
			"id",
			FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
		)],
		constraints: vec![],
		without_rowid: None,
		partition: None,
		interleave_in_parent: None,
	});

	let migration2 = Migration::new("0001_initial", "app2")
		.add_dependency("app1", "0001_initial")
		.add_operation(Operation::CreateTable {
			name: "dep_table2".to_string(),
			columns: vec![create_basic_column(
				"id",
				FieldType::Custom("SERIAL PRIMARY KEY".to_string()),
			)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		});

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
	let mut executor = DatabaseMigrationExecutor::new(connection);

	// First create a table
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "evolving_table".to_string(),
			columns: vec![
				create_basic_column("id", FieldType::Custom("SERIAL PRIMARY KEY".to_string())),
				create_basic_column("name", FieldType::Custom("TEXT".to_string())),
			],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
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
			table: "evolving_table".to_string(),
			column: create_basic_column("email", FieldType::Custom("TEXT".to_string())),
			mysql_options: None,
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
