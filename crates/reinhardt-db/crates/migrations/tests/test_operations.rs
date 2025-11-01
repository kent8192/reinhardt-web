//! Tests for migration operations
//! Translated and adapted from Django's test_operations.py

use reinhardt_migrations::{ColumnDefinition, Operation, SqlDialect};

#[test]
fn test_create_table_basic() {
	// Test basic CreateTable operation
	let operation = Operation::CreateTable {
		name: "test_table".to_string(),
		columns: vec![
			ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
			ColumnDefinition::new("name", "TEXT NOT NULL"),
		],
		constraints: vec![],
	};

	// Test SQL generation for SQLite
	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("CREATE TABLE"),
		"Expected SQL to contain 'CREATE TABLE', got: {}",
		sql
	);
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);
	assert!(
		sql.contains("id"),
		"Expected SQL to contain 'id', got: {}",
		sql
	);
	assert!(
		sql.contains("name"),
		"Expected SQL to contain 'name', got: {}",
		sql
	);
}

#[test]
fn test_create_table_with_constraints() {
	// Test CreateTable with constraints
	let operation = Operation::CreateTable {
		name: "test_table".to_string(),
		columns: vec![
			ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
			ColumnDefinition::new("email", "TEXT NOT NULL"),
		],
		constraints: vec!["UNIQUE(email)".to_string()],
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("UNIQUE"),
		"Expected SQL to contain 'UNIQUE', got: {}",
		sql
	);
	assert!(
		sql.contains("email"),
		"Expected SQL to contain 'email', got: {}",
		sql
	);
}

#[test]
fn test_drop_table() {
	// Test DropTable operation
	let operation = Operation::DropTable {
		name: "test_table".to_string(),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("DROP TABLE"),
		"Expected SQL to contain 'DROP TABLE', got: {}",
		sql
	);
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);
}

#[test]
fn test_add_column() {
	// Test AddColumn operation
	let operation = Operation::AddColumn {
		table: "test_table".to_string(),
		column: ColumnDefinition::new("new_field", "TEXT"),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("ALTER TABLE"),
		"Expected SQL to contain 'ALTER TABLE', got: {}",
		sql
	);
	assert!(
		sql.contains("ADD COLUMN"),
		"Expected SQL to contain 'ADD COLUMN', got: {}",
		sql
	);
	assert!(
		sql.contains("new_field"),
		"Expected SQL to contain 'new_field', got: {}",
		sql
	);
}

#[test]
fn test_add_column_with_default() {
	// Test AddColumn with default value
	let operation = Operation::AddColumn {
		table: "test_table".to_string(),
		column: ColumnDefinition::new("status", "TEXT DEFAULT 'pending'"),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("DEFAULT"),
		"Expected SQL to contain 'DEFAULT', got: {}",
		sql
	);
	assert!(
		sql.contains("pending"),
		"Expected SQL to contain 'pending', got: {}",
		sql
	);
}

#[test]
fn test_drop_column() {
	// Test DropColumn operation
	let operation = Operation::DropColumn {
		table: "test_table".to_string(),
		column: "old_field".to_string(),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("ALTER TABLE"),
		"Expected SQL to contain 'ALTER TABLE', got: {}",
		sql
	);
	assert!(
		sql.contains("DROP COLUMN"),
		"Expected SQL to contain 'DROP COLUMN', got: {}",
		sql
	);
	assert!(
		sql.contains("old_field"),
		"Expected SQL to contain 'old_field', got: {}",
		sql
	);
}

#[test]
fn test_alter_column() {
	// Test AlterColumn operation
	let operation = Operation::AlterColumn {
		table: "test_table".to_string(),
		column: "field_name".to_string(),
		new_definition: ColumnDefinition::new("field_name", "INTEGER NOT NULL"),
	};

	// SQLite doesn't support ALTER COLUMN natively
	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);

	// Test with PostgreSQL
	let sql_pg = operation.to_sql(&SqlDialect::Postgres);
	assert!(
		sql_pg.contains("ALTER TABLE"),
		"Expected PostgreSQL SQL to contain 'ALTER TABLE', got: {}",
		sql_pg
	);
	assert!(
		sql_pg.contains("ALTER COLUMN"),
		"Expected PostgreSQL SQL to contain 'ALTER COLUMN', got: {}",
		sql_pg
	);
	assert!(
		sql_pg.contains("field_name"),
		"Expected PostgreSQL SQL to contain 'field_name', got: {}",
		sql_pg
	);

	// Test with MySQL
	let sql_mysql = operation.to_sql(&SqlDialect::Mysql);
	assert!(
		sql_mysql.contains("ALTER TABLE"),
		"Expected MySQL SQL to contain 'ALTER TABLE', got: {}",
		sql_mysql
	);
	assert!(
		sql_mysql.contains("MODIFY COLUMN"),
		"Expected MySQL SQL to contain 'MODIFY COLUMN', got: {}",
		sql_mysql
	);
	assert!(
		sql_mysql.contains("field_name"),
		"Expected MySQL SQL to contain 'field_name', got: {}",
		sql_mysql
	);
}

#[test]
fn test_rename_table() {
	// Test RenameTable operation
	let operation = Operation::RenameTable {
		old_name: "old_table".to_string(),
		new_name: "new_table".to_string(),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("old_table"),
		"Expected SQL to contain 'old_table', got: {}",
		sql
	);
	assert!(
		sql.contains("new_table"),
		"Expected SQL to contain 'new_table', got: {}",
		sql
	);
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
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);
}

