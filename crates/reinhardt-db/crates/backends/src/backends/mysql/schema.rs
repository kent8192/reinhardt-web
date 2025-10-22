//! MySQL schema editor implementation
//!
//! This module provides MySQL-specific DDL operations through the `MySQLSchemaEditor`.

use crate::schema::{BaseDatabaseSchemaEditor, SchemaEditorError, SchemaEditorResult};

/// MySQL schema editor for DDL operations
///
/// Implements the `BaseDatabaseSchemaEditor` trait with MySQL-specific SQL syntax.
///
/// # Example
///
/// ```rust
/// use reinhardt_database::backends::mysql::schema::MySQLSchemaEditor;
/// use reinhardt_database::schema::BaseDatabaseSchemaEditor;
///
/// let editor = MySQLSchemaEditor::new();
///
// Create a table
/// let sql = editor.create_table_sql("users", &[
///     ("id", "INT PRIMARY KEY AUTO_INCREMENT"),
///     ("name", "VARCHAR(100)"),
/// ]);
/// assert!(sql.contains("CREATE TABLE"));
/// assert!(sql.contains("`users`"));
/// ```
#[derive(Debug, Default, Clone)]
pub struct MySQLSchemaEditor;

impl MySQLSchemaEditor {
    /// Create a new MySQL schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_database::backends::mysql::schema::MySQLSchemaEditor;
    ///
    /// let editor = MySQLSchemaEditor::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Generate ALTER COLUMN SQL (MySQL-specific)
    ///
    /// MySQL uses MODIFY COLUMN for altering columns
    pub fn alter_column_sql(&self, table_name: &str, column_name: &str, new_type: &str) -> String {
        format!(
            "ALTER TABLE {} MODIFY COLUMN {} {}",
            self.quote_name(table_name),
            self.quote_name(column_name),
            new_type
        )
    }

    /// Generate RENAME TABLE SQL (MySQL-specific)
    pub fn rename_table_sql(&self, old_name: &str, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {}",
            self.quote_name(old_name),
            self.quote_name(new_name)
        )
    }

    /// Generate ADD CONSTRAINT SQL (MySQL-specific)
    pub fn add_constraint_sql(&self, table_name: &str, constraint_sql: &str) -> String {
        format!(
            "ALTER TABLE {} ADD {}",
            self.quote_name(table_name),
            constraint_sql
        )
    }

    /// Generate DROP CONSTRAINT SQL (MySQL-specific)
    pub fn drop_constraint_sql(&self, table_name: &str, constraint_name: &str) -> String {
        format!(
            "ALTER TABLE {} DROP CONSTRAINT {}",
            self.quote_name(table_name),
            self.quote_name(constraint_name)
        )
    }
}

#[async_trait::async_trait]
impl BaseDatabaseSchemaEditor for MySQLSchemaEditor {
    async fn execute(&mut self, _sql: &str) -> SchemaEditorResult<()> {
        Err(SchemaEditorError::ExecutionError(
            "Execution not supported in schema editor".to_string(),
        ))
    }

