use reinhardt_backends::DatabaseConnection;
use reinhardt_migrations::{
	ColumnDefinition, DatabaseMigrationExecutor, FieldType, Migration, Operation,
};

/// Test helper to create a simple migration
fn create_test_migration(app: &str, name: &str, operations: Vec<Operation>) -> Migration {
	let mut migration = Migration::new(name, app);
	for op in operations {
		migration = migration.add_operation(op);
	}
	migration
}

#[tokio::test]
async fn test_executor_basic_run() {
	// Test running a simple set of migrations
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to database");

	let mut executor = DatabaseMigrationExecutor::new(db.clone());

	// Create test migrations
	let migration1 = create_test_migration(
		"testapp_basic_run",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "test_author".to_string(),
			columns: vec![
				ColumnDefinition {
					name: "id".to_string(),
					type_definition: FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				ColumnDefinition {
					name: "name".to_string(),
					type_definition: FieldType::Custom("TEXT NOT NULL".to_string()),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
			],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		}],
	);

	let migration2 = create_test_migration(
		"testapp_basic_run",
		"0002_add_book",
		vec![Operation::CreateTable {
			name: "test_book".to_string(),
			columns: vec![
				ColumnDefinition {
					name: "id".to_string(),
					type_definition: FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				ColumnDefinition {
					name: "title".to_string(),
					type_definition: FieldType::Custom("TEXT NOT NULL".to_string()),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				ColumnDefinition {
					name: "author_id".to_string(),
					type_definition: FieldType::Custom("INTEGER".to_string()),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
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

	// Verify tables were created using the high-level API
	let connection = executor.connection();
	let tables_query = connection
		.fetch_all("SELECT name FROM sqlite_master WHERE type='table'", vec![])
		.await
		.unwrap();

	let table_names: Vec<String> = tables_query
		.iter()
		.filter_map(|row| row.get::<String>("name").ok())
		.collect();

	assert!(table_names.contains(&"test_author".to_string()));
	assert!(table_names.contains(&"test_book".to_string()));
}

#[tokio::test]
async fn test_executor_rollback() {
	// Test rolling back migrations
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to database");

	let mut executor = DatabaseMigrationExecutor::new(db.clone());

	// Create and apply migrations
	let migration1 = create_test_migration(
		"testapp_rollback",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "rollback_test".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(),
				type_definition: FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			}],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		}],
	);

	executor
		.apply_migrations(std::slice::from_ref(&migration1))
		.await
		.unwrap();

	// Now rollback
	let rollback_ops = vec![Operation::DropTable {
		name: "rollback_test".to_string(),
	}];

	let rollback_migration =
		create_test_migration("testapp_rollback", "0001_rollback", rollback_ops);

	let result = executor.apply_migrations(&[rollback_migration]).await;
	assert!(result.is_ok());

	// Verify table was dropped using high-level API
	let connection = executor.connection();
	let tables_query = connection
		.fetch_all(
			"SELECT name FROM sqlite_master WHERE type='table' AND name='rollback_test'",
			vec![],
		)
		.await
		.unwrap();

	assert_eq!(tables_query.len(), 0, "Table should be dropped");
}

#[tokio::test]
async fn test_executor_already_applied() {
	// Test that already applied migrations are skipped
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to database");

	let mut executor = DatabaseMigrationExecutor::new(db.clone());

	let migration = create_test_migration(
		"testapp_already_applied",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "skip_test".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(),
				type_definition: FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			}],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
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
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to database");

	let mut executor = DatabaseMigrationExecutor::new(db);

	let result = executor.apply_migrations(&[]).await;
	let execution_result = result.unwrap();
	assert_eq!(execution_result.applied.len(), 0);
	assert!(execution_result.failed.is_none());
}

#[tokio::test]
async fn test_executor_with_dependencies() {
	// Test migrations with dependencies
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to database");

	let mut executor = DatabaseMigrationExecutor::new(db.clone());

	let migration1 = Migration::new("0001_initial", "app1").add_operation(Operation::CreateTable {
		name: "dep_table1".to_string(),
		columns: vec![ColumnDefinition {
			name: "id".to_string(),
			type_definition: FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
			not_null: false,
			unique: false,
			primary_key: false,
			auto_increment: false,
			default: None,
		}],
		constraints: vec![],
		without_rowid: None,
		partition: None,
		interleave_in_parent: None,
	});

	let migration2 = Migration::new("0001_initial", "app2")
		.add_dependency("app1", "0001_initial")
		.add_operation(Operation::CreateTable {
			name: "dep_table2".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(),
				type_definition: FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			}],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		});

	// Apply in correct order
	let result = executor.apply_migrations(&[migration1, migration2]).await;
	let execution_result = result.unwrap();
	assert_eq!(execution_result.applied.len(), 2);
}

#[tokio::test]
async fn test_executor_migration_recording() {
	use reinhardt_migrations::recorder::DatabaseMigrationRecorder;

	// Test that DatabaseMigrationRecorder properly records migrations to the database
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
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
}

#[tokio::test]
async fn test_executor_add_column_migration() {
	// Test adding a column to existing table
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.expect("Failed to connect to database");

	let mut executor = DatabaseMigrationExecutor::new(db.clone());

	// First create a table
	let migration1 = create_test_migration(
		"testapp_add_column",
		"0001_initial",
		vec![Operation::CreateTable {
			name: "evolving_table".to_string(),
			columns: vec![
				ColumnDefinition {
					name: "id".to_string(),
					type_definition: FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				ColumnDefinition {
					name: "name".to_string(),
					type_definition: FieldType::Custom("TEXT".to_string()),
					not_null: false,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
			],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		}],
	);

	executor.apply_migrations(&[migration1]).await.unwrap();

	// Then add a column
	let migration2 = create_test_migration(
		"testapp_add_column",
		"0002_add_email",
		vec![Operation::AddColumn {
			table: "evolving_table".to_string(),
			column: ColumnDefinition {
				name: "email".to_string(),
				type_definition: FieldType::Custom("TEXT".to_string()),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
			mysql_options: None,
		}],
	);

	let result = executor.apply_migrations(&[migration2]).await;
	assert!(result.is_ok(), "Adding column should succeed");

	// Verify column was added using high-level API
	let connection = executor.connection();
	let columns_query = connection
		.fetch_all("PRAGMA table_info(evolving_table)", vec![])
		.await
		.unwrap();

	let column_names: Vec<String> = columns_query
		.iter()
		.filter_map(|row| row.get::<String>("name").ok())
		.collect();

	assert!(
		column_names.contains(&"email".to_string()),
		"New column should exist"
	);
}
