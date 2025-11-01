//! Tests for migration writer
//! Translated from Django's test_writer.py

use reinhardt_migrations::{ColumnDefinition, Migration, MigrationWriter, Operation};

#[test]
fn test_write_simple_migration() {
	// Test writing a simple migration with one operation
	let migration =
		Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
			name: "users".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
				ColumnDefinition::new("username", "VARCHAR(150) NOT NULL"),
			],
			constraints: vec![],
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	// Verify generated content
	assert!(
		content.contains("0001_initial"),
		"Migration name '0001_initial' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("testapp"),
		"App name 'testapp' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("CreateTable"),
		"Operation::CreateTable not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("users"),
		"Table name 'users' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("id"),
		"Column name 'id' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("username"),
		"Column name 'username' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("INTEGER PRIMARY KEY"),
		"Column definition 'INTEGER PRIMARY KEY' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("VARCHAR(150) NOT NULL"),
		"Column definition 'VARCHAR(150) NOT NULL' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_write_add_column_migration() {
	// Test writing a migration that adds a column
	let migration =
		Migration::new("0002_add_email", "testapp").add_operation(Operation::AddColumn {
			table: "users".to_string(),
			column: ColumnDefinition::new("email", "VARCHAR(255)"),
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	assert!(
		content.contains("0002_add_email"),
		"Migration name '0002_add_email' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("AddColumn"),
		"Operation::AddColumn not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("email"),
		"Column name 'email' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("VARCHAR(255)"),
		"Column definition 'VARCHAR(255)' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_write_drop_column_migration() {
	// Test writing a migration that drops a column
	let migration =
		Migration::new("0003_remove_email", "testapp").add_operation(Operation::DropColumn {
			table: "users".to_string(),
			column: "email".to_string(),
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	assert!(
		content.contains("0003_remove_email"),
		"Migration name '0003_remove_email' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("DropColumn"),
		"Operation::DropColumn not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("email"),
		"Column name 'email' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_write_alter_column_migration() {
	// Test writing a migration that alters a column
	let migration =
		Migration::new("0004_alter_username", "testapp").add_operation(Operation::AlterColumn {
			table: "users".to_string(),
			column: "username".to_string(),
			new_definition: ColumnDefinition::new("username", "VARCHAR(200) NOT NULL"),
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	assert!(
		content.contains("0004_alter_username"),
		"Migration name '0004_alter_username' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("AlterColumn"),
		"Operation::AlterColumn not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("username"),
		"Column name 'username' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("VARCHAR(200) NOT NULL"),
		"Column definition 'VARCHAR(200) NOT NULL' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_write_drop_table_migration() {
	// Test writing a migration that drops a table
	let migration =
		Migration::new("0005_delete_users", "testapp").add_operation(Operation::DropTable {
			name: "users".to_string(),
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	assert!(
		content.contains("0005_delete_users"),
		"Migration name '0005_delete_users' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("DropTable"),
		"Operation::DropTable not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("users"),
		"Table name 'users' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_write_migration_with_dependencies() {
	// Test writing a migration with dependencies
	let migration = Migration::new("0002_add_profile", "users")
		.add_dependency("auth", "0001_initial")
		.add_operation(Operation::CreateTable {
			name: "profile".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
				ColumnDefinition::new("user_id", "INTEGER NOT NULL"),
			],
			constraints: vec![],
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	assert!(
		content.contains("0002_add_profile"),
		"Migration name '0002_add_profile' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("add_dependency"),
		"Dependency method 'add_dependency' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("auth"),
		"Dependency app 'auth' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("0001_initial"),
		"Dependency migration '0001_initial' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_write_migration_with_multiple_operations() {
	// Test writing a migration with multiple operations
	let migration = Migration::new("0006_complex", "testapp")
		.add_operation(Operation::CreateTable {
			name: "categories".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
				ColumnDefinition::new("name", "VARCHAR(100) NOT NULL"),
			],
			constraints: vec![],
		})
		.add_operation(Operation::AddColumn {
			table: "users".to_string(),
			column: ColumnDefinition::new("category_id", "INTEGER"),
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	assert!(
		content.contains("0006_complex"),
		"Migration name '0006_complex' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("CreateTable"),
		"Operation::CreateTable not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("categories"),
		"Table name 'categories' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("AddColumn"),
		"Operation::AddColumn not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("category_id"),
		"Column name 'category_id' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_migration_file_format() {
	// Test that the generated migration file has correct format
	let migration = Migration::new("0001_initial", "myapp").add_operation(Operation::CreateTable {
		name: "test_table".to_string(),
		columns: vec![ColumnDefinition::new("id", "INTEGER")],
		constraints: vec![],
	});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	// Check file header
	assert!(
		content.contains("//! Auto-generated migration"),
		"File header '//! Auto-generated migration' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("//! Name: 0001_initial"),
		"Migration name header '//! Name: 0001_initial' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("//! App: myapp"),
		"App name header '//! App: myapp' not found in generated code.\nGenerated code:\n{}",
		content
	);

	// Check imports
	assert!(
		content.contains("use reinhardt_migrations"),
		"Import statement 'use reinhardt_migrations' not found in generated code.\nGenerated code:\n{}",
		content
	);

	// Check function definition
	assert!(
		content.contains("pub fn migration_0001_initial() -> Migration"),
		"Function definition 'pub fn migration_0001_initial() -> Migration' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("Migration::new(\"0001_initial\", \"myapp\")"),
		"Migration initialization code 'Migration::new(\"0001_initial\", \"myapp\")' not found in generated code.\nGenerated code:\n{}",
		content
	);
}

#[test]
fn test_write_to_file() {
	// Test writing migration to actual file
	let migration =
		Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
			name: "test".to_string(),
			columns: vec![ColumnDefinition::new("id", "INTEGER")],
			constraints: vec![],
		});

	let temp_dir = std::env::temp_dir().join("reinhardt_test_migrations");
	std::fs::create_dir_all(&temp_dir).unwrap();

	let writer = MigrationWriter::new(migration);
	let filepath = writer.write_to_file(&temp_dir).unwrap();

	// Verify file was created
	assert!(
		std::path::Path::new(&filepath).exists(),
		"Migration file was not created: {}",
		filepath
	);

	// Verify file content
	let content = std::fs::read_to_string(&filepath).unwrap();
	assert!(
		content.contains("0001_initial"),
		"File content does not contain migration name '0001_initial'.\nFile path: {}\nContent:\n{}",
		filepath,
		content
	);
	assert!(
		content.contains("testapp"),
		"File content does not contain app name 'testapp'.\nFile path: {}\nContent:\n{}",
		filepath,
		content
	);

	// Cleanup
	std::fs::remove_file(&filepath).unwrap();
}

#[test]
fn test_serialization_indentation() {
	// Test that the serialization maintains proper indentation
	let migration =
		Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
			name: "users".to_string(),
			columns: vec![
				ColumnDefinition::new("id", "INTEGER"),
				ColumnDefinition::new("name", "VARCHAR(100)"),
			],
			constraints: vec![],
		});

	let writer = MigrationWriter::new(migration);
	let content = writer.as_string();

	// Check that proper indentation is maintained
	assert!(
		content.contains("    .add_operation"),
		"Indentation '    .add_operation' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("        name:"),
		"Indentation '        name:' not found in generated code.\nGenerated code:\n{}",
		content
	);
	assert!(
		content.contains("        columns: vec!["),
		"Indentation '        columns: vec![' not found in generated code.\nGenerated code:\n{}",
		content
	);
}
