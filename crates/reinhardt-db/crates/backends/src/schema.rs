/// Database schema editor module
///
/// This module provides the foundation for DDL (Data Definition Language) operations
/// across different database backends, inspired by Django's schema editor architecture.
///
/// # Example
///
/// ```rust
/// use reinhardt_db::reinhardt_backends::schema::{BaseDatabaseSchemaEditor, DDLStatement};
///
// Example of using schema editor to generate DDL
/// let create_table = DDLStatement::CreateTable {
///     table: "users".to_string(),
///     columns: vec![
///         ("id".to_string(), "INTEGER PRIMARY KEY".to_string()),
///         ("name".to_string(), "VARCHAR(100)".to_string()),
///     ],
/// };
/// assert_eq!(create_table.table_name(), "users");
/// ```
use std::fmt;

use sea_query::{
	Alias, ColumnDef, Index, IndexCreateStatement, IndexDropStatement, Table, TableAlterStatement,
	TableCreateStatement, TableDropStatement,
};

/// DDL reference objects for schema operations
pub mod ddl_references;

/// Schema editor factory for creating database-specific editors
pub mod factory;

/// Represents a DDL statement type
#[derive(Debug, Clone, PartialEq)]
pub enum DDLStatement {
	/// CREATE TABLE statement
	CreateTable {
		table: String,
		columns: Vec<(String, String)>,
	},
	/// ALTER TABLE statement
	AlterTable {
		table: String,
		changes: Vec<AlterTableChange>,
	},
	/// DROP TABLE statement
	DropTable { table: String, cascade: bool },
	/// CREATE INDEX statement
	CreateIndex {
		name: String,
		table: String,
		columns: Vec<String>,
		unique: bool,
		condition: Option<String>,
	},
	/// DROP INDEX statement
	DropIndex { name: String },
	/// Raw SQL statement
	RawSQL(String),
}

impl DDLStatement {
	/// Get the table name associated with this DDL statement
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::reinhardt_backends::schema::DDLStatement;
	///
	/// let stmt = DDLStatement::CreateTable {
	///     table: "users".to_string(),
	///     columns: vec![],
	/// };
	/// assert_eq!(stmt.table_name(), "users");
	/// ```
	pub fn table_name(&self) -> &str {
		match self {
			DDLStatement::CreateTable { table, .. } => table,
			DDLStatement::AlterTable { table, .. } => table,
			DDLStatement::DropTable { table, .. } => table,
			DDLStatement::CreateIndex { table, .. } => table,
			DDLStatement::DropIndex { .. } => "",
			DDLStatement::RawSQL(_) => "",
		}
	}
}

/// ALTER TABLE change operations
#[derive(Debug, Clone, PartialEq)]
pub enum AlterTableChange {
	/// Add a column
	AddColumn { name: String, definition: String },
	/// Drop a column
	DropColumn { name: String },
	/// Rename a column
	RenameColumn { old_name: String, new_name: String },
	/// Alter column type
	AlterColumnType {
		name: String,
		new_type: String,
		collation: Option<String>,
	},
	/// Set/drop column default
	AlterColumnDefault {
		name: String,
		default: Option<String>,
	},
	/// Set/drop NOT NULL constraint
	AlterColumnNullability { name: String, nullable: bool },
	/// Add constraint
	AddConstraint { name: String, definition: String },
	/// Drop constraint
	DropConstraint { name: String },
}

