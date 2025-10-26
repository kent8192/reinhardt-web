//! Special operations for migrations
//!
//! This module provides special operations like RunSQL and RunCode (Rust equivalent of RunPython),
//! inspired by Django's `django/db/migrations/operations/special.py`.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_migrations::operations::special::RunSQL;
//!
//! // Create a RunSQL operation
//! let run_sql = RunSQL::new(
//!     "INSERT INTO users (name, email) VALUES ('admin', 'admin@example.com')",
//! ).with_reverse_sql("DELETE FROM users WHERE email = 'admin@example.com'");
//!
//! // Get forward SQL
//! assert_eq!(run_sql.sql, "INSERT INTO users (name, email) VALUES ('admin', 'admin@example.com')");
//! ```

use crate::ProjectState;
use backends::schema::BaseDatabaseSchemaEditor;
use serde::{Deserialize, Serialize};

/// Execute raw SQL
///
/// This operation allows you to execute arbitrary SQL statements during migration.
/// It's useful for data migrations, custom schema modifications, or database-specific operations.
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::special::RunSQL;
///
// Simple SQL execution
/// let sql = RunSQL::new("CREATE INDEX idx_email ON users(email)");
///
// With reverse SQL for rollback
/// let sql_reversible = RunSQL::new("CREATE INDEX idx_email ON users(email)")
///     .with_reverse_sql("DROP INDEX idx_email");
///
// Multiple statements
/// let multi_sql = RunSQL::new_multi(vec![
///     "INSERT INTO roles (name) VALUES ('admin')".to_string(),
///     "INSERT INTO roles (name) VALUES ('user')".to_string(),
/// ]);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSQL {
    pub sql: String,
    pub reverse_sql: Option<String>,
    pub state_operations: Vec<StateOperation>,
}

impl RunSQL {
    /// Create a new RunSQL operation
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::special::RunSQL;
    ///
    /// let sql = RunSQL::new("INSERT INTO config (key, value) VALUES ('version', '1.0')");
    /// assert!(sql.reverse_sql.is_none());
    /// ```
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            reverse_sql: None,
            state_operations: vec![],
        }
    }

    /// Create a RunSQL operation with multiple statements
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::special::RunSQL;
    ///
    /// let sql = RunSQL::new_multi(vec![
    ///     "UPDATE users SET active = true WHERE id = 1".to_string(),
    ///     "UPDATE users SET active = false WHERE id = 2".to_string(),
    /// ]);
    ///
    /// assert!(sql.sql.contains("UPDATE users SET active = true"));
    /// assert!(sql.sql.contains("UPDATE users SET active = false"));
    /// ```
    pub fn new_multi(statements: Vec<String>) -> Self {
        Self {
            sql: statements.join(";\n"),
            reverse_sql: None,
            state_operations: vec![],
        }
    }

    /// Set reverse SQL for rollback
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::special::RunSQL;
    ///
    /// let sql = RunSQL::new("CREATE INDEX idx_email ON users(email)")
    ///     .with_reverse_sql("DROP INDEX idx_email");
    ///
    /// assert!(sql.reverse_sql.is_some());
    /// assert_eq!(sql.reverse_sql.unwrap(), "DROP INDEX idx_email");
    /// ```
    pub fn with_reverse_sql(mut self, reverse_sql: impl Into<String>) -> Self {
        self.reverse_sql = Some(reverse_sql.into());
        self
    }

    /// Add state operations to be applied along with the SQL
    ///
    /// This allows you to keep the project state in sync when running custom SQL.
    pub fn with_state_operations(mut self, operations: Vec<StateOperation>) -> Self {
        self.state_operations = operations;
        self
    }

    /// Apply to project state (forward)
    ///
    /// RunSQL doesn't modify state by default unless state_operations are specified
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        for op in &self.state_operations {
            op.apply(app_label, state);
        }
    }

    /// Generate SQL
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::special::RunSQL;
    /// use backends::schema::factory::{SchemaEditorFactory, DatabaseType};
    ///
    /// let sql = RunSQL::new("SELECT 1");
    /// let factory = SchemaEditorFactory::new();
    /// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
    ///
    /// let statements = sql.database_forwards(editor.as_ref());
    /// assert_eq!(statements.len(), 1);
    /// assert_eq!(statements[0], "SELECT 1");
    /// ```
    pub fn database_forwards(&self, _schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        vec![self.sql.clone()]
    }

    /// Get reverse SQL for rollback
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::special::RunSQL;
    ///
    /// let sql = RunSQL::new("CREATE TABLE temp (id INT)")
    ///     .with_reverse_sql("DROP TABLE temp");
    ///
    /// assert_eq!(sql.get_reverse_sql(), Some("DROP TABLE temp"));
    ///
    /// let irreversible = RunSQL::new("DROP TABLE important_data");
    /// assert_eq!(irreversible.get_reverse_sql(), None);
    /// ```
    pub fn get_reverse_sql(&self) -> Option<&str> {
        self.reverse_sql.as_deref()
    }
}

/// State operation to apply alongside SQL
///
/// This allows RunSQL to update the project state appropriately
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateOperation {
    AddModel { name: String },
    RemoveModel { name: String },
    AddField { model: String, field: String },
    RemoveField { model: String, field: String },
}

impl StateOperation {
    fn apply(&self, app_label: &str, state: &mut ProjectState) {
        match self {
            StateOperation::AddModel { .. } => {
                // Would need model definition
            }
            StateOperation::RemoveModel { name } => {
                state.remove_model(app_label, name);
            }
            StateOperation::AddField { .. } => {
                // Would need field definition
            }
            StateOperation::RemoveField { model, field } => {
                if let Some(model_state) = state.get_model_mut(app_label, model) {
                    model_state.remove_field(field);
                }
            }
        }
    }
}

