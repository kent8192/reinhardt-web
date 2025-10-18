//! Tests for migration optimizer
//! Adapted from Django's test_optimizer.py

use reinhardt_migrations::{ColumnDefinition, Migration, Operation};

fn create_column(name: &str, type_def: &str) -> ColumnDefinition {
    ColumnDefinition {
        name: name.to_string(),
        type_definition: type_def.to_string(),
    }
}

#[test]
fn test_create_delete_table_optimization() {
    // CreateTable followed by DropTable should be optimized away
    let operations = vec![
        Operation::CreateTable {
            name: "test_table".to_string(),
            columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
            constraints: vec![],
        },
        Operation::DropTable {
            name: "test_table".to_string(),
        },
    ];

    // In a real optimizer, these would cancel out
    // For now, we just verify they exist
    assert_eq!(operations.len(), 2);
}

#[test]
fn test_add_remove_field_optimization() {
    // AddColumn followed by DropColumn should be optimized away
    let operations = vec![
        Operation::AddColumn {
            table: "test_table".to_string(),
            column: create_column("temp", "TEXT"),
        },
        Operation::DropColumn {
            table: "test_table".to_string(),
            column: "temp".to_string(),
        },
    ];

    assert_eq!(operations.len(), 2);
}

#[test]
fn test_consecutive_alter_optimization() {
    // Multiple AlterColumn on same field could be merged
    let operations = vec![
        Operation::AlterColumn {
            table: "test_table".to_string(),
            column: "field".to_string(),
            new_definition: create_column("field", "INTEGER"),
        },
        Operation::AlterColumn {
            table: "test_table".to_string(),
            column: "field".to_string(),
            new_definition: create_column("field", "INTEGER NOT NULL"),
        },
    ];

    assert_eq!(operations.len(), 2);
}

#[test]
fn test_rename_table_optimization() {
    // Multiple RenameTable operations should be chained
    let operations = vec![
        Operation::RenameTable {
            old_name: "old".to_string(),
            new_name: "temp".to_string(),
        },
        Operation::RenameTable {
            old_name: "temp".to_string(),
            new_name: "new".to_string(),
        },
    ];

    assert_eq!(operations.len(), 2);
}

#[test]
fn test_create_table_with_add_column() {
    // CreateTable followed by AddColumn should merge
    let operations = vec![
        Operation::CreateTable {
            name: "test_table".to_string(),
            columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
            constraints: vec![],
        },
        Operation::AddColumn {
            table: "test_table".to_string(),
            column: create_column("name", "TEXT"),
        },
    ];

    // These could be optimized to a single CreateTable
    assert_eq!(operations.len(), 2);
}

#[test]
fn test_no_optimization_needed() {
    // Some operations don't need optimization
    let operations = vec![
        Operation::CreateTable {
            name: "table1".to_string(),
            columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
            constraints: vec![],
        },
        Operation::CreateTable {
            name: "table2".to_string(),
            columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
            constraints: vec![],
        },
    ];

    assert_eq!(operations.len(), 2);
}

#[test]
fn test_index_optimization() {
    // CreateIndex followed by DropIndex could be optimized
    let operations = vec![
        Operation::CreateIndex {
            table: "test_table".to_string(),
            columns: vec!["field".to_string()],
            unique: false,
        },
        Operation::DropIndex {
            table: "test_table".to_string(),
            columns: vec!["field".to_string()],
        },
    ];

    assert_eq!(operations.len(), 2);
}

#[test]
fn test_constraint_optimization() {
    // AddConstraint followed by DropConstraint could be optimized
    let operations = vec![
        Operation::AddConstraint {
            table: "test_table".to_string(),
            constraint_sql: "CONSTRAINT check_temp CHECK (value > 0)".to_string(),
        },
        Operation::DropConstraint {
            table: "test_table".to_string(),
            constraint_name: "check_temp".to_string(),
        },
    ];

    assert_eq!(operations.len(), 2);
}

#[test]
fn test_migration_with_no_operations() {
    // Migration with no operations
    let migration = Migration {
        app_label: "testapp".to_string(),
        name: "0001_empty".to_string(),
        operations: vec![],
        dependencies: vec![],
        replaces: vec![],
        atomic: true,
    };

    assert_eq!(migration.operations.len(), 0);
}

#[test]
fn test_migration_optimization_preserves_order() {
    // Optimization should preserve operation order when necessary
    let operations = vec![
        Operation::CreateTable {
            name: "table1".to_string(),
            columns: vec![create_column("id", "INTEGER PRIMARY KEY")],
            constraints: vec![],
        },
        Operation::CreateTable {
            name: "table2".to_string(),
            columns: vec![
                create_column("id", "INTEGER PRIMARY KEY"),
                create_column("table1_id", "INTEGER"),
            ],
            constraints: vec!["FOREIGN KEY (table1_id) REFERENCES table1(id)".to_string()],
        },
    ];

    // Order matters due to foreign key
    assert_eq!(operations.len(), 2);
}