/// Base trait for database schema editors
///
/// This trait defines the interface that all database-specific schema editors must implement.
/// It provides methods for creating, altering, and dropping database schema objects.
///
/// # Example
///
/// ```rust
/// use reinhardt_db::reinhardt_backends::schema::{BaseDatabaseSchemaEditor, SchemaEditorResult};
/// use async_trait::async_trait;
///
/// struct MySchemaEditor;
///
/// #[async_trait]
/// impl BaseDatabaseSchemaEditor for MySchemaEditor {
///     async fn execute(&mut self, sql: &str) -> SchemaEditorResult<()> {
///         // Execute SQL
///         println!("Executing: {}", sql);
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait BaseDatabaseSchemaEditor: Send + Sync {
	/// Execute a SQL statement
	async fn execute(&mut self, sql: &str) -> SchemaEditorResult<()>;

	/// Generate CREATE TABLE statement using SeaQuery
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::reinhardt_backends::schema::{BaseDatabaseSchemaEditor, SchemaEditorResult};
	/// use async_trait::async_trait;
	/// use sea_query::PostgresQueryBuilder;
	///
	/// struct TestEditor;
	///
	/// #[async_trait]
	/// impl BaseDatabaseSchemaEditor for TestEditor {
	///     async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
	///         Ok(())
	///     }
	/// }
	///
	/// let editor = TestEditor;
	/// let stmt = editor.create_table_statement("users", &[
	///     ("id", "INTEGER PRIMARY KEY"),
	///     ("name", "VARCHAR(100)"),
	/// ]);
	/// let sql = stmt.to_string(PostgresQueryBuilder);
	/// assert!(sql.contains("CREATE TABLE"));
	/// ```
	fn create_table_statement(
		&self,
		table: &str,
		columns: &[(&str, &str)],
	) -> TableCreateStatement {
		let mut stmt = Table::create();
		stmt.table(Alias::new(table)).if_not_exists();

		for (name, definition) in columns {
			let mut col = ColumnDef::new(Alias::new(*name));
			// Use custom() for raw type definitions since we receive them as strings
			col.custom(Alias::new(*definition));
			stmt.col(&mut col);
		}

		stmt.to_owned()
	}

	/// Generate DROP TABLE statement using SeaQuery
	fn drop_table_statement(&self, table: &str, cascade: bool) -> TableDropStatement {
		let mut stmt = Table::drop();
		stmt.table(Alias::new(table)).if_exists();

		if cascade {
			stmt.cascade();
		}

		stmt.to_owned()
	}

	/// Generate ALTER TABLE ADD COLUMN statement using SeaQuery
	fn add_column_statement(
		&self,
		table: &str,
		column: &str,
		definition: &str,
	) -> TableAlterStatement {
		let mut stmt = Table::alter();
		stmt.table(Alias::new(table));

		let mut col = ColumnDef::new(Alias::new(column));
		// Use custom() for raw type definitions
		col.custom(Alias::new(definition));
		stmt.add_column(&mut col);

		stmt.to_owned()
	}

	/// Generate ALTER TABLE DROP COLUMN statement using SeaQuery
	fn drop_column_statement(&self, table: &str, column: &str) -> TableAlterStatement {
		let mut stmt = Table::alter();
		stmt.table(Alias::new(table));
		stmt.drop_column(Alias::new(column));

		stmt.to_owned()
	}

	/// Generate ALTER TABLE RENAME COLUMN SQL
	///
	/// Always uses double quotes for PostgreSQL identifier safety.
	/// Note: SeaQuery doesn't support RENAME COLUMN, so we use raw SQL.
	fn rename_column_statement(&self, table: &str, old_name: &str, new_name: &str) -> String {
		format!(
			"ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
			table, old_name, new_name
		)
	}

	/// Generate ALTER TABLE ALTER COLUMN TYPE SQL
	///
	/// Returns database-specific SQL for changing a column's type.
	/// Note: SeaQuery doesn't support ALTER COLUMN TYPE, so we use raw SQL.
	///
	/// Default implementation uses PostgreSQL syntax:
	/// `ALTER TABLE table ALTER COLUMN column TYPE new_type`
	///
	/// Override this method in database-specific schema editors for:
	/// - MySQL: `ALTER TABLE table MODIFY COLUMN column new_type`
	/// - SQLite: Requires table recreation (complex multi-step process)
	/// - CockroachDB: Same as PostgreSQL
	fn alter_column_statement(&self, table: &str, column: &str, new_type: &str) -> String {
		format!(
			"ALTER TABLE \"{}\" ALTER COLUMN \"{}\" TYPE {}",
			table, column, new_type
		)
	}

	/// Generate CREATE INDEX statement using SeaQuery (or pg_escape for partial indexes)
	///
	/// Note: SeaQuery doesn't support partial indexes (WHERE clause), so we use raw SQL with pg_escape for those cases
	fn create_index_statement(
		&self,
		name: &str,
		table: &str,
		columns: &[&str],
		unique: bool,
		condition: Option<&str>,
	) -> Result<IndexCreateStatement, String> {
		if condition.is_some() {
			// SeaQuery doesn't support partial indexes, return error to indicate fallback needed
			// Always use double quotes for PostgreSQL identifier safety
			return Err(format!(
				"Partial indexes not supported by SeaQuery. Use raw SQL: CREATE {}INDEX \"{}\" ON \"{}\" ({}) WHERE {}",
				if unique { "UNIQUE " } else { "" },
				name,
				table,
				columns
					.iter()
					.map(|c| format!("\"{}\"", c))
					.collect::<Vec<_>>()
					.join(", "),
				condition.unwrap()
			));
		}

		let mut stmt = Index::create();
		stmt.name(name).table(Alias::new(table));

		if unique {
			stmt.unique();
		}

		for col in columns {
			stmt.col(Alias::new(*col));
		}

		Ok(stmt.to_owned())
	}

	/// Generate DROP INDEX statement using SeaQuery
	fn drop_index_statement(&self, name: &str) -> IndexDropStatement {
		let mut stmt = Index::drop();
		stmt.name(name).if_exists();

		stmt.to_owned()
	}
}

/// Result type for schema editor operations
pub type SchemaEditorResult<T> = Result<T, SchemaEditorError>;

