//! Unit tests for introspect generator module
//!
//! Tests for code generation including:
//! - Model struct generation
//! - Field attribute generation
//! - Import statement generation
//! - Single file vs multi-file output
//! - Header comment generation
//!
//! **Test Categories:**
//! - Happy path: Standard model generation
//! - Edge cases: Empty tables, special column names
//! - Property tests: Generated code contains required elements

use reinhardt_migrations::introspection::{
	ColumnInfo, DatabaseSchema, TableInfo, UniqueConstraintInfo,
};
use reinhardt_migrations::{
	FieldType, GeneratedOutput, IntrospectConfig, OutputConfig, SchemaCodeGenerator,
	TableFilterConfig,
};
use rstest::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Create a simple test table with common columns
fn create_users_table() -> TableInfo {
	let mut columns = HashMap::new();

	columns.insert(
		"id".to_string(),
		ColumnInfo {
			name: "id".to_string(),
			column_type: FieldType::BigInteger,
			nullable: false,
			default: None,
			auto_increment: true,
		},
	);

	columns.insert(
		"name".to_string(),
		ColumnInfo {
			name: "name".to_string(),
			column_type: FieldType::VarChar(255),
			nullable: false,
			default: None,
			auto_increment: false,
		},
	);

	columns.insert(
		"email".to_string(),
		ColumnInfo {
			name: "email".to_string(),
			column_type: FieldType::VarChar(255),
			nullable: true,
			default: None,
			auto_increment: false,
		},
	);

	columns.insert(
		"created_at".to_string(),
		ColumnInfo {
			name: "created_at".to_string(),
			column_type: FieldType::TimestampTz,
			nullable: false,
			default: Some("NOW()".to_string()),
			auto_increment: false,
		},
	);

	TableInfo {
		name: "users".to_string(),
		columns,
		indexes: HashMap::new(),
		primary_key: vec!["id".to_string()],
		foreign_keys: vec![],
		unique_constraints: vec![UniqueConstraintInfo {
			name: "users_email_unique".to_string(),
			columns: vec!["email".to_string()],
		}],
		check_constraints: vec![],
	}
}

/// Create a posts table with foreign key
fn create_posts_table() -> TableInfo {
	let mut columns = HashMap::new();

	columns.insert(
		"id".to_string(),
		ColumnInfo {
			name: "id".to_string(),
			column_type: FieldType::BigInteger,
			nullable: false,
			default: None,
			auto_increment: true,
		},
	);

	columns.insert(
		"title".to_string(),
		ColumnInfo {
			name: "title".to_string(),
			column_type: FieldType::VarChar(200),
			nullable: false,
			default: None,
			auto_increment: false,
		},
	);

	columns.insert(
		"content".to_string(),
		ColumnInfo {
			name: "content".to_string(),
			column_type: FieldType::Text,
			nullable: true,
			default: None,
			auto_increment: false,
		},
	);

	columns.insert(
		"user_id".to_string(),
		ColumnInfo {
			name: "user_id".to_string(),
			column_type: FieldType::BigInteger,
			nullable: false,
			default: None,
			auto_increment: false,
		},
	);

	TableInfo {
		name: "posts".to_string(),
		columns,
		indexes: HashMap::new(),
		primary_key: vec!["id".to_string()],
		foreign_keys: vec![],
		unique_constraints: vec![],
		check_constraints: vec![],
	}
}

/// Create a test schema with multiple tables
fn create_test_schema() -> DatabaseSchema {
	let mut tables = HashMap::new();
	tables.insert("users".to_string(), create_users_table());
	tables.insert("posts".to_string(), create_posts_table());

	DatabaseSchema { tables }
}

/// Create a minimal config for testing
fn create_test_config() -> IntrospectConfig {
	IntrospectConfig::default()
		.with_database_url("postgres://test@localhost/test")
		.with_app_label("test_app")
}

// ============================================================================
// Happy Path Tests: Model Generation
// ============================================================================

/// Test generating a simple model
///
/// **Test Intent**: Verify basic model generation produces valid code
#[rstest]
#[test]
fn test_generate_simple_model() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	// Should have 2 files: users.rs and mod.rs
	assert_eq!(output.files.len(), 2);

	// Find the users.rs file
	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Verify content contains expected elements
	assert!(users_file.content.contains("pub struct Users"));
	assert!(users_file.content.contains("#[model"));
	assert!(users_file.content.contains("app_label"));
	assert!(users_file.content.contains("table_name"));
}

/// Test model contains correct field types
///
/// **Test Intent**: Verify field types are mapped correctly
#[rstest]
#[test]
fn test_model_field_types() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Check field types
	assert!(
		users_file.content.contains("pub id: i64"),
		"Should have i64 id field"
	);
	assert!(
		users_file.content.contains("pub name: String"),
		"Should have String name field"
	);
	assert!(
		users_file.content.contains("pub email: Option<String>"),
		"Should have Option<String> for nullable email"
	);
}

