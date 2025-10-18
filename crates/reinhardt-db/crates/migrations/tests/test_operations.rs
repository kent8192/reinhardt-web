//! Tests for migration operations
//! Translated and adapted from Django's test_operations.py

use reinhardt_migrations::{ColumnDefinition, Operation, SqlDialect};

#[test]
fn test_create_table_basic() {
    // Test basic CreateTable operation
    let operation = Operation::CreateTable {
        name: "test_table".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                type_definition: "INTEGER PRIMARY KEY".to_string(),
            },
            ColumnDefinition {
                name: "name".to_string(),
                type_definition: "TEXT NOT NULL".to_string(),
            },
        ],
        constraints: vec![],
    };

    // Test SQL generation for SQLite
    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("test_table"));
    assert!(sql.contains("id"));
    assert!(sql.contains("name"));
}

#[test]
fn test_create_table_with_constraints() {
    // Test CreateTable with constraints
    let operation = Operation::CreateTable {
        name: "test_table".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                type_definition: "INTEGER PRIMARY KEY".to_string(),
            },
            ColumnDefinition {
                name: "email".to_string(),
                type_definition: "TEXT NOT NULL".to_string(),
            },
        ],
        constraints: vec!["UNIQUE(email)".to_string()],
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("UNIQUE"));
    assert!(sql.contains("email"));
}

#[test]
fn test_drop_table() {
    // Test DropTable operation
    let operation = Operation::DropTable {
        name: "test_table".to_string(),
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("DROP TABLE"));
    assert!(sql.contains("test_table"));
}

#[test]
fn test_add_column() {
    // Test AddColumn operation
    let operation = Operation::AddColumn {
        table: "test_table".to_string(),
        column: ColumnDefinition {
            name: "new_field".to_string(),
            type_definition: "TEXT".to_string(),
        },
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("ALTER TABLE"));
    assert!(sql.contains("ADD COLUMN"));
    assert!(sql.contains("new_field"));
}

#[test]
fn test_add_column_with_default() {
    // Test AddColumn with default value
    let operation = Operation::AddColumn {
        table: "test_table".to_string(),
        column: ColumnDefinition {
            name: "status".to_string(),
            type_definition: "TEXT DEFAULT 'pending'".to_string(),
        },
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("DEFAULT"));
    assert!(sql.contains("pending"));
}

#[test]
fn test_drop_column() {
    // Test DropColumn operation
    let operation = Operation::DropColumn {
        table: "test_table".to_string(),
        column: "old_field".to_string(),
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("ALTER TABLE"));
    assert!(sql.contains("DROP COLUMN"));
    assert!(sql.contains("old_field"));
}

#[test]
fn test_alter_column() {
    // Test AlterColumn operation
    let operation = Operation::AlterColumn {
        table: "test_table".to_string(),
        column: "field_name".to_string(),
        new_definition: ColumnDefinition {
            name: "field_name".to_string(),
            type_definition: "INTEGER NOT NULL".to_string(),
        },
    };

    // SQLite doesn't support ALTER COLUMN natively
    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("test_table"));

    // Test with PostgreSQL
    let sql_pg = operation.to_sql(&SqlDialect::Postgres);
    assert!(sql_pg.contains("ALTER TABLE"));
    assert!(sql_pg.contains("ALTER COLUMN"));
    assert!(sql_pg.contains("field_name"));

    // Test with MySQL
    let sql_mysql = operation.to_sql(&SqlDialect::Mysql);
    assert!(sql_mysql.contains("ALTER TABLE"));
    assert!(sql_mysql.contains("MODIFY COLUMN"));
    assert!(sql_mysql.contains("field_name"));
}

#[test]
fn test_rename_table() {
    // Test RenameTable operation
    let operation = Operation::RenameTable {
        old_name: "old_table".to_string(),
        new_name: "new_table".to_string(),
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("old_table"));
    assert!(sql.contains("new_table"));
}

#[test]
fn test_rename_column() {
    // Test RenameColumn operation
    let operation = Operation::RenameColumn {
        table: "test_table".to_string(),
        old_name: "old_col".to_string(),
        new_name: "new_col".to_string(),
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("test_table"));
}

#[test]
fn test_run_sql() {
    // Test RunSQL operation
    let operation = Operation::RunSQL {
        sql: "CREATE INDEX idx_name ON test_table(name)".to_string(),
        reverse_sql: Some("DROP INDEX idx_name".to_string()),
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("CREATE INDEX"));
    assert!(sql.contains("idx_name"));
}

