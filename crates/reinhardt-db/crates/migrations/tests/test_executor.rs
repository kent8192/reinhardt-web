use reinhardt_backends::DatabaseConnection;
use reinhardt_migrations::{ColumnDefinition, Migration, MigrationExecutor, Operation};
use sqlx::{Row, SqlitePool};

/// Helper function to leak a string to get a 'static lifetime
fn leak_str(s: impl Into<String>) -> &'static str {
	Box::leak(s.into().into_boxed_str())
}

/// Test helper to create a simple migration
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

#[tokio::test]
async fn test_executor_basic_run() {
	// Test running a simple set of migrations
	let pool = SqlitePool::connect("sqlite::memory:")
		.await
		.expect("Failed to create pool");

	let mut executor = MigrationExecutor::new(pool.clone());

	// Create test migrations
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: leak_str("test_author"),
			columns: vec![
				ColumnDefinition {
					name: leak_str("id"),
					type_definition: "INTEGER PRIMARY KEY",
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: leak_str("name"),
					type_definition: "TEXT NOT NULL",
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
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
				ColumnDefinition {
					name: leak_str("id"),
					type_definition: "INTEGER PRIMARY KEY",
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: leak_str("title"),
					type_definition: "TEXT NOT NULL",
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: leak_str("author_id"),
					type_definition: "INTEGER",
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
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

	// Verify tables were created
	let tables_query = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
		.fetch_all(&pool)
		.await
		.unwrap();

	let table_names: Vec<String> = tables_query
		.iter()
		.map(|row| row.get::<String, _>("name"))
		.collect();

	assert!(table_names.contains(&"test_author".to_string()));
	assert!(table_names.contains(&"test_book".to_string()));
}

#[tokio::test]
async fn test_executor_rollback() {
	// Test rolling back migrations
	let pool = SqlitePool::connect("sqlite::memory:")
		.await
		.expect("Failed to create pool");

	let mut executor = MigrationExecutor::new(pool.clone());

	// Create and apply migrations
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: leak_str("rollback_test"),
			columns: vec![ColumnDefinition {
				name: leak_str("id"),
				type_definition: "INTEGER PRIMARY KEY",
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				max_length: None,
			}],
			constraints: vec![],
		}],
	);

	executor
		.apply_migrations(std::slice::from_ref(&migration1))
		.await
		.unwrap();

	// Now rollback
	let rollback_ops = vec![Operation::DropTable {
		name: leak_str("rollback_test"),
	}];

	let rollback_migration = create_test_migration("testapp", "0001_rollback", rollback_ops);

	let result = executor.apply_migrations(&[rollback_migration]).await;
	assert!(result.is_ok());

	// Verify table was dropped
	let tables_query =
		sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='rollback_test'")
			.fetch_all(&pool)
			.await
			.unwrap();

	assert_eq!(tables_query.len(), 0, "Table should be dropped");
}

#[tokio::test]
async fn test_executor_already_applied() {
	// Test that already applied migrations are skipped
	let pool = SqlitePool::connect("sqlite::memory:")
		.await
		.expect("Failed to create pool");

	let mut executor = MigrationExecutor::new(pool.clone());

	let migration = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: leak_str("skip_test"),
			columns: vec![ColumnDefinition {
				name: leak_str("id"),
				type_definition: "INTEGER PRIMARY KEY",
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				max_length: None,
			}],
			constraints: vec![],
		}],
	);

	// Apply once
	executor
		.apply_migrations(std::slice::from_ref(&migration))
		.await
		.unwrap();

	// Apply again - should be skipped
	let result = executor
		.apply_migrations(std::slice::from_ref(&migration))
		.await
		.unwrap();

	// Should show 0 newly applied (already applied)
	assert_eq!(
		result.applied.len(),
		0,
		"Already applied migration should be skipped"
	);
}

#[tokio::test]
async fn test_executor_empty_plan() {
	// Test that empty migration plan doesn't cause issues
	let pool = SqlitePool::connect("sqlite::memory:")
		.await
		.expect("Failed to create pool");

	let mut executor = MigrationExecutor::new(pool);

	let result = executor.apply_migrations(&[]).await;
	let execution_result = result.unwrap();
	assert_eq!(execution_result.applied.len(), 0);
	assert!(execution_result.failed.is_none());
}

#[tokio::test]
async fn test_executor_with_dependencies() {
	// Test migrations with dependencies
	let pool = SqlitePool::connect("sqlite::memory:")
		.await
		.expect("Failed to create pool");

	let mut executor = MigrationExecutor::new(pool.clone());

	let migration1 = Migration {
		app_label: "app1",
		name: leak_str("0001_initial"),
		operations: vec![Operation::CreateTable {
			name: leak_str("dep_table1"),
			columns: vec![ColumnDefinition {
				name: leak_str("id"),
				type_definition: "INTEGER PRIMARY KEY",
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				max_length: None,
			}],
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
			columns: vec![ColumnDefinition {
				name: leak_str("id"),
				type_definition: "INTEGER PRIMARY KEY",
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				max_length: None,
			}],
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
}

#[tokio::test]
async fn test_executor_migration_recording() {
	use reinhardt_migrations::recorder::DatabaseMigrationRecorder;

	// Test that DatabaseMigrationRecorder properly records migrations to the database
	let pool = SqlitePool::connect("sqlite::memory:")
		.await
		.expect("Failed to create pool");

	let connection = DatabaseConnection::from_sqlite_pool(pool.clone());
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
}

#[tokio::test]
async fn test_executor_add_column_migration() {
	// Test adding a column to existing table
	let pool = SqlitePool::connect("sqlite::memory:")
		.await
		.expect("Failed to create pool");

	let mut executor = MigrationExecutor::new(pool.clone());

	// First create a table
	let migration1 = create_test_migration(
		"testapp",
		"0001_initial",
		vec![Operation::CreateTable {
			name: leak_str("evolving_table"),
			columns: vec![
				ColumnDefinition {
					name: leak_str("id"),
					type_definition: "INTEGER PRIMARY KEY",
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: leak_str("name"),
					type_definition: "TEXT",
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
			],
			constraints: vec![],
		}],
	);

	executor.apply_migrations(&[migration1]).await.unwrap();

	// Then add a column
	let migration2 = create_test_migration(
		"testapp",
		"0002_add_email",
		vec![Operation::AddColumn {
			table: leak_str("evolving_table"),
			column: ColumnDefinition {
				name: leak_str("email"),
				type_definition: "TEXT",
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				max_length: None,
			},
		}],
	);

	let result = executor.apply_migrations(&[migration2]).await;
	assert!(result.is_ok(), "Adding column should succeed");

	// Verify column was added
	let columns_query = sqlx::query("PRAGMA table_info(evolving_table)")
		.fetch_all(&pool)
		.await
		.unwrap();

	let column_names: Vec<String> = columns_query
		.iter()
		.map(|row| row.get::<String, _>("name"))
		.collect();

	assert!(
		column_names.contains(&"email".to_string()),
		"New column should exist"
	);
}
