use backends::DatabaseConnection;
use reinhardt_migrations::{ColumnDefinition, Migration, MigrationExecutor, Operation};
use sqlx::{Row, SqlitePool};

/// Test helper to create a simple migration
fn create_test_migration(app: &str, name: &str, operations: Vec<Operation>) -> Migration {
	Migration {
		app_label: app.to_string(),
		name: name.to_string(),
		operations,
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
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
			name: "test_author".to_string(),
			columns: vec![
				ColumnDefinition {
					name: "id".to_string(),
					type_definition: "INTEGER PRIMARY KEY".to_string(),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: "name".to_string(),
					type_definition: "TEXT NOT NULL".to_string(),
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
			name: "test_book".to_string(),
			columns: vec![
				ColumnDefinition {
					name: "id".to_string(),
					type_definition: "INTEGER PRIMARY KEY".to_string(),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: "title".to_string(),
					type_definition: "TEXT NOT NULL".to_string(),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: "author_id".to_string(),
					type_definition: "INTEGER".to_string(),
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
			name: "rollback_test".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(),
				type_definition: "INTEGER PRIMARY KEY".to_string(),
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
		.apply_migrations(&vec![migration1.clone()])
		.await
		.unwrap();

	// Now rollback
	let rollback_ops = vec![Operation::DropTable {
		name: "rollback_test".to_string(),
	}];

	let rollback_migration = create_test_migration("testapp", "0001_rollback", rollback_ops);

	let result = executor.apply_migrations(&vec![rollback_migration]).await;
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
			name: "skip_test".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(),
				type_definition: "INTEGER PRIMARY KEY".to_string(),
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
		.apply_migrations(&vec![migration.clone()])
		.await
		.unwrap();

	// Apply again - should be skipped
	let result = executor
		.apply_migrations(&vec![migration.clone()])
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

	let result = executor.apply_migrations(&vec![]).await;
	assert!(result.is_ok());

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
		app_label: "app1".to_string(),
		name: "0001_initial".to_string(),
		operations: vec![Operation::CreateTable {
			name: "dep_table1".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(),
				type_definition: "INTEGER PRIMARY KEY".to_string(),
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
	};

	let migration2 = Migration {
		app_label: "app2".to_string(),
		name: "0001_initial".to_string(),
		operations: vec![Operation::CreateTable {
			name: "dep_table2".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(),
				type_definition: "INTEGER PRIMARY KEY".to_string(),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				max_length: None,
			}],
			constraints: vec![],
		}],
		dependencies: vec![("app1".to_string(), "0001_initial".to_string())],
		replaces: vec![],
		atomic: true,
	};

	// Apply in correct order
	let result = executor
		.apply_migrations(&vec![migration1, migration2])
		.await;
	assert!(result.is_ok());

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
			name: "evolving_table".to_string(),
			columns: vec![
				ColumnDefinition {
					name: "id".to_string(),
					type_definition: "INTEGER PRIMARY KEY".to_string(),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
					max_length: None,
				},
				ColumnDefinition {
					name: "name".to_string(),
					type_definition: "TEXT".to_string(),
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

	executor.apply_migrations(&vec![migration1]).await.unwrap();

	// Then add a column
	let migration2 = create_test_migration(
		"testapp",
		"0002_add_email",
		vec![Operation::AddColumn {
			table: "evolving_table".to_string(),
			column: ColumnDefinition {
				name: "email".to_string(),
				type_definition: "TEXT".to_string(),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
				max_length: None,
			},
		}],
	);

	let result = executor.apply_migrations(&vec![migration2]).await;
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