/// Execute Rust code during migration
///
/// This is the Rust equivalent of Django's RunPython. It allows you to execute
/// arbitrary Rust code during migration, useful for data transformations.
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::special::RunCode;
///
// Create a code operation with description
/// let code = RunCode::new("Update user emails", |_| {
///     // In a real implementation, this would receive database access
///     println!("Updating emails...");
///     Ok(())
/// });
/// ```
#[derive(Clone)]
pub struct RunCode {
    pub description: String,
    #[allow(clippy::type_complexity)]
    pub code: fn(&dyn BaseDatabaseSchemaEditor) -> Result<(), String>,
    #[allow(clippy::type_complexity)]
    pub reverse_code: Option<fn(&dyn BaseDatabaseSchemaEditor) -> Result<(), String>>,
}

impl std::fmt::Debug for RunCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunCode")
            .field("description", &self.description)
            .field("has_reverse", &self.reverse_code.is_some())
            .finish()
    }
}

impl RunCode {
    /// Create a new RunCode operation
    pub fn new(
        description: impl Into<String>,
        code: fn(&dyn BaseDatabaseSchemaEditor) -> Result<(), String>,
    ) -> Self {
        Self {
            description: description.into(),
            code,
            reverse_code: None,
        }
    }

    /// Set reverse code for rollback
    pub fn with_reverse_code(
        mut self,
        reverse: fn(&dyn BaseDatabaseSchemaEditor) -> Result<(), String>,
    ) -> Self {
        self.reverse_code = Some(reverse);
        self
    }

    /// Execute the code
    pub fn execute(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Result<(), String> {
        (self.code)(schema_editor)
    }

    /// Execute reverse code
    pub fn execute_reverse(
        &self,
        schema_editor: &dyn BaseDatabaseSchemaEditor,
    ) -> Result<(), String> {
        if let Some(reverse) = self.reverse_code {
            reverse(schema_editor)
        } else {
            Err("This operation is not reversible".to_string())
        }
    }

    /// Apply to project state (forward)
    ///
    /// RunCode doesn't modify state by default
    pub fn state_forwards(&self, _app_label: &str, _state: &mut ProjectState) {
        // Custom code operations don't modify state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_sql_basic() {
        let sql = RunSQL::new("CREATE INDEX idx_email ON users(email)");
        assert_eq!(sql.sql, "CREATE INDEX idx_email ON users(email)");
        assert!(sql.reverse_sql.is_none());
    }

    #[test]
    fn test_run_sql_with_reverse() {
        let sql = RunSQL::new("CREATE INDEX idx_email ON users(email)")
            .with_reverse_sql("DROP INDEX idx_email");

        assert_eq!(sql.sql, "CREATE INDEX idx_email ON users(email)");
        assert_eq!(sql.reverse_sql, Some("DROP INDEX idx_email".to_string()));
        assert_eq!(sql.get_reverse_sql(), Some("DROP INDEX idx_email"));
    }

    #[test]
    fn test_run_sql_multi() {
        let sql = RunSQL::new_multi(vec![
            "INSERT INTO roles (name) VALUES ('admin')".to_string(),
            "INSERT INTO roles (name) VALUES ('user')".to_string(),
        ]);

        assert!(
            sql.sql
                .contains("INSERT INTO roles (name) VALUES ('admin')")
        );
        assert!(sql.sql.contains("INSERT INTO roles (name) VALUES ('user')"));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_run_sql_database_forwards() {
        use backends::schema::factory::{DatabaseType, SchemaEditorFactory};

        let sql = RunSQL::new("SELECT COUNT(*) FROM users");
        let factory = SchemaEditorFactory::new();
        let editor = factory.create_for_database(DatabaseType::PostgreSQL);

        let statements = sql.database_forwards(editor.as_ref());
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "SELECT COUNT(*) FROM users");
    }

    #[test]
    fn test_run_code_basic() {
        fn migrate(_editor: &dyn BaseDatabaseSchemaEditor) -> Result<(), String> {
            Ok(())
        }

        let code = RunCode::new("Test migration", migrate);
        assert_eq!(code.description, "Test migration");
        assert!(code.reverse_code.is_none());
    }

    #[test]
    fn test_run_code_with_reverse() {
        fn migrate(_editor: &dyn BaseDatabaseSchemaEditor) -> Result<(), String> {
            Ok(())
        }

        fn reverse(_editor: &dyn BaseDatabaseSchemaEditor) -> Result<(), String> {
            Ok(())
        }

        let code = RunCode::new("Test migration", migrate).with_reverse_code(reverse);
        assert!(code.reverse_code.is_some());
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_run_code_execute() {
        use backends::schema::factory::{DatabaseType, SchemaEditorFactory};

        fn migrate(_editor: &dyn BaseDatabaseSchemaEditor) -> Result<(), String> {
            Ok(())
        }

        let code = RunCode::new("Test migration", migrate);
        let factory = SchemaEditorFactory::new();
        let editor = factory.create_for_database(DatabaseType::PostgreSQL);

        let result = code.execute(editor.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_operation_remove_model() {
        use crate::operations::FieldDefinition;
        use crate::operations::models::CreateModel;

        let mut state = ProjectState::new();

        // Create a model first
        let create = CreateModel::new(
            "User",
            vec![FieldDefinition::new(
                "id",
                "INTEGER",
                true,
                false,
                None::<String>,
            )],
        );
        create.state_forwards("myapp", &mut state);
        assert!(state.get_model("myapp", "User").is_some());

        // Remove it via state operation
        let op = StateOperation::RemoveModel {
            name: "User".to_string(),
        };
        op.apply("myapp", &mut state);
        assert!(state.get_model("myapp", "User").is_none());
    }
}
