/// Database schema editor module
///
/// This module provides the foundation for DDL (Data Definition Language) operations
/// across different database backends, inspired by Django's schema editor architecture.
///
/// # Example
///
/// ```rust
/// use reinhardt_database::schema::{BaseDatabaseSchemaEditor, DDLStatement};
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
    /// use reinhardt_database::schema::DDLStatement;
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
/// use reinhardt_database::schema::{BaseDatabaseSchemaEditor, SchemaEditorResult};
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
///
///     fn quote_name(&self, name: &str) -> String {
///         format!("\"{}\"", name)
///     }
///
///     fn quote_value(&self, value: &str) -> String {
///         format!("'{}'", value.replace('\'', "''"))
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait BaseDatabaseSchemaEditor: Send + Sync {
    /// Execute a SQL statement
    async fn execute(&mut self, sql: &str) -> SchemaEditorResult<()>;

    /// Quote a database identifier (table name, column name, etc.)
    fn quote_name(&self, name: &str) -> String;

    /// Quote a value for SQL
    fn quote_value(&self, value: &str) -> String;

    /// Generate CREATE TABLE SQL
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_database::schema::{BaseDatabaseSchemaEditor, SchemaEditorResult};
    /// use async_trait::async_trait;
    ///
    /// struct TestEditor;
    ///
    /// #[async_trait]
    /// impl BaseDatabaseSchemaEditor for TestEditor {
    ///     async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
    ///         Ok(())
    ///     }
    ///
    ///     fn quote_name(&self, name: &str) -> String {
    ///         format!("\"{}\"", name)
    ///     }
    ///
    ///     fn quote_value(&self, value: &str) -> String {
    ///         format!("'{}'", value)
    ///     }
    /// }
    ///
    /// let editor = TestEditor;
    /// let sql = editor.create_table_sql("users", &[
    ///     ("id", "INTEGER PRIMARY KEY"),
    ///     ("name", "VARCHAR(100)"),
    /// ]);
    /// assert!(sql.contains("CREATE TABLE"));
    /// assert!(sql.contains("\"users\""));
    /// ```
    fn create_table_sql(&self, table: &str, columns: &[(&str, &str)]) -> String {
        let quoted_table = self.quote_name(table);
        let column_defs: Vec<String> = columns
            .iter()
            .map(|(name, def)| format!("{} {}", self.quote_name(name), def))
            .collect();

        format!("CREATE TABLE {} ({})", quoted_table, column_defs.join(", "))
    }

    /// Generate DROP TABLE SQL
    fn drop_table_sql(&self, table: &str, cascade: bool) -> String {
        let quoted_table = self.quote_name(table);
        if cascade {
            format!("DROP TABLE {} CASCADE", quoted_table)
        } else {
            format!("DROP TABLE {}", quoted_table)
        }
    }

    /// Generate ALTER TABLE ADD COLUMN SQL
    fn add_column_sql(&self, table: &str, column: &str, definition: &str) -> String {
        format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            self.quote_name(table),
            self.quote_name(column),
            definition
        )
    }

    /// Generate ALTER TABLE DROP COLUMN SQL
    fn drop_column_sql(&self, table: &str, column: &str) -> String {
        format!(
            "ALTER TABLE {} DROP COLUMN {}",
            self.quote_name(table),
            self.quote_name(column)
        )
    }

    /// Generate ALTER TABLE RENAME COLUMN SQL
    fn rename_column_sql(&self, table: &str, old_name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {}",
            self.quote_name(table),
            self.quote_name(old_name),
            self.quote_name(new_name)
        )
    }

    /// Generate CREATE INDEX SQL
    fn create_index_sql(
        &self,
        name: &str,
        table: &str,
        columns: &[&str],
        unique: bool,
        condition: Option<&str>,
    ) -> String {
        let unique_keyword = if unique { "UNIQUE " } else { "" };
        let quoted_columns: Vec<String> = columns.iter().map(|c| self.quote_name(c)).collect();

        let mut sql = format!(
            "CREATE {}INDEX {} ON {} ({})",
            unique_keyword,
            self.quote_name(name),
            self.quote_name(table),
            quoted_columns.join(", ")
        );

        if let Some(cond) = condition {
            sql.push_str(&format!(" WHERE {}", cond));
        }

        sql
    }

    /// Generate DROP INDEX SQL
    fn drop_index_sql(&self, name: &str) -> String {
        format!("DROP INDEX {}", self.quote_name(name))
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

        fn quote_name(&self, name: &str) -> String {
            format!("\"{}\"", name)
        }

        fn quote_value(&self, value: &str) -> String {
            format!("'{}'", value.replace('\'', "''"))
        }
    }

    #[test]
    fn test_create_table_sql() {
        let editor = TestSchemaEditor;
        let sql = editor.create_table_sql(
            "users",
            &[("id", "INTEGER PRIMARY KEY"), ("name", "VARCHAR(100)")],
        );

        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("\"users\""));
        assert!(sql.contains("\"id\""));
        assert!(sql.contains("\"name\""));
    }

    #[test]
    fn test_drop_table_sql() {
        let editor = TestSchemaEditor;

        let sql_no_cascade = editor.drop_table_sql("users", false);
        assert_eq!(sql_no_cascade, "DROP TABLE \"users\"");

        let sql_cascade = editor.drop_table_sql("users", true);
        assert_eq!(sql_cascade, "DROP TABLE \"users\" CASCADE");
    }

    #[test]
    fn test_add_column_sql() {
        let editor = TestSchemaEditor;
        let sql = editor.add_column_sql("users", "email", "VARCHAR(255)");

        assert!(sql.contains("ALTER TABLE \"users\""));
        assert!(sql.contains("ADD COLUMN \"email\" VARCHAR(255)"));
    }

    #[test]
    fn test_create_index_sql() {
        let editor = TestSchemaEditor;

        let sql = editor.create_index_sql("idx_email", "users", &["email"], false, None);
        assert!(sql.contains("CREATE INDEX \"idx_email\""));
        assert!(sql.contains("ON \"users\""));

        let unique_sql = editor.create_index_sql("idx_email_uniq", "users", &["email"], true, None);
        assert!(unique_sql.contains("CREATE UNIQUE INDEX"));

        let partial_sql = editor.create_index_sql(
            "idx_active",
            "users",
            &["email"],
            false,
            Some("active = true"),
        );
        assert!(partial_sql.contains("WHERE active = true"));
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
