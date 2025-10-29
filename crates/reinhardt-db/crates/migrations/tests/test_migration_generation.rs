//! Integration tests for migration generation from model changes
//!
//! This test suite validates the complete flow of:
//! 1. Detecting model changes
//! 2. Generating migration operations
//! 3. Writing migration files
//!
//! # Django Reference
//! Tests inspired by Django's migration system testing approach:
//! - django/tests/migrations/test_autodetector.py
//! - django/tests/migrations/test_writer.py

use reinhardt_migrations::{
    FieldState, MakeMigrationsCommand, MakeMigrationsOptions, MigrationAutodetector, ModelState,
    ProjectState,
};
use std::fs;
use std::path::PathBuf;

/// Helper function to create a simple field
fn field(name: &str, field_type: &str, nullable: bool) -> FieldState {
    FieldState::new(name.to_string(), field_type.to_string(), nullable)
}

/// Test generating migrations for a new model
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_autodetector.py::AutodetectorTests::test_create_model
#[test]
fn test_generate_migration_for_new_model() {
    // Setup: Create empty from_state and to_state with a new model
    let from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Add a new User model
    let mut user_model = ModelState::new("users", "User");
    user_model.add_field(field("id", "INTEGER", false));
    user_model.add_field(field("email", "VARCHAR(255)", false));
    user_model.add_field(field("name", "VARCHAR(100)", false));
    to_state.add_model(user_model);

    // Run autodetector
    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let migrations = autodetector.generate_migrations();

    // Verify
    assert_eq!(migrations.len(), 1, "Should generate 1 migration");
    let migration = &migrations[0];
    assert_eq!(migration.app_label, "users");
    assert_eq!(migration.operations.len(), 1, "Should have 1 operation");

    // Verify operation is CreateTable
    match &migration.operations[0] {
        reinhardt_migrations::Operation::CreateTable { name, columns, .. } => {
            assert_eq!(name, "User");
            assert_eq!(columns.len(), 3, "Should have 3 columns");
        }
        _ => panic!("Expected CreateTable operation"),
    }
}

/// Test generating migrations for adding a field
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_autodetector.py::AutodetectorTests::test_add_field
#[test]
fn test_generate_migration_for_added_field() {
    // Setup: Create from_state with a model and to_state with an added field
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Initial model state (from)
    let mut user_model_old = ModelState::new("users", "User");
    user_model_old.add_field(field("id", "INTEGER", false));
    user_model_old.add_field(field("email", "VARCHAR(255)", false));
    from_state.add_model(user_model_old);

    // New model state with added field (to)
    let mut user_model_new = ModelState::new("users", "User");
    user_model_new.add_field(field("id", "INTEGER", false));
    user_model_new.add_field(field("email", "VARCHAR(255)", false));
    user_model_new.add_field(field("name", "VARCHAR(100)", false)); // New field
    to_state.add_model(user_model_new);

    // Run autodetector
    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify
    assert_eq!(changes.added_fields.len(), 1, "Should detect 1 added field");
    assert_eq!(changes.added_fields[0].0, "users");
    assert_eq!(changes.added_fields[0].1, "User");
    assert_eq!(changes.added_fields[0].2, "name");

    // Generate operations
    let operations = autodetector.generate_operations();
    assert_eq!(operations.len(), 1, "Should generate 1 operation");

    // Verify operation is AddColumn
    match &operations[0] {
        reinhardt_migrations::Operation::AddColumn { table, column } => {
            assert_eq!(table, "User");
            assert_eq!(column.name, "name");
        }
        _ => panic!("Expected AddColumn operation"),
    }
}