/// Errors that can occur during schema editing operations
#[derive(Debug, Clone)]
pub enum SchemaEditorError {
	/// SQL execution error
	ExecutionError(String),
	/// Invalid operation
	InvalidOperation(String),
	/// Database error
	DatabaseError(String),
}

impl fmt::Display for SchemaEditorError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			SchemaEditorError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
			SchemaEditorError::InvalidOperation(msg) => {
				write!(f, "Invalid operation: {}", msg)
			}
			SchemaEditorError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
		}
	}
}

impl std::error::Error for SchemaEditorError {}

#[cfg(test)]
mod tests {
	use super::*;

	struct TestSchemaEditor;

	#[async_trait::async_trait]
	impl BaseDatabaseSchemaEditor for TestSchemaEditor {
		async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
			Ok(())
		}
	}

	#[test]
	fn test_create_table_statement() {
		use sea_query::PostgresQueryBuilder;

		let editor = TestSchemaEditor;
		let stmt = editor.create_table_statement(
			"users",
			&[("id", "INTEGER PRIMARY KEY"), ("name", "VARCHAR(100)")],
		);
		let sql = stmt.to_string(PostgresQueryBuilder);

		assert!(sql.contains("CREATE TABLE"));
		assert!(sql.contains("\"users\""));
		assert!(sql.contains("\"id\""));
		assert!(sql.contains("\"name\""));
	}

	#[test]
	fn test_drop_table_statement() {
		use sea_query::PostgresQueryBuilder;

		let editor = TestSchemaEditor;

		let stmt_no_cascade = editor.drop_table_statement("users", false);
		let sql_no_cascade = stmt_no_cascade.to_string(PostgresQueryBuilder);
		assert!(sql_no_cascade.contains("DROP TABLE"));
		assert!(sql_no_cascade.contains("\"users\""));

		let stmt_cascade = editor.drop_table_statement("users", true);
		let sql_cascade = stmt_cascade.to_string(PostgresQueryBuilder);
		assert!(sql_cascade.contains("DROP TABLE"));
		assert!(sql_cascade.contains("\"users\""));
		assert!(sql_cascade.contains("CASCADE"));
	}

	#[test]
	fn test_add_column_statement() {
		use sea_query::PostgresQueryBuilder;

		let editor = TestSchemaEditor;
		let stmt = editor.add_column_statement("users", "email", "VARCHAR(255)");
		let sql = stmt.to_string(PostgresQueryBuilder);

		assert!(sql.contains("ALTER TABLE"));
		assert!(sql.contains("\"users\""));
		assert!(sql.contains("ADD COLUMN"));
		assert!(sql.contains("\"email\""));
		assert!(sql.contains("VARCHAR(255)"));
	}

	#[test]
	fn test_create_index_statement() {
		use sea_query::PostgresQueryBuilder;

		let editor = TestSchemaEditor;

		// Simple index
		let stmt = editor.create_index_statement("idx_email", "users", &["email"], false, None);
		assert!(stmt.is_ok());
		let sql = stmt.unwrap().to_string(PostgresQueryBuilder);
		assert!(sql.contains("CREATE INDEX"));
		assert!(sql.contains("idx_email"));
		assert!(sql.contains("\"users\""));

		// Unique index
		let unique_stmt =
			editor.create_index_statement("idx_email_uniq", "users", &["email"], true, None);
		assert!(unique_stmt.is_ok());
		let unique_sql = unique_stmt.unwrap().to_string(PostgresQueryBuilder);
		assert!(unique_sql.contains("CREATE UNIQUE INDEX"));

		// Partial index (not supported by SeaQuery, returns error with fallback SQL)
		let partial_result = editor.create_index_statement(
			"idx_active",
			"users",
			&["email"],
			false,
			Some("active = true"),
		);
		assert!(partial_result.is_err());
		let error_msg = partial_result.unwrap_err();
		assert!(error_msg.contains("Partial indexes not supported"));
		assert!(error_msg.contains("WHERE active = true"));
	}

	#[test]
	fn test_alter_column_statement() {
		let editor = TestSchemaEditor;

		// Test default PostgreSQL syntax
		let sql = editor.alter_column_statement("users", "email", "TEXT");
		assert_eq!(
			sql,
			"ALTER TABLE \"users\" ALTER COLUMN \"email\" TYPE TEXT"
		);

		// Verify identifier quoting
		assert!(sql.contains("\"users\""));
		assert!(sql.contains("\"email\""));
		assert!(sql.contains("TYPE TEXT"));
	}

	#[test]
	fn test_ddl_statement_table_name() {
		let stmt = DDLStatement::CreateTable {
			table: "users".to_string(),
			columns: vec![],
		};
		assert_eq!(stmt.table_name(), "users");

		let alter_stmt = DDLStatement::AlterTable {
			table: "posts".to_string(),
			changes: vec![],
		};
		assert_eq!(alter_stmt.table_name(), "posts");
	}
}