/// Test model has derive macros
///
/// **Test Intent**: Verify derive macros are included
#[rstest]
#[test]
fn test_model_derives() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Check derives
	assert!(users_file.content.contains("Debug"));
	assert!(users_file.content.contains("Clone"));
	assert!(users_file.content.contains("Serialize"));
	assert!(users_file.content.contains("Deserialize"));
}

/// Test multiple tables generate separate files
///
/// **Test Intent**: Verify multi-file output mode
#[rstest]
#[test]
fn test_multiple_tables_separate_files() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	// Should have 3 files: users.rs, posts.rs, mod.rs
	assert_eq!(output.files.len(), 3);

	// Check file names
	let file_names: Vec<_> = output
		.files
		.iter()
		.map(|f| f.path.file_name().unwrap().to_str().unwrap())
		.collect();

	assert!(file_names.contains(&"users.rs"));
	assert!(file_names.contains(&"posts.rs"));
	assert!(file_names.contains(&"mod.rs"));
}

/// Test mod.rs contains re-exports
///
/// **Test Intent**: Verify mod.rs exports all models
#[rstest]
#[test]
fn test_mod_file_exports() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	let mod_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "mod.rs")
		.unwrap();

	// Should have module declarations
	assert!(mod_file.content.contains("pub mod users"));
	assert!(mod_file.content.contains("pub mod posts"));

	// Should have re-exports
	assert!(mod_file.content.contains("pub use users::Users"));
	assert!(mod_file.content.contains("pub use posts::Posts"));
}

// ============================================================================
// Single File Mode Tests
// ============================================================================

/// Test single file mode generates one file
///
/// **Test Intent**: Verify single_file option works
#[rstest]
#[test]
fn test_single_file_mode() {
	let config = IntrospectConfig {
		output: OutputConfig {
			directory: PathBuf::from("src/models"),
			single_file: true,
			single_file_name: "all_models.rs".to_string(),
		},
		..create_test_config()
	};

	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	// Should have only 1 file
	assert_eq!(output.files.len(), 1);

	let file = &output.files[0];
	assert_eq!(file.path.file_name().unwrap(), "all_models.rs");

	// Should contain both models
	assert!(file.content.contains("pub struct Users"));
	assert!(file.content.contains("pub struct Posts"));
}

// ============================================================================
// Field Attribute Tests
// ============================================================================

/// Test primary key attribute generation
///
/// **Test Intent**: Verify primary_key attribute is set
#[rstest]
#[test]
fn test_primary_key_attribute() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Should have primary_key and auto_increment
	assert!(users_file.content.contains("primary_key = true"));
	assert!(users_file.content.contains("auto_increment = true"));
}

/// Test unique attribute generation
///
/// **Test Intent**: Verify unique attribute is set for unique columns
#[rstest]
#[test]
fn test_unique_attribute() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Email should have unique = true
	assert!(users_file.content.contains("unique = true"));
}

/// Test max_length attribute for VarChar
///
/// **Test Intent**: Verify max_length is set for VarChar fields
#[rstest]
#[test]
fn test_max_length_attribute() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Should have max_length = 255
	assert!(users_file.content.contains("max_length = 255"));
}

// ============================================================================
// Table Filtering Tests
// ============================================================================

/// Test table filtering excludes specified tables
///
/// **Test Intent**: Verify exclude patterns filter tables
#[rstest]
#[test]
fn test_table_filtering_exclude() {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec![".*".to_string()],
			exclude: vec!["posts".to_string()],
		},
		..create_test_config()
	};

	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	// Should only have users.rs and mod.rs
	let file_names: Vec<_> = output
		.files
		.iter()
		.map(|f| f.path.file_name().unwrap().to_str().unwrap())
		.collect();

	assert!(file_names.contains(&"users.rs"));
	assert!(!file_names.contains(&"posts.rs"));
}

/// Test table filtering includes only specified tables
///
/// **Test Intent**: Verify include patterns filter tables
#[rstest]
#[test]
fn test_table_filtering_include() {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec!["users".to_string()],
			exclude: vec![],
		},
		..create_test_config()
	};

	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	// Should only have users.rs and mod.rs
	let file_names: Vec<_> = output
		.files
		.iter()
		.map(|f| f.path.file_name().unwrap().to_str().unwrap())
		.collect();

	assert!(file_names.contains(&"users.rs"));
	assert!(!file_names.contains(&"posts.rs"));
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test empty schema produces empty output
///
/// **Test Intent**: Verify empty schema is handled gracefully
#[rstest]
#[test]
fn test_empty_schema() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let schema = DatabaseSchema {
		tables: HashMap::new(),
	};

	let output = generator.generate(&schema).unwrap();

	// Should have no files
	assert!(output.files.is_empty());
}

