//! Tests for AutoMigrationGenerator duplicate detection
//!
//! Tests the duplicate migration detection feature that prevents
//! generating identical migrations multiple times.

use reinhardt_migrations::{
	AutoMigrationError, AutoMigrationGenerator, ColumnDefinition, ColumnSchema, DatabaseSchema,
	Operation, TableSchema,
};
use std::collections::HashMap;

/// Helper to create a simple table schema
fn create_table_schema(table_name: &'static str) -> TableSchema {
	let mut columns = HashMap::new();
	columns.insert(
		"id".to_string(),
		ColumnSchema {
			name: "id",
			data_type: "INTEGER".to_string(),
			nullable: false,
			default: None,
			primary_key: true,
			auto_increment: true,
			max_length: None,
		},
	);
	columns.insert(
		"name".to_string(),
		ColumnSchema {
			name: "name",
			data_type: "VARCHAR(100)".to_string(),
			nullable: false,
			default: None,
			primary_key: false,
			auto_increment: false,
			max_length: Some(100),
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
	let generator = AutoMigrationGenerator::new(target_schema.clone(), temp_dir);

	// First generation should succeed (no previous migration)
	let empty_schema = DatabaseSchema::default();
	let result1 = generator.generate(empty_schema.clone(), None).await;
	assert!(result1.is_ok(), "First generation should succeed");

	let operations = result1.unwrap().operations;
	assert!(!operations.is_empty(), "Should generate operations");

	// Second generation with same operations should fail with DuplicateMigration error
	let result2 = generator
		.generate(empty_schema, Some(operations.clone()))
		.await;

	match result2 {
		Err(AutoMigrationError::DuplicateMigration) => {
			// Expected error
		}
		Err(e) => panic!("Expected DuplicateMigration error, got: {:?}", e),
		Ok(_) => panic!("Expected DuplicateMigration error, but generation succeeded"),
	}
}

#[tokio::test]
async fn test_different_migrations_allowed() {
	// Create target schema with first table
	let mut target_schema1 = DatabaseSchema::default();
	target_schema1
		.tables
		.insert("users".to_string(), create_table_schema("users"));

	let temp_dir = std::env::temp_dir().join("test_different_migrations");
	let generator = AutoMigrationGenerator::new(target_schema1.clone(), temp_dir.clone());

	// First generation
	let empty_schema = DatabaseSchema::default();
	let result1 = generator.generate(empty_schema, None).await;
	assert!(result1.is_ok(), "First generation should succeed");

	let operations1 = result1.unwrap().operations;

	// Create different target schema with additional table
	let mut target_schema2 = DatabaseSchema::default();
	target_schema2
		.tables
		.insert("users".to_string(), create_table_schema("users"));
	target_schema2
		.tables
		.insert("posts".to_string(), create_table_schema("posts"));

	let generator2 = AutoMigrationGenerator::new(target_schema2, temp_dir);

	// Second generation with different operations should succeed
	let result2 = generator2.generate(target_schema1, Some(operations1)).await;

	assert!(
		result2.is_ok(),
		"Generation with different operations should succeed"
	);

	let operations2 = result2.unwrap().operations;
	assert!(!operations2.is_empty(), "Should generate new operations");
}

#[tokio::test]
async fn test_no_changes_error() {
	// Create identical target and current schemas
	let mut schema = DatabaseSchema::default();
	schema
		.tables
		.insert("users".to_string(), create_table_schema("users"));

	let temp_dir = std::env::temp_dir().join("test_no_changes");
	let generator = AutoMigrationGenerator::new(schema.clone(), temp_dir);

	// Should fail with NoChangesDetected error
	let result = generator.generate(schema, None).await;

	match result {
		Err(AutoMigrationError::NoChangesDetected) => {
			// Expected error
		}
		Err(e) => panic!("Expected NoChangesDetected error, got: {:?}", e),
		Ok(_) => panic!("Expected NoChangesDetected error, but generation succeeded"),
	}
}

#[test]
fn test_operation_equality() {
	// Test that identical operations are considered equal
	let col1 = ColumnDefinition {
		name: "id",
		type_definition: "INTEGER",
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: false,
		default: None,
		max_length: None,
	};

	let col2 = ColumnDefinition {
		name: "id",
		type_definition: "INTEGER",
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: false,
		default: None,
		max_length: None,
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
		type_definition: "INTEGER",
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: false,
		default: None,
		max_length: None,
	};

	let op3 = Operation::CreateTable {
		name: "posts",
		columns: vec![col3],
		constraints: vec![],
	};

	assert_ne!(op1, op3, "Different operations should not be equal");
}