/// Test generating migrations for removing a field
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_autodetector.py::AutodetectorTests::test_remove_field
#[test]
fn test_generate_migration_for_removed_field() {
    // Setup: Create from_state with a field and to_state without it
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Initial model state with field to be removed (from)
    let mut user_model_old = ModelState::new("users", "User");
    user_model_old.add_field(field("id", "INTEGER", false));
    user_model_old.add_field(field("email", "VARCHAR(255)", false));
    user_model_old.add_field(field("old_field", "VARCHAR(50)", false)); // To be removed
    from_state.add_model(user_model_old);

    // New model state without the field (to)
    let mut user_model_new = ModelState::new("users", "User");
    user_model_new.add_field(field("id", "INTEGER", false));
    user_model_new.add_field(field("email", "VARCHAR(255)", false));
    to_state.add_model(user_model_new);

    // Run autodetector
    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify
    assert_eq!(
        changes.removed_fields.len(),
        1,
        "Should detect 1 removed field"
    );
    assert_eq!(changes.removed_fields[0].0, "users");
    assert_eq!(changes.removed_fields[0].1, "User");
    assert_eq!(changes.removed_fields[0].2, "old_field");

    // Generate operations
    let operations = autodetector.generate_operations();
    assert_eq!(operations.len(), 1, "Should generate 1 operation");

    // Verify operation is DropColumn
    match &operations[0] {
        reinhardt_migrations::Operation::DropColumn { table, column } => {
            assert_eq!(table, "User");
            assert_eq!(column, "old_field");
        }
        _ => panic!("Expected DropColumn operation"),
    }
}

/// Test generating migrations for altering a field
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_autodetector.py::AutodetectorTests::test_alter_field
#[test]
fn test_generate_migration_for_altered_field() {
    // Setup: Create from_state and to_state with different field definitions
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Initial model state with original field type (from)
    let mut user_model_old = ModelState::new("users", "User");
    user_model_old.add_field(field("id", "INTEGER", false));
    user_model_old.add_field(field("email", "VARCHAR(100)", false)); // Original type
    from_state.add_model(user_model_old);

    // New model state with altered field type (to)
    let mut user_model_new = ModelState::new("users", "User");
    user_model_new.add_field(field("id", "INTEGER", false));
    user_model_new.add_field(field("email", "VARCHAR(255)", false)); // Altered type
    to_state.add_model(user_model_new);

    // Run autodetector
    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify
    assert_eq!(
        changes.altered_fields.len(),
        1,
        "Should detect 1 altered field"
    );
    assert_eq!(changes.altered_fields[0].0, "users");
    assert_eq!(changes.altered_fields[0].1, "User");
    assert_eq!(changes.altered_fields[0].2, "email");

    // Generate operations
    let operations = autodetector.generate_operations();
    assert_eq!(operations.len(), 1, "Should generate 1 operation");

    // Verify operation is AlterColumn
    match &operations[0] {
        reinhardt_migrations::Operation::AlterColumn {
            table,
            column,
            new_definition,
        } => {
            assert_eq!(table, "User");
            assert_eq!(column, "email");
            assert_eq!(new_definition.type_definition, "VARCHAR(255)");
        }
        _ => panic!("Expected AlterColumn operation"),
    }
}

/// Test generating migrations for deleting a model
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_autodetector.py::AutodetectorTests::test_delete_model
#[test]
fn test_generate_migration_for_deleted_model() {
    // Setup: Create from_state with a model and empty to_state
    let mut from_state = ProjectState::new();
    let to_state = ProjectState::new();

    // Model to be deleted (from)
    let mut user_model = ModelState::new("users", "User");
    user_model.add_field(field("id", "INTEGER", false));
    user_model.add_field(field("email", "VARCHAR(255)", false));
    from_state.add_model(user_model);

    // Run autodetector
    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify
    assert_eq!(
        changes.deleted_models.len(),
        1,
        "Should detect 1 deleted model"
    );
    assert_eq!(changes.deleted_models[0].0, "users");
    assert_eq!(changes.deleted_models[0].1, "User");

    // Generate operations
    let operations = autodetector.generate_operations();
    assert_eq!(operations.len(), 1, "Should generate 1 operation");

    // Verify operation is DropTable
    match &operations[0] {
        reinhardt_migrations::Operation::DropTable { name } => {
            assert_eq!(name, "User");
        }
        _ => panic!("Expected DropTable operation"),
    }
}