#[test]
fn test_run_sql() {
	// Test RunSQL operation
	let operation = Operation::RunSQL {
		sql: "CREATE INDEX idx_name ON test_table(name)".to_string(),
		reverse_sql: Some("DROP INDEX idx_name".to_string()),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("CREATE INDEX"),
		"Expected SQL to contain 'CREATE INDEX', got: {}",
		sql
	);
	assert!(
		sql.contains("idx_name"),
		"Expected SQL to contain 'idx_name', got: {}",
		sql
	);
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
	assert!(
		sql.contains("CREATE INDEX"),
		"Expected SQL to contain 'CREATE INDEX', got: {}",
		sql
	);
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);
}

#[test]
fn test_drop_index() {
	// Test DropIndex operation
	let operation = Operation::DropIndex {
		table: "test_table".to_string(),
		columns: vec!["status".to_string()],
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("DROP INDEX"),
		"Expected SQL to contain 'DROP INDEX', got: {}",
		sql
	);
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);
}

#[test]
fn test_add_constraint() {
	// Test AddConstraint operation
	let operation = Operation::AddConstraint {
		table: "test_table".to_string(),
		constraint_sql: "CONSTRAINT check_positive CHECK (value >= 0)".to_string(),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);
	assert!(
		sql.contains("check_positive"),
		"Expected SQL to contain 'check_positive', got: {}",
		sql
	);
}

#[test]
fn test_drop_constraint() {
	// Test DropConstraint operation
	let operation = Operation::DropConstraint {
		table: "test_table".to_string(),
		constraint_name: "check_positive".to_string(),
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("test_table"),
		"Expected SQL to contain 'test_table', got: {}",
		sql
	);
}

#[test]
fn test_postgres_sql_generation() {
	// Test SQL generation for PostgreSQL
	let operation = Operation::CreateTable {
		name: "test_table".to_string(),
		columns: vec![
			ColumnDefinition::new("id", "SERIAL PRIMARY KEY"),
			ColumnDefinition::new("data", "JSONB"),
		],
		constraints: vec![],
	};

	let sql = operation.to_sql(&SqlDialect::Postgres);
	assert!(
		sql.contains("CREATE TABLE"),
		"Expected PostgreSQL SQL to contain 'CREATE TABLE', got: {}",
		sql
	);
	assert!(
		sql.contains("SERIAL"),
		"Expected PostgreSQL SQL to contain 'SERIAL', got: {}",
		sql
	);
	assert!(
		sql.contains("JSONB"),
		"Expected PostgreSQL SQL to contain 'JSONB', got: {}",
		sql
	);
}

#[test]
fn test_mysql_sql_generation() {
	// Test SQL generation for MySQL
	let operation = Operation::CreateTable {
		name: "test_table".to_string(),
		columns: vec![
			ColumnDefinition::new("id", "INT AUTO_INCREMENT PRIMARY KEY"),
			ColumnDefinition::new("name", "VARCHAR(100)"),
		],
		constraints: vec![],
	};

	let sql = operation.to_sql(&SqlDialect::Mysql);
	assert!(
		sql.contains("CREATE TABLE"),
		"Expected MySQL SQL to contain 'CREATE TABLE', got: {}",
		sql
	);
	assert!(
		sql.contains("AUTO_INCREMENT"),
		"Expected MySQL SQL to contain 'AUTO_INCREMENT', got: {}",
		sql
	);
}

#[test]
fn test_operation_reversibility() {
	// Test that operations can be reversed
	let forward_op = Operation::CreateTable {
		name: "test_table".to_string(),
		columns: vec![ColumnDefinition::new("id", "INTEGER PRIMARY KEY")],
		constraints: vec![],
	};

	let reverse_op = Operation::DropTable {
		name: "test_table".to_string(),
	};

	let forward_sql = forward_op.to_sql(&SqlDialect::Sqlite);
	let reverse_sql = reverse_op.to_sql(&SqlDialect::Sqlite);

	assert!(
		forward_sql.contains("CREATE"),
		"Expected forward SQL to contain 'CREATE', got: {}",
		forward_sql
	);
	assert!(
		reverse_sql.contains("DROP"),
		"Expected reverse SQL to contain 'DROP', got: {}",
		reverse_sql
	);
}

#[test]
fn test_column_definition_with_multiple_constraints() {
	// Test column with multiple constraints
	let operation = Operation::CreateTable {
		name: "users".to_string(),
		columns: vec![
			ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
			ColumnDefinition::new("email", "TEXT NOT NULL UNIQUE"),
			ColumnDefinition::new("age", "INTEGER CHECK(age >= 0)"),
		],
		constraints: vec![],
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("NOT NULL"),
		"Expected SQL to contain 'NOT NULL', got: {}",
		sql
	);
	assert!(
		sql.contains("UNIQUE"),
		"Expected SQL to contain 'UNIQUE', got: {}",
		sql
	);
	assert!(
		sql.contains("CHECK"),
		"Expected SQL to contain 'CHECK', got: {}",
		sql
	);
}

#[test]
fn test_migrations_foreign_key_constraint() {
	// Test table creation with foreign key
	let operation = Operation::CreateTable {
		name: "orders".to_string(),
		columns: vec![
			ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
			ColumnDefinition::new("user_id", "INTEGER NOT NULL"),
		],
		constraints: vec!["FOREIGN KEY (user_id) REFERENCES users(id)".to_string()],
	};

	let sql = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql.contains("FOREIGN KEY"),
		"Expected SQL to contain 'FOREIGN KEY', got: {}",
		sql
	);
	assert!(
		sql.contains("REFERENCES"),
		"Expected SQL to contain 'REFERENCES', got: {}",
		sql
	);
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
	assert!(
		sql.contains("name"),
		"Expected SQL to contain 'name', got: {}",
		sql
	);
	assert!(
		sql.contains("email"),
		"Expected SQL to contain 'email', got: {}",
		sql
	);
}