#[test]
fn test_create_index() {
    // Test CreateIndex operation
    let operation = Operation::CreateIndex {
        table: "test_table".to_string(),
        columns: vec!["status".to_string()],
        unique: false,
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("CREATE INDEX"));
    assert!(sql.contains("test_table"));
}

#[test]
fn test_drop_index() {
    // Test DropIndex operation
    let operation = Operation::DropIndex {
        table: "test_table".to_string(),
        columns: vec!["status".to_string()],
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("DROP INDEX"));
    assert!(sql.contains("test_table"));
}

#[test]
fn test_add_constraint() {
    // Test AddConstraint operation
    let operation = Operation::AddConstraint {
        table: "test_table".to_string(),
        constraint_sql: "CONSTRAINT check_positive CHECK (value >= 0)".to_string(),
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("test_table"));
    assert!(sql.contains("check_positive"));
}

#[test]
fn test_drop_constraint() {
    // Test DropConstraint operation
    let operation = Operation::DropConstraint {
        table: "test_table".to_string(),
        constraint_name: "check_positive".to_string(),
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("test_table"));
}

#[test]
fn test_postgres_sql_generation() {
    // Test SQL generation for PostgreSQL
    let operation = Operation::CreateTable {
        name: "test_table".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                type_definition: "SERIAL PRIMARY KEY".to_string(),
            },
            ColumnDefinition {
                name: "data".to_string(),
                type_definition: "JSONB".to_string(),
            },
        ],
        constraints: vec![],
    };

    let sql = operation.to_sql(&SqlDialect::Postgres);
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("SERIAL"));
    assert!(sql.contains("JSONB"));
}

#[test]
fn test_mysql_sql_generation() {
    // Test SQL generation for MySQL
    let operation = Operation::CreateTable {
        name: "test_table".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                type_definition: "INT AUTO_INCREMENT PRIMARY KEY".to_string(),
            },
            ColumnDefinition {
                name: "name".to_string(),
                type_definition: "VARCHAR(100)".to_string(),
            },
        ],
        constraints: vec![],
    };

    let sql = operation.to_sql(&SqlDialect::Mysql);
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("AUTO_INCREMENT"));
}

#[test]
fn test_operation_reversibility() {
    // Test that operations can be reversed
    let forward_op = Operation::CreateTable {
        name: "test_table".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            type_definition: "INTEGER PRIMARY KEY".to_string(),
        }],
        constraints: vec![],
    };

    let reverse_op = Operation::DropTable {
        name: "test_table".to_string(),
    };

    let forward_sql = forward_op.to_sql(&SqlDialect::Sqlite);
    let reverse_sql = reverse_op.to_sql(&SqlDialect::Sqlite);

    assert!(forward_sql.contains("CREATE"));
    assert!(reverse_sql.contains("DROP"));
}

#[test]
fn test_column_definition_with_multiple_constraints() {
    // Test column with multiple constraints
    let operation = Operation::CreateTable {
        name: "users".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                type_definition: "INTEGER PRIMARY KEY".to_string(),
            },
            ColumnDefinition {
                name: "email".to_string(),
                type_definition: "TEXT NOT NULL UNIQUE".to_string(),
            },
            ColumnDefinition {
                name: "age".to_string(),
                type_definition: "INTEGER CHECK(age >= 0)".to_string(),
            },
        ],
        constraints: vec![],
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("NOT NULL"));
    assert!(sql.contains("UNIQUE"));
    assert!(sql.contains("CHECK"));
}

#[test]
fn test_migrations_foreign_key_constraint() {
    // Test table creation with foreign key
    let operation = Operation::CreateTable {
        name: "orders".to_string(),
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
        constraints: vec!["FOREIGN KEY (user_id) REFERENCES users(id)".to_string()],
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("FOREIGN KEY"));
    assert!(sql.contains("REFERENCES"));
}

#[test]
fn test_migrations_operations_composite_index() {
    // Test creating a composite index
    let operation = Operation::CreateIndex {
        table: "users".to_string(),
        columns: vec!["name".to_string(), "email".to_string()],
        unique: false,
    };

    let sql = operation.to_sql(&SqlDialect::Sqlite);
    assert!(sql.contains("name"));
    assert!(sql.contains("email"));
}
