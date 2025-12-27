//! Tests for AutoMigrationGenerator duplicate detection
//!
//! Tests the duplicate migration detection feature that prevents
//! generating identical migrations multiple times.

use reinhardt_migrations::{
	AutoMigrationError, AutoMigrationGenerator, ColumnDefinition, ColumnSchema, DatabaseSchema,
	FieldType, FilesystemRepository, Migration, MigrationRepository, Operation, TableSchema,
};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Helper to create a simple table schema
fn create_table_schema(table_name: &'static str) -> TableSchema {
	let mut columns = BTreeMap::new();
	columns.insert(
		"id".to_string(),
		ColumnSchema {
			name: "id",
			data_type: FieldType::Integer,
			nullable: false,
			default: None,
			primary_key: true,
			auto_increment: true,
		},
	);
	columns.insert(
		"name".to_string(),
		ColumnSchema {
			name: "name",
			data_type: FieldType::VarChar(100),
			nullable: false,
			default: None,
			primary_key: false,
			auto_increment: false,
		},
	);

	TableSchema {
		name: table_name,
		columns,
		indexes: Vec::new(),
		constraints: Vec::new(),
	}
}

#[tokio::test]
async fn test_duplicate_migration_detection() {
	// Create target schema with a simple table
	let mut target_schema = DatabaseSchema::default();
	target_schema
		.tables
		.insert("users".to_string(), create_table_schema("users"));

	// Create generator with temp directory
	let temp_dir = std::env::temp_dir().join("test_duplicate_migration");
	let _ = std::fs::remove_dir_all(&temp_dir); // Clean up from previous runs
	std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

	let repository = Arc::new(tokio::sync::Mutex::new(FilesystemRepository::new(
		temp_dir.clone(),
	)));
	let generator = AutoMigrationGenerator::new(target_schema.clone(), repository.clone());

	// First generation should succeed (no previous migration)
	let empty_schema = DatabaseSchema::default();
	let result1 = generator.generate("test_app", empty_schema.clone()).await;
	assert!(result1.is_ok(), "First generation should succeed");

	let migration_result = result1.unwrap();
	assert!(
		!migration_result.operations.is_empty(),
		"Should generate operations"
	);

	// Save the first migration to repository (this is what the caller would do)
	let first_migration = Migration {
		app_label: "test_app",
		name: "0001_initial",
		operations: migration_result.operations.clone(),
		dependencies: vec![],
		replaces: vec![],
		atomic: true,
		initial: Some(true),
		state_only: false,
		database_only: false,
	};
	{
		let mut repo = repository.lock().await;
		repo.save(&first_migration)
			.await
			.expect("Failed to save migration");
	}

	// Second generation with same schema should fail with DuplicateMigration error
	let result2 = generator.generate("test_app", empty_schema).await;

	match result2 {
		Err(AutoMigrationError::DuplicateMigration) => {
			// Expected error
		}
		Err(e) => panic!("Expected DuplicateMigration error, got: {:?}", e),
		Ok(_) => panic!("Expected DuplicateMigration error, but generation succeeded"),
	}

	// Clean up
	let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_different_migrations_allowed() {
	// Create target schema with first table
	let mut target_schema1 = DatabaseSchema::default();
	target_schema1
		.tables
		.insert("users".to_string(), create_table_schema("users"));

	let temp_dir = std::env::temp_dir().join("test_different_migrations");
	let _ = std::fs::remove_dir_all(&temp_dir); // Clean up from previous runs
	std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

	let repository = Arc::new(tokio::sync::Mutex::new(FilesystemRepository::new(
		temp_dir.clone(),
	)));
	let generator = AutoMigrationGenerator::new(target_schema1.clone(), repository.clone());

	// First generation
	let empty_schema = DatabaseSchema::default();
	let result1 = generator.generate("test_app", empty_schema).await;
	assert!(result1.is_ok(), "First generation should succeed");

	let _operations1 = result1.unwrap().operations;

	// Create different target schema with additional table
	let mut target_schema2 = DatabaseSchema::default();
	target_schema2
		.tables
		.insert("users".to_string(), create_table_schema("users"));
	target_schema2
		.tables
		.insert("posts".to_string(), create_table_schema("posts"));

	let generator2 = AutoMigrationGenerator::new(target_schema2, repository);

	// Second generation with different operations should succeed
	let result2 = generator2.generate("test_app", target_schema1).await;

	assert!(
		result2.is_ok(),
		"Generation with different operations should succeed"
	);

	let operations2 = result2.unwrap().operations;
	assert!(!operations2.is_empty(), "Should generate new operations");

	// Clean up
	let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_no_changes_error() {
	// Create identical target and current schemas
	let mut schema = DatabaseSchema::default();
	schema
		.tables
		.insert("users".to_string(), create_table_schema("users"));

	let temp_dir = std::env::temp_dir().join("test_no_changes");
	let _ = std::fs::remove_dir_all(&temp_dir); // Clean up from previous runs
	std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

	let repository = Arc::new(tokio::sync::Mutex::new(FilesystemRepository::new(
		temp_dir.clone(),
	)));
	let generator = AutoMigrationGenerator::new(schema.clone(), repository);

	// Should fail with NoChangesDetected error
	let result = generator.generate("test_app", schema).await;

	match result {
		Err(AutoMigrationError::NoChangesDetected) => {
			// Expected error
		}
		Err(e) => panic!("Expected NoChangesDetected error, got: {:?}", e),
		Ok(_) => panic!("Expected NoChangesDetected error, but generation succeeded"),
	}

	// Clean up
	let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_operation_equality() {
	// Test that identical operations are considered equal
	let col1 = ColumnDefinition {
		name: "id",
		type_definition: FieldType::Integer,
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: false,
		default: None,
	};

	let col2 = ColumnDefinition {
		name: "id",
		type_definition: FieldType::Integer,
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: false,
		default: None,
	};

	assert_eq!(col1, col2, "Identical column definitions should be equal");

	let op1 = Operation::CreateTable {
		name: "users",
		columns: vec![col1],
		constraints: vec![],
	};

	let op2 = Operation::CreateTable {
		name: "users",
		columns: vec![col2],
		constraints: vec![],
	};

	assert_eq!(op1, op2, "Identical operations should be equal");

	// Test different operations are not equal
	let col3 = ColumnDefinition {
		name: "id",
		type_definition: FieldType::Integer,
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: false,
		default: None,
	};

	let op3 = Operation::CreateTable {
		name: "posts",
		columns: vec![col3],
		constraints: vec![],
	};

	assert_ne!(op1, op3, "Different operations should not be equal");
}
