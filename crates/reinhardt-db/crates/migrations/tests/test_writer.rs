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
                ColumnDefinition {
                    name: "id".to_string(),
                    type_definition: "INTEGER PRIMARY KEY".to_string(),
                },
                ColumnDefinition {
                    name: "username".to_string(),
                    type_definition: "VARCHAR(150) NOT NULL".to_string(),
                },
            ],
            constraints: vec![],
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    // Verify generated content
    assert!(content.contains("0001_initial"));
    assert!(content.contains("testapp"));
    assert!(content.contains("CreateTable"));
    assert!(content.contains("users"));
    assert!(content.contains("id"));
    assert!(content.contains("username"));
    assert!(content.contains("INTEGER PRIMARY KEY"));
    assert!(content.contains("VARCHAR(150) NOT NULL"));
}

#[test]
fn test_write_add_column_migration() {
    // Test writing a migration that adds a column
    let migration =
        Migration::new("0002_add_email", "testapp").add_operation(Operation::AddColumn {
            table: "users".to_string(),
            column: ColumnDefinition {
                name: "email".to_string(),
                type_definition: "VARCHAR(255)".to_string(),
            },
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(content.contains("0002_add_email"));
    assert!(content.contains("AddColumn"));
    assert!(content.contains("email"));
    assert!(content.contains("VARCHAR(255)"));
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

    assert!(content.contains("0003_remove_email"));
    assert!(content.contains("DropColumn"));
    assert!(content.contains("email"));
}

#[test]
fn test_write_alter_column_migration() {
    // Test writing a migration that alters a column
    let migration =
        Migration::new("0004_alter_username", "testapp").add_operation(Operation::AlterColumn {
            table: "users".to_string(),
            column: "username".to_string(),
            new_definition: ColumnDefinition {
                name: "username".to_string(),
                type_definition: "VARCHAR(200) NOT NULL".to_string(),
            },
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(content.contains("0004_alter_username"));
    assert!(content.contains("AlterColumn"));
    assert!(content.contains("username"));
    assert!(content.contains("VARCHAR(200) NOT NULL"));
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

    assert!(content.contains("0005_delete_users"));
    assert!(content.contains("DropTable"));
    assert!(content.contains("users"));
}

#[test]
fn test_write_migration_with_dependencies() {
    // Test writing a migration with dependencies
    let migration = Migration::new("0002_add_profile", "users")
        .add_dependency("auth", "0001_initial")
        .add_operation(Operation::CreateTable {
            name: "profile".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    type_definition: "INTEGER PRIMARY KEY".to_string(),
                },
                ColumnDefinition {
                    name: "user_id".to_string(),
                    type_definition: "INTEGER NOT NULL".to_string(),
                },
            ],
            constraints: vec![],
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(content.contains("0002_add_profile"));
    assert!(content.contains("add_dependency"));
    assert!(content.contains("auth"));
    assert!(content.contains("0001_initial"));
}

#[test]
fn test_write_migration_with_multiple_operations() {
    // Test writing a migration with multiple operations
    let migration = Migration::new("0006_complex", "testapp")
        .add_operation(Operation::CreateTable {
            name: "categories".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    type_definition: "INTEGER PRIMARY KEY".to_string(),
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    type_definition: "VARCHAR(100) NOT NULL".to_string(),
                },
            ],
            constraints: vec![],
        })
        .add_operation(Operation::AddColumn {
            table: "users".to_string(),
            column: ColumnDefinition {
                name: "category_id".to_string(),
                type_definition: "INTEGER".to_string(),
            },
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    assert!(content.contains("0006_complex"));
    assert!(content.contains("CreateTable"));
    assert!(content.contains("categories"));
    assert!(content.contains("AddColumn"));
    assert!(content.contains("category_id"));
}

#[test]
fn test_migration_file_format() {
    // Test that the generated migration file has correct format
    let migration = Migration::new("0001_initial", "myapp").add_operation(Operation::CreateTable {
        name: "test_table".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            type_definition: "INTEGER".to_string(),
        }],
        constraints: vec![],
    });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    // Check file header
    assert!(content.contains("//! Auto-generated migration"));
    assert!(content.contains("//! Name: 0001_initial"));
    assert!(content.contains("//! App: myapp"));

    // Check imports
    assert!(content.contains("use reinhardt_migrations"));

    // Check function definition
    assert!(content.contains("pub fn migration_0001_initial() -> Migration"));
    assert!(content.contains("Migration::new(\"0001_initial\", \"myapp\")"));
}

#[test]
fn test_write_to_file() {
    // Test writing migration to actual file
    let migration =
        Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                type_definition: "INTEGER".to_string(),
            }],
            constraints: vec![],
        });

    let temp_dir = std::env::temp_dir().join("reinhardt_test_migrations");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let writer = MigrationWriter::new(migration);
    let filepath = writer.write_to_file(&temp_dir).unwrap();

    // Verify file was created
    assert!(std::path::Path::new(&filepath).exists());

    // Verify file content
    let content = std::fs::read_to_string(&filepath).unwrap();
    assert!(content.contains("0001_initial"));
    assert!(content.contains("testapp"));

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
                ColumnDefinition {
                    name: "id".to_string(),
                    type_definition: "INTEGER".to_string(),
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    type_definition: "VARCHAR(100)".to_string(),
                },
            ],
            constraints: vec![],
        });

    let writer = MigrationWriter::new(migration);
    let content = writer.as_string();

    // Check that proper indentation is maintained
    assert!(content.contains("    .add_operation"));
    assert!(content.contains("        name:"));
    assert!(content.contains("        columns: vec!["));
}
