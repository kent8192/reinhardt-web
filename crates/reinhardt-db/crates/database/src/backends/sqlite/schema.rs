//! SQLite schema editor implementation
//!
//! This module provides SQLite-specific DDL operations through the `SQLiteSchemaEditor`.
//!
//! Note: SQLite has limited ALTER TABLE support. Some operations require table recreation.

use crate::schema::{BaseDatabaseSchemaEditor, SchemaEditorError, SchemaEditorResult};

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
            self.quote_name(table_name)
        )
    }

    /// Generate RENAME TABLE SQL (SQLite-specific)
    pub fn rename_table_sql(&self, old_name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            self.quote_name(old_name),
            self.quote_name(new_name)
        )
    }

    /// Generate ADD CONSTRAINT note (SQLite-specific)
    ///
    /// SQLite does not support ADD CONSTRAINT. Table recreation is required.
    pub fn add_constraint_note(&self, table_name: &str) -> String {
        format!(
            "-- SQLite does not support ADD CONSTRAINT, table recreation required for {}",
            self.quote_name(table_name)
        )
    }

    /// Generate DROP CONSTRAINT note (SQLite-specific)
    ///
    /// SQLite does not support DROP CONSTRAINT. Table recreation is required.
    pub fn drop_constraint_note(&self, table_name: &str) -> String {
        format!(
            "-- SQLite does not support DROP CONSTRAINT, table recreation required for {}",
            self.quote_name(table_name)
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

    fn quote_name(&self, name: &str) -> String {
        format!("\"{}\"", name)
    }

    fn quote_value(&self, value: &str) -> String {
        format!("'{}'", value.replace('\'', "''"))
    }

    fn create_table_sql(&self, table: &str, columns: &[(&str, &str)]) -> String {
        let mut parts = Vec::new();
        for (name, type_def) in columns {
            parts.push(format!("{} {}", self.quote_name(name), type_def));
        }
        format!(
            "CREATE TABLE {} ({})",
            self.quote_name(table),
            parts.join(", ")
        )
    }

    fn drop_table_sql(&self, table: &str, cascade: bool) -> String {
        let quoted_table = self.quote_name(table);
        if cascade {
            // SQLite doesn't have CASCADE, but we note it
            format!(
                "{} /* CASCADE not supported in SQLite */",
                format!("DROP TABLE {}", quoted_table)
            )
        } else {
            format!("DROP TABLE {}", quoted_table)
        }
    }

    fn add_column_sql(&self, table: &str, column: &str, definition: &str) -> String {
        format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            self.quote_name(table),
            self.quote_name(column),
            definition
        )
    }

    fn drop_column_sql(&self, table: &str, column: &str) -> String {
        // SQLite 3.35.0+ supports DROP COLUMN
        format!(
            "ALTER TABLE {} DROP COLUMN {}",
            self.quote_name(table),
            self.quote_name(column)
        )
    }

    fn rename_column_sql(&self, table: &str, old_name: &str, new_name: &str) -> String {
        // SQLite 3.25.0+ supports RENAME COLUMN
        format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {}",
            self.quote_name(table),
            self.quote_name(old_name),
            self.quote_name(new_name)
        )
    }

    fn create_index_sql(
        &self,
        name: &str,
        table: &str,
        columns: &[&str],
        unique: bool,
        condition: Option<&str>,
    ) -> String {
        let unique_str = if unique { "UNIQUE " } else { "" };
        let columns_str = columns
            .iter()
            .map(|col| self.quote_name(col))
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!(
            "CREATE {}INDEX {} ON {} ({})",
            unique_str,
            self.quote_name(name),
            self.quote_name(table),
            columns_str
        );

        if let Some(cond) = condition {
            sql.push_str(&format!(" WHERE {}", cond));
        }

        sql
    }

    fn drop_index_sql(&self, name: &str) -> String {
        // SQLite's DROP INDEX doesn't require table name
        format!("DROP INDEX {}", self.quote_name(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_name() {
        let editor = SQLiteSchemaEditor::new();
        assert_eq!(editor.quote_name("users"), "\"users\"");
        assert_eq!(editor.quote_name("user_name"), "\"user_name\"");
    }

    #[test]
    fn test_quote_value() {
        let editor = SQLiteSchemaEditor::new();
        assert_eq!(editor.quote_value("hello"), "'hello'");
        assert_eq!(editor.quote_value("it's"), "'it''s'");
    }

    #[test]
    fn test_create_table_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql =
            editor.create_table_sql("users", &[("id", "INTEGER PRIMARY KEY"), ("name", "TEXT")]);
        assert!(sql.contains("CREATE TABLE \"users\""));
        assert!(sql.contains("\"id\" INTEGER PRIMARY KEY"));
        assert!(sql.contains("\"name\" TEXT"));
    }

    #[test]
    fn test_drop_table_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.drop_table_sql("users", false);
        assert_eq!(sql, "DROP TABLE \"users\"");
    }

    #[test]
    fn test_drop_table_sql_cascade() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.drop_table_sql("users", true);
        assert!(sql.contains("DROP TABLE \"users\""));
        assert!(sql.contains("CASCADE not supported"));
    }

    #[test]
    fn test_add_column_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.add_column_sql("users", "email", "TEXT");
        assert_eq!(sql, "ALTER TABLE \"users\" ADD COLUMN \"email\" TEXT");
    }

    #[test]
    fn test_drop_column_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.drop_column_sql("users", "email");
        assert_eq!(sql, "ALTER TABLE \"users\" DROP COLUMN \"email\"");
    }

    #[test]
    fn test_alter_column_note() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.alter_column_note("users");
        assert!(sql.contains("SQLite does not support ALTER COLUMN"));
        assert!(sql.contains("\"users\""));
    }

    #[test]
    fn test_rename_column_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.rename_column_sql("users", "name", "full_name");
        assert_eq!(
            sql,
            "ALTER TABLE \"users\" RENAME COLUMN \"name\" TO \"full_name\""
        );
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

    #[test]
    fn test_create_index_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.create_index_sql("idx_email", "users", &["email"], false, None);
        assert_eq!(sql, "CREATE INDEX \"idx_email\" ON \"users\" (\"email\")");
    }

    #[test]
    fn test_create_unique_index_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.create_index_sql("idx_email_unique", "users", &["email"], true, None);
        assert_eq!(
            sql,
            "CREATE UNIQUE INDEX \"idx_email_unique\" ON \"users\" (\"email\")"
        );
    }

    #[test]
    fn test_create_index_sql_with_condition() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.create_index_sql(
            "idx_active_users",
            "users",
            &["email"],
            false,
            Some("active = 1"),
        );
        assert_eq!(
            sql,
            "CREATE INDEX \"idx_active_users\" ON \"users\" (\"email\") WHERE active = 1"
        );
    }

    #[test]
    fn test_drop_index_sql() {
        let editor = SQLiteSchemaEditor::new();
        let sql = editor.drop_index_sql("idx_email");
        assert_eq!(sql, "DROP INDEX \"idx_email\"");
    }
}