    fn quote_name(&self, name: &str) -> String {
        format!("`{}`", name)
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
            "CREATE TABLE {} ({}) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci",
            self.quote_name(table),
            parts.join(", ")
        )
    }

    fn drop_table_sql(&self, table: &str, cascade: bool) -> String {
        let quoted_table = self.quote_name(table);
        if cascade {
            // MySQL doesn't have CASCADE for DROP TABLE, but we can use FOREIGN_KEY_CHECKS
            format!(
                "SET FOREIGN_KEY_CHECKS=0; DROP TABLE {}; SET FOREIGN_KEY_CHECKS=1",
                quoted_table
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
        format!(
            "ALTER TABLE {} DROP COLUMN {}",
            self.quote_name(table),
            self.quote_name(column)
        )
    }

    fn rename_column_sql(&self, table: &str, old_name: &str, new_name: &str) -> String {
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

        // MySQL doesn't support WHERE clause in CREATE INDEX
        if condition.is_some() {
            sql.push_str(" /* WHERE clause not supported in MySQL */");
        }

        sql
    }

    fn drop_index_sql(&self, name: &str) -> String {
        format!("DROP INDEX {}", self.quote_name(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_name() {
        let editor = MySQLSchemaEditor::new();
        assert_eq!(editor.quote_name("users"), "`users`");
        assert_eq!(editor.quote_name("user_name"), "`user_name`");
    }

    #[test]
    fn test_quote_value() {
        let editor = MySQLSchemaEditor::new();
        assert_eq!(editor.quote_value("hello"), "'hello'");
        assert_eq!(editor.quote_value("it's"), "'it''s'");
    }

    #[test]
    fn test_create_table_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.create_table_sql(
            "users",
            &[("id", "INT PRIMARY KEY"), ("name", "VARCHAR(100)")],
        );
        assert!(sql.contains("CREATE TABLE `users`"));
        assert!(sql.contains("`id` INT PRIMARY KEY"));
        assert!(sql.contains("`name` VARCHAR(100)"));
        assert!(sql.contains("ENGINE=InnoDB"));
        assert!(sql.contains("DEFAULT CHARSET=utf8mb4"));
    }

    #[test]
    fn test_drop_table_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.drop_table_sql("users", false);
        assert_eq!(sql, "DROP TABLE `users`");
    }

    #[test]
    fn test_drop_table_sql_cascade() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.drop_table_sql("users", true);
        assert!(sql.contains("SET FOREIGN_KEY_CHECKS=0"));
        assert!(sql.contains("DROP TABLE `users`"));
        assert!(sql.contains("SET FOREIGN_KEY_CHECKS=1"));
    }

    #[test]
    fn test_add_column_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.add_column_sql("users", "email", "VARCHAR(255)");
        assert_eq!(sql, "ALTER TABLE `users` ADD COLUMN `email` VARCHAR(255)");
    }

    #[test]
    fn test_drop_column_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.drop_column_sql("users", "email");
        assert_eq!(sql, "ALTER TABLE `users` DROP COLUMN `email`");
    }

    #[test]
    fn test_alter_column_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.alter_column_sql("users", "email", "TEXT");
        assert_eq!(sql, "ALTER TABLE `users` MODIFY COLUMN `email` TEXT");
    }

    #[test]
    fn test_rename_column_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.rename_column_sql("users", "name", "full_name");
        assert_eq!(
            sql,
            "ALTER TABLE `users` RENAME COLUMN `name` TO `full_name`"
        );
    }

    #[test]
    fn test_rename_table_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.rename_table_sql("users", "people");
        assert_eq!(sql, "ALTER TABLE `users` RENAME TO `people`");
    }

    #[test]
    fn test_add_constraint_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.add_constraint_sql("users", "UNIQUE (email)");
        assert_eq!(sql, "ALTER TABLE `users` ADD UNIQUE (email)");
    }

    #[test]
    fn test_drop_constraint_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.drop_constraint_sql("users", "unique_email");
        assert_eq!(sql, "ALTER TABLE `users` DROP CONSTRAINT `unique_email`");
    }

    #[test]
    fn test_create_index_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.create_index_sql("idx_email", "users", &["email"], false, None);
        assert_eq!(sql, "CREATE INDEX `idx_email` ON `users` (`email`)");
    }

    #[test]
    fn test_create_unique_index_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.create_index_sql("idx_email_unique", "users", &["email"], true, None);
        assert_eq!(
            sql,
            "CREATE UNIQUE INDEX `idx_email_unique` ON `users` (`email`)"
        );
    }

    #[test]
    fn test_drop_index_sql() {
        let editor = MySQLSchemaEditor::new();
        let sql = editor.drop_index_sql("idx_email");
        assert_eq!(sql, "DROP INDEX `idx_email`");
    }
}
