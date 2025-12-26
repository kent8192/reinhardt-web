//! Tests for migration operations
//! Translated and adapted from Django's test_operations.py

use reinhardt_migrations::{ColumnDefinition, Constraint, FieldType, Operation, SqlDialect};

/// Helper function to leak a string to get a 'static lifetime
fn leak_str(s: impl Into<String>) -> &'static str {
	Box::leak(s.into().into_boxed_str())
}

#[test]
fn test_create_table_basic() {
	// Test basic CreateTable operation
	let operation = Operation::CreateTable {
		name: leak_str("test_table"),
		columns: vec![
			ColumnDefinition::new("id", FieldType::Custom("INTEGER PRIMARY KEY".to_string())),
			ColumnDefinition::new("name", FieldType::Custom("TEXT NOT NULL".to_string())),
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
		name: leak_str("test_table"),
		columns: vec![
			ColumnDefinition::new("id", FieldType::Custom("INTEGER PRIMARY KEY".to_string())),
			ColumnDefinition::new("email", FieldType::Custom("TEXT NOT NULL".to_string())),
		],
		constraints: vec![Constraint::Unique {
			name: "unique_email".to_string(),
			columns: vec!["email".to_string()],
		}],
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
		name: leak_str("test_table"),
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
		table: leak_str("test_table"),
		column: ColumnDefinition::new("new_field", FieldType::Custom("TEXT".to_string())),
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
		table: leak_str("test_table"),
		column: ColumnDefinition::new(
			"status",
			FieldType::Custom("TEXT DEFAULT 'pending'".to_string()),
		),
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
		table: leak_str("test_table"),
		column: leak_str("old_field"),
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
		table: leak_str("test_table"),
		column: leak_str("field_name"),
		new_definition: ColumnDefinition::new(
			"field_name",
			FieldType::Custom("INTEGER NOT NULL".to_string()),
		),
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
		old_name: leak_str("old_table"),
		new_name: leak_str("new_table"),
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
		table: leak_str("test_table"),
		old_name: leak_str("old_col"),
		new_name: leak_str("new_col"),
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
		sql: "CREATE INDEX idx_name ON test_table(name)",
		reverse_sql: Some("DROP INDEX idx_name"),
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
		table: leak_str("test_table"),
		columns: vec!["status"],
		unique: false,
		index_type: None,
		where_clause: None,
		concurrently: false,
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
		table: leak_str("test_table"),
		columns: vec!["status"],
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
		table: leak_str("test_table"),
		constraint_sql: leak_str("CONSTRAINT check_positive CHECK (value >= 0)"),
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
		table: leak_str("test_table"),
		constraint_name: leak_str("check_positive"),
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
		name: leak_str("test_table"),
		columns: vec![
			ColumnDefinition::new("id", FieldType::Custom("SERIAL PRIMARY KEY".to_string())),
			ColumnDefinition::new("data", FieldType::Custom("JSONB".to_string())),
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
		name: leak_str("test_table"),
		columns: vec![
			ColumnDefinition::new(
				"id",
				FieldType::Custom("INT AUTO_INCREMENT PRIMARY KEY".to_string()),
			),
			ColumnDefinition::new("name", FieldType::Custom("VARCHAR(100)".to_string())),
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
		name: leak_str("test_table"),
		columns: vec![ColumnDefinition::new(
			"id",
			FieldType::Custom("INTEGER PRIMARY KEY".to_string()),
		)],
		constraints: vec![],
	};

	let reverse_op = Operation::DropTable {
		name: leak_str("test_table"),
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
		name: leak_str("users"),
		columns: vec![
			ColumnDefinition::new("id", FieldType::Custom("INTEGER PRIMARY KEY".to_string())),
			ColumnDefinition::new(
				"email",
				FieldType::Custom("TEXT NOT NULL UNIQUE".to_string()),
			),
			ColumnDefinition::new(
				"age",
				FieldType::Custom("INTEGER CHECK(age >= 0)".to_string()),
			),
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
		name: leak_str("orders"),
		columns: vec![
			ColumnDefinition::new("id", FieldType::Custom("INTEGER PRIMARY KEY".to_string())),
			ColumnDefinition::new("user_id", FieldType::Custom("INTEGER NOT NULL".to_string())),
		],
		constraints: vec![Constraint::ForeignKey {
			name: "fk_orders_user".to_string(),
			columns: vec!["user_id".to_string()],
			referenced_table: "users".to_string(),
			referenced_columns: vec!["id".to_string()],
			on_delete: reinhardt_migrations::ForeignKeyAction::NoAction,
			on_update: reinhardt_migrations::ForeignKeyAction::NoAction,
		}],
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
		table: leak_str("users"),
		columns: vec!["name", "email"],
		unique: false,
		index_type: None,
		where_clause: None,
		concurrently: false,
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
// Multi-DB SQL Generation Tests

#[test]
fn test_alter_column_multi_db() {
	// Test AlterColumn operation across all supported databases
	let operation = Operation::AlterColumn {
		table: leak_str("users"),
		column: leak_str("status"),
		new_definition: ColumnDefinition::new(
			"status",
			FieldType::Custom("VARCHAR(20) NOT NULL".to_string()),
		),
	};

	// PostgreSQL: ALTER COLUMN ... TYPE
	let sql_pg = operation.to_sql(&SqlDialect::Postgres);
	assert_eq!(
		sql_pg, "ALTER TABLE users ALTER COLUMN status TYPE VARCHAR(20) NOT NULL;",
		"PostgreSQL syntax mismatch"
	);

	// CockroachDB: Same as PostgreSQL
	let sql_crdb = operation.to_sql(&SqlDialect::Cockroachdb);
	assert_eq!(
		sql_crdb, "ALTER TABLE users ALTER COLUMN status TYPE VARCHAR(20) NOT NULL;",
		"CockroachDB syntax mismatch"
	);

	// MySQL: MODIFY COLUMN
	let sql_mysql = operation.to_sql(&SqlDialect::Mysql);
	assert_eq!(
		sql_mysql, "ALTER TABLE users MODIFY COLUMN status VARCHAR(20) NOT NULL;",
		"MySQL syntax mismatch"
	);

	// SQLite: Warning comment
	let sql_sqlite = operation.to_sql(&SqlDialect::Sqlite);
	assert!(
		sql_sqlite.contains("SQLite does not support ALTER COLUMN"),
		"SQLite should return warning comment"
	);
}

#[test]
fn test_drop_index_multi_db() {
	// Test DropIndex operation across databases
	let operation = Operation::DropIndex {
		table: leak_str("users"),
		columns: vec!["email"],
	};

	// PostgreSQL, SQLite, CockroachDB: DROP INDEX idx_name;
	let sql_pg = operation.to_sql(&SqlDialect::Postgres);
	assert_eq!(
		sql_pg, "DROP INDEX idx_users_email;",
		"PostgreSQL DROP INDEX syntax"
	);

	let sql_sqlite = operation.to_sql(&SqlDialect::Sqlite);
	assert_eq!(
		sql_sqlite, "DROP INDEX idx_users_email;",
		"SQLite DROP INDEX syntax"
	);

	let sql_crdb = operation.to_sql(&SqlDialect::Cockroachdb);
	assert_eq!(
		sql_crdb, "DROP INDEX idx_users_email;",
		"CockroachDB DROP INDEX syntax"
	);

	// MySQL: DROP INDEX idx_name ON table_name;
	let sql_mysql = operation.to_sql(&SqlDialect::Mysql);
	assert_eq!(
		sql_mysql, "DROP INDEX idx_users_email ON users;",
		"MySQL DROP INDEX syntax"
	);
}

#[test]
fn test_alter_table_comment_multi_db() {
	// Test AlterTableComment operation across databases
	let operation = Operation::AlterTableComment {
		table: leak_str("users"),
		comment: Some("User account table"),
	};

	// PostgreSQL and CockroachDB: COMMENT ON TABLE
	let sql_pg = operation.to_sql(&SqlDialect::Postgres);
	assert_eq!(
		sql_pg, "COMMENT ON TABLE users IS 'User account table';",
		"PostgreSQL COMMENT syntax"
	);

	let sql_crdb = operation.to_sql(&SqlDialect::Cockroachdb);
	assert_eq!(
		sql_crdb, "COMMENT ON TABLE users IS 'User account table';",
		"CockroachDB COMMENT syntax"
	);

	// MySQL: ALTER TABLE ... COMMENT=''
	let sql_mysql = operation.to_sql(&SqlDialect::Mysql);
	assert_eq!(
		sql_mysql, "ALTER TABLE users COMMENT='User account table';",
		"MySQL COMMENT syntax"
	);

	// SQLite: No comment support (empty string)
	let sql_sqlite = operation.to_sql(&SqlDialect::Sqlite);
	assert_eq!(sql_sqlite, "", "SQLite does not support table comments");
}

#[test]
fn test_create_table_same_across_databases() {
	// Verify CreateTable generates identical SQL across databases
	let operation = Operation::CreateTable {
		name: leak_str("products"),
		columns: vec![
			ColumnDefinition::new("id", FieldType::Custom("SERIAL PRIMARY KEY".to_string())),
			ColumnDefinition::new(
				"name",
				FieldType::Custom("VARCHAR(255) NOT NULL".to_string()),
			),
			ColumnDefinition::new("price", FieldType::Custom("DECIMAL(10,2)".to_string())),
		],
		constraints: vec![Constraint::Unique {
			name: "unique_name".to_string(),
			columns: vec!["name".to_string()],
		}],
	};

	let sql_pg = operation.to_sql(&SqlDialect::Postgres);
	let sql_mysql = operation.to_sql(&SqlDialect::Mysql);
	let sql_sqlite = operation.to_sql(&SqlDialect::Sqlite);
	let sql_crdb = operation.to_sql(&SqlDialect::Cockroachdb);

	// All databases should generate identical CREATE TABLE syntax
	assert_eq!(
		sql_pg, sql_mysql,
		"PostgreSQL and MySQL CREATE TABLE should match"
	);
	assert_eq!(
		sql_pg, sql_sqlite,
		"PostgreSQL and SQLite CREATE TABLE should match"
	);
	assert_eq!(
		sql_pg, sql_crdb,
		"PostgreSQL and CockroachDB CREATE TABLE should match"
	);

	// Verify common structure
	assert!(
		sql_pg.contains("CREATE TABLE products"),
		"Should contain table name"
	);
	assert!(
		sql_pg.contains("id SERIAL PRIMARY KEY"),
		"Should contain id column"
	);
	assert!(
		sql_pg.contains("UNIQUE (name)"),
		"Should contain constraint"
	);
}
