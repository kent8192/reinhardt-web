//! SQLite schema editor implementation
//!
//! This module provides SQLite-specific DDL operations through the `SQLiteSchemaEditor`.
//!
//! Note: SQLite has limited ALTER TABLE support. Some operations require table recreation.

use crate::schema::{BaseDatabaseSchemaEditor, SchemaEditorError, SchemaEditorResult};

/// Quote SQLite identifier (double-quote escaping)
fn quote_sqlite_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// SQLite schema editor for DDL operations
///
/// Implements the `BaseDatabaseSchemaEditor` trait with SQLite-specific SQL syntax.
///
/// # SQLite Limitations
///
/// SQLite has limited ALTER TABLE support:
/// - Cannot ALTER COLUMN (requires table recreation)
/// - Cannot DROP CONSTRAINT (requires table recreation)
/// - Limited RENAME COLUMN support (added in SQLite 3.25.0)
///
/// # Example
///
/// ```rust
/// use reinhardt_database::backends::sqlite::schema::SQLiteSchemaEditor;
/// use reinhardt_database::schema::BaseDatabaseSchemaEditor;
///
/// let editor = SQLiteSchemaEditor::new();
///
// Create a table
/// let sql = editor.create_table_sql("users", &[
///     ("id", "INTEGER PRIMARY KEY"),
///     ("name", "TEXT"),
/// ]);
/// assert!(sql.contains("CREATE TABLE"));
/// assert!(sql.contains("\"users\""));
/// ```
#[derive(Debug, Default, Clone)]
pub struct SQLiteSchemaEditor;

impl SQLiteSchemaEditor {
    /// Create a new SQLite schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_database::backends::sqlite::schema::SQLiteSchemaEditor;
    ///
    /// let editor = SQLiteSchemaEditor::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Generate ALTER COLUMN SQL (SQLite-specific note)
    ///
    /// SQLite does not support ALTER COLUMN directly. Table recreation is required.
    pub fn alter_column_note(&self, table_name: &str) -> String {
        format!(
            "-- SQLite does not support ALTER COLUMN, table recreation required for {}",
            quote_sqlite_identifier(table_name)
        )
    }

    /// Generate RENAME TABLE SQL (SQLite-specific)
    pub fn rename_table_sql(&self, old_name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            quote_sqlite_identifier(old_name),
            quote_sqlite_identifier(new_name)
        )
    }

    /// Generate ADD CONSTRAINT note (SQLite-specific)
    ///
    /// SQLite does not support ADD CONSTRAINT. Table recreation is required.
    pub fn add_constraint_note(&self, table_name: &str) -> String {
        format!(
            "-- SQLite does not support ADD CONSTRAINT, table recreation required for {}",
            quote_sqlite_identifier(table_name)
        )
    }

    /// Generate DROP CONSTRAINT note (SQLite-specific)
    ///
    /// SQLite does not support DROP CONSTRAINT. Table recreation is required.
    pub fn drop_constraint_note(&self, table_name: &str) -> String {
        format!(
            "-- SQLite does not support DROP CONSTRAINT, table recreation required for {}",
            quote_sqlite_identifier(table_name)
        )
    }
}

#[async_trait::async_trait]
impl BaseDatabaseSchemaEditor for SQLiteSchemaEditor {
    async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
        Err(SchemaEditorError::ExecutionError(
            "Execution not supported in schema editor".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alter_column_note() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.alter_column_note("users");
        assert!(sql.contains("SQLite does not support ALTER COLUMN"));
        assert!(sql.contains("\"users\""));
    }

    #[test]
    fn test_rename_table_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.rename_table_sql("users", "people");
        assert_eq!(sql, "ALTER TABLE \"users\" RENAME TO \"people\"");
    }

    #[test]
    fn test_add_constraint_note() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.add_constraint_note("users");
        assert!(sql.contains("SQLite does not support ADD CONSTRAINT"));
        assert!(sql.contains("\"users\""));
    }

    #[test]
    fn test_drop_constraint_note() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.drop_constraint_note("users");
        assert!(sql.contains("SQLite does not support DROP CONSTRAINT"));
        assert!(sql.contains("\"users\""));
    }
}