/// Test all tables filtered out
///
/// **Test Intent**: Verify graceful handling when all tables are excluded
#[rstest]
#[test]
fn test_all_tables_filtered() {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec!["nonexistent".to_string()],
			exclude: vec![],
		},
		..create_test_config()
	};

	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	// Should have no files
	assert!(output.files.is_empty());
}

/// Test table with reserved Rust keyword name
///
/// **Test Intent**: Verify reserved keywords are escaped
#[rstest]
#[test]
fn test_reserved_keyword_table_name() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut columns = HashMap::new();
	columns.insert(
		"id".to_string(),
		ColumnInfo {
			name: "id".to_string(),
			column_type: FieldType::BigInteger,
			nullable: false,
			default: None,
			auto_increment: true,
		},
	);
	columns.insert(
		"type".to_string(), // Reserved keyword
		ColumnInfo {
			name: "type".to_string(),
			column_type: FieldType::VarChar(50),
			nullable: false,
			default: None,
			auto_increment: false,
		},
	);

	let table = TableInfo {
		name: "items".to_string(),
		columns,
		indexes: HashMap::new(),
		primary_key: vec!["id".to_string()],
		foreign_keys: vec![],
		unique_constraints: vec![],
		check_constraints: vec![],
	};

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema.tables.insert("items".to_string(), table);

	let output = generator.generate(&schema).unwrap();

	let items_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "items.rs")
		.unwrap();

	// Field should be escaped with r#
	assert!(items_file.content.contains("r#type"));
}

// ============================================================================
// Header and Imports Tests
// ============================================================================

/// Test generated file has header comment
///
/// **Test Intent**: Verify header comment is included
#[rstest]
#[test]
fn test_header_comment() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Should have header comment
	assert!(users_file.content.contains("Generated by"));
	assert!(users_file.content.contains("DO NOT EDIT"));
}

/// Test generated file has imports
///
/// **Test Intent**: Verify import statements are included
#[rstest]
#[test]
fn test_imports_included() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Should have reinhardt prelude import
	assert!(users_file.content.contains("use reinhardt::prelude"));
	// Should have serde imports
	assert!(users_file.content.contains("use serde"));
	// Should have chrono imports for timestamp fields
	assert!(users_file.content.contains("use chrono"));
}

/// Test password is masked in header
///
/// **Test Intent**: Verify database password is not exposed
#[rstest]
#[test]
fn test_password_masked_in_header() {
	let config = IntrospectConfig::default()
		.with_database_url("postgres://user:secret@localhost/db")
		.with_app_label("test");

	let generator = SchemaCodeGenerator::new(config);

	let mut schema = DatabaseSchema {
		tables: HashMap::new(),
	};
	schema
		.tables
		.insert("users".to_string(), create_users_table());

	let output = generator.generate(&schema).unwrap();

	let users_file = output
		.files
		.iter()
		.find(|f| f.path.file_name().unwrap() == "users.rs")
		.unwrap();

	// Should NOT contain password
	assert!(!users_file.content.contains("secret"));
	// Should contain masked password
	assert!(users_file.content.contains("****"));
}

// ============================================================================
// GeneratedOutput Tests
// ============================================================================

/// Test GeneratedOutput default
///
/// **Test Intent**: Verify default output is empty
#[rstest]
#[test]
fn test_generated_output_default() {
	let output = GeneratedOutput::default();
	assert!(output.files.is_empty());
}

/// Test GeneratedOutput::add_file
///
/// **Test Intent**: Verify files can be added
#[rstest]
#[test]
fn test_generated_output_add_file() {
	let mut output = GeneratedOutput::new();

	output.add_file(reinhardt_migrations::GeneratedFile::new(
		PathBuf::from("test.rs"),
		"// test content",
	));

	assert_eq!(output.files.len(), 1);
	assert_eq!(output.files[0].path, PathBuf::from("test.rs"));
}

// ============================================================================
// Property Tests: Generated Code Requirements
// ============================================================================

/// Test all generated model files contain struct keyword
///
/// **Test Intent**: Verify all model files have struct definitions
#[rstest]
#[test]
fn test_all_files_have_struct() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	for file in &output.files {
		if file.path.file_name().unwrap() != "mod.rs" {
			assert!(
				file.content.contains("pub struct"),
				"File {} should contain struct definition",
				file.path.display()
			);
		}
	}
}

/// Test all generated model files contain model attribute
///
/// **Test Intent**: Verify all model files have #[model] attribute
#[rstest]
#[test]
fn test_all_files_have_model_attribute() {
	let config = create_test_config();
	let generator = SchemaCodeGenerator::new(config);
	let schema = create_test_schema();

	let output = generator.generate(&schema).unwrap();

	for file in &output.files {
		if file.path.file_name().unwrap() != "mod.rs" {
			assert!(
				file.content.contains("#[model"),
				"File {} should contain #[model] attribute",
				file.path.display()
			);
		}
	}
}