/// Test makemigrations command dry-run mode
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_commands.py::MakeMigrationsTests::test_makemigrations_dry_run
#[test]
fn test_makemigrations_dry_run() {
    // Setup
    let options = MakeMigrationsOptions {
        dry_run: true,
        migrations_dir: "/tmp/test_migrations_dry_run".to_string(),
        ..Default::default()
    };

    let command = MakeMigrationsCommand::new(options);

    // Execute (no models registered, should show "No changes detected")
    let files = command.execute();

    // Verify: No files should be created in dry-run mode
    assert!(
        files.is_empty() || !PathBuf::from(&files[0]).exists(),
        "Dry-run should not create actual files"
    );
}

/// Test migration file writing
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_writer.py::WriterTests
#[test]
fn test_write_migration_file() {
    use reinhardt_migrations::{Migration, MigrationWriter, Operation};
    use std::fs;
    use std::path::PathBuf;

    // Setup
    let test_dir = PathBuf::from("/tmp/test_write_migration");
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).ok();
    }
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    // Create a migration
    let migration =
        Migration::new("0001_initial", "testapp").add_operation(Operation::CreateTable {
            name: "Test".to_string(),
            columns: vec![],
            constraints: vec![],
        });

    // Write migration
    let writer = MigrationWriter::new(migration);
    let filepath = writer
        .write_to_file(&test_dir)
        .expect("Failed to write migration file");

    // Verify
    assert!(
        PathBuf::from(&filepath).exists(),
        "Migration file should exist"
    );
    assert!(
        filepath.ends_with("0001_initial.rs"),
        "Migration file should have correct name"
    );

    // Read and verify content
    let content = fs::read_to_string(&filepath).expect("Failed to read migration file");
    assert!(
        content.contains("Migration::new"),
        "Content should contain Migration::new"
    );
    assert!(
        content.contains("0001_initial"),
        "Content should contain migration name"
    );
    assert!(
        content.contains("testapp"),
        "Content should contain app label"
    );

    // Cleanup
    fs::remove_dir_all(&test_dir).ok();
}

/// Test complex model changes (multiple operations)
///
/// # Django Reference
/// Similar to: django/tests/migrations/test_autodetector.py::AutodetectorTests::test_multiple_changes
#[test]
fn test_generate_migration_for_multiple_changes() {
    // Setup: Create from_state and to_state with multiple changes
    let mut from_state = ProjectState::new();
    let mut to_state = ProjectState::new();

    // Initial model state (from)
    let mut user_model_old = ModelState::new("users", "User");
    user_model_old.add_field(field("id", "INTEGER", false));
    user_model_old.add_field(field("email", "VARCHAR(100)", false));
    user_model_old.add_field(field("old_field", "VARCHAR(50)", false));
    from_state.add_model(user_model_old);

    // New model state with multiple changes (to)
    let mut user_model_new = ModelState::new("users", "User");
    user_model_new.add_field(field("id", "INTEGER", false));
    user_model_new.add_field(field("email", "VARCHAR(255)", false)); // Altered
    user_model_new.add_field(field("name", "VARCHAR(100)", false)); // Added (old_field removed)

    to_state.add_model(user_model_new);

    // Run autodetector
    let autodetector = MigrationAutodetector::new(from_state, to_state);
    let changes = autodetector.detect_changes();

    // Verify multiple changes detected
    assert_eq!(changes.added_fields.len(), 1, "Should detect 1 added field");
    assert_eq!(
        changes.removed_fields.len(),
        1,
        "Should detect 1 removed field"
    );
    assert_eq!(
        changes.altered_fields.len(),
        1,
        "Should detect 1 altered field"
    );

    // Generate operations
    let operations = autodetector.generate_operations();
    assert!(
        operations.len() >= 3,
        "Should generate at least 3 operations"
    );
}
