//! Field operations for migrations
//!
//! This module provides operations for adding, removing, altering, and renaming fields,
//! inspired by Django's `django/db/migrations/operations/fields.py`.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_migrations::operations::fields::{AddField, RemoveField};
//! use reinhardt_migrations::operations::FieldDefinition;
//! use reinhardt_migrations::operations::models::CreateModel;
//! use reinhardt_migrations::ProjectState;
//!
//! let mut state = ProjectState::new();
//!
//! // Create a model first
//! let create = CreateModel::new(
//!     "User",
//!     vec![FieldDefinition::new("id", "INTEGER", true, false, None)],
//! );
//! create.state_forwards("myapp", &mut state);
//!
//! // Add a field
//! let add = AddField::new("User", FieldDefinition::new("email", "VARCHAR(255)", false, false, None));
//! add.state_forwards("myapp", &mut state);
//! assert_eq!(state.get_model("myapp", "User").unwrap().fields.len(), 2);
//!
//! // Remove a field
//! let remove = RemoveField::new("User", "email");
//! remove.state_forwards("myapp", &mut state);
//! assert_eq!(state.get_model("myapp", "User").unwrap().fields.len(), 1);
//! ```

use crate::{FieldState, ProjectState};
use backends::schema::BaseDatabaseSchemaEditor;
use serde::{Deserialize, Serialize};

pub use super::models::FieldDefinition;

/// Add a field to an existing model
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::fields::AddField;
/// use reinhardt_migrations::operations::FieldDefinition;
/// use reinhardt_migrations::operations::models::CreateModel;
/// use reinhardt_migrations::ProjectState;
///
/// let mut state = ProjectState::new();
///
/// // Create a model first
/// let create = CreateModel::new(
///     "User",
///     vec![FieldDefinition::new("id", "INTEGER", true, false, None)],
/// );
/// create.state_forwards("myapp", &mut state);
///
/// // Add a field
/// let add = AddField::new("User", FieldDefinition::new("email", "VARCHAR(255)", false, false, None));
/// add.state_forwards("myapp", &mut state);
///
/// let model = state.get_model("myapp", "User").unwrap();
/// assert_eq!(model.fields.len(), 2);
/// assert!(model.fields.contains_key("email"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddField {
    pub model_name: String,
    pub field: FieldDefinition,
    pub preserve_default: bool,
}

impl AddField {
    /// Create a new AddField operation
    pub fn new(model_name: impl Into<String>, field: FieldDefinition) -> Self {
        Self {
            model_name: model_name.into(),
            field,
            preserve_default: true,
        }
    }

    /// Set whether to preserve the default value after adding
    pub fn with_preserve_default(mut self, preserve: bool) -> Self {
        self.preserve_default = preserve;
        self
    }

    /// Apply to project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        if let Some(model) = state.get_model_mut(app_label, &self.model_name) {
            let field = FieldState::new(
                self.field.name.clone(),
                self.field.field_type.clone(),
                self.field.primary_key,
            );
            model.add_field(field);
        }
    }

    /// Generate SQL using schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::fields::AddField;
    /// use reinhardt_migrations::operations::FieldDefinition;
    /// use backends::schema::factory::{SchemaEditorFactory, DatabaseType};
    ///
    /// let add = AddField::new("users", FieldDefinition::new("email", "VARCHAR(255)", false, false, None));
    /// let factory = SchemaEditorFactory::new();
    /// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
    ///
    /// let sql = add.database_forwards(editor.as_ref());
    /// assert_eq!(sql.len(), 1);
    /// assert!(sql[0].contains("ALTER TABLE"));
    /// assert!(sql[0].contains("ADD COLUMN"));
    /// assert!(sql[0].contains("\"email\""));
    /// ```
    pub fn database_forwards(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        let definition = self.field.to_sql_definition();
        vec![schema_editor.add_column_sql(&self.model_name, &self.field.name, &definition)]
    }
}

/// Remove a field from a model
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::fields::RemoveField;
/// use reinhardt_migrations::operations::FieldDefinition;
/// use reinhardt_migrations::operations::models::CreateModel;
/// use reinhardt_migrations::ProjectState;
///
/// let mut state = ProjectState::new();
///
/// // Create a model with fields
/// let create = CreateModel::new(
///     "User",
///     vec![
///         FieldDefinition::new("id", "INTEGER", true, false, None),
///         FieldDefinition::new("email", "VARCHAR(255)", false, false, None),
///     ],
/// );
/// create.state_forwards("myapp", &mut state);
///
/// // Remove a field
/// let remove = RemoveField::new("User", "email");
/// remove.state_forwards("myapp", &mut state);
///
/// let model = state.get_model("myapp", "User").unwrap();
/// assert_eq!(model.fields.len(), 1);
/// assert!(!model.fields.contains_key("email"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveField {
    pub model_name: String,
    pub field_name: String,
}

impl RemoveField {
    /// Create a new RemoveField operation
    pub fn new(model_name: impl Into<String>, field_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            field_name: field_name.into(),
        }
    }

    /// Apply to project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        if let Some(model) = state.get_model_mut(app_label, &self.model_name) {
            model.remove_field(&self.field_name);
        }
    }

    /// Generate SQL using schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::fields::RemoveField;
    /// use backends::schema::factory::{SchemaEditorFactory, DatabaseType};
    ///
    /// let remove = RemoveField::new("users", "email");
    /// let factory = SchemaEditorFactory::new();
    /// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
    ///
    /// let sql = remove.database_forwards(editor.as_ref());
    /// assert_eq!(sql.len(), 1);
    /// assert!(sql[0].contains("ALTER TABLE"));
    /// assert!(sql[0].contains("DROP COLUMN"));
    /// assert!(sql[0].contains("\"email\""));
    /// ```
    pub fn database_forwards(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        vec![schema_editor.drop_column_sql(&self.model_name, &self.field_name)]
    }
}

/// Alter a field's definition
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::fields::AlterField;
/// use reinhardt_migrations::operations::FieldDefinition;
/// use reinhardt_migrations::operations::models::CreateModel;
/// use reinhardt_migrations::ProjectState;
///
/// let mut state = ProjectState::new();
///
/// // Create a model with a field
/// let create = CreateModel::new(
///     "User",
///     vec![
///         FieldDefinition::new("id", "INTEGER", true, false, None),
///         FieldDefinition::new("email", "VARCHAR(100)", false, false, None),
///     ],
/// );
/// create.state_forwards("myapp", &mut state);
///
/// // Alter the field to make it longer
/// let alter = AlterField::new("User", FieldDefinition::new("email", "VARCHAR(255)", false, false, None));
/// alter.state_forwards("myapp", &mut state);
///
/// let model = state.get_model("myapp", "User").unwrap();
/// let field = model.fields.get("email").unwrap();
/// assert_eq!(field.field_type, "VARCHAR(255)");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlterField {
    pub model_name: String,
    pub field: FieldDefinition,
}

impl AlterField {
    /// Create a new AlterField operation
    pub fn new(model_name: impl Into<String>, field: FieldDefinition) -> Self {
        Self {
            model_name: model_name.into(),
            field,
        }
    }

    /// Apply to project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        if let Some(model) = state.get_model_mut(app_label, &self.model_name) {
            let field = FieldState::new(
                self.field.name.clone(),
                self.field.field_type.clone(),
                self.field.primary_key,
            );
            model.alter_field(&self.field.name, field);
        }
    }

    /// Generate SQL using schema editor
    ///
    /// Note: Altering columns is database-specific and complex.
    /// This is a simplified version that may need enhancement.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::fields::AlterField;
    /// use reinhardt_migrations::operations::FieldDefinition;
    /// use backends::schema::factory::{SchemaEditorFactory, DatabaseType};
    ///
    /// let alter = AlterField::new("users", FieldDefinition::new("email", "VARCHAR(500)", false, false, None));
    /// let factory = SchemaEditorFactory::new();
    /// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
    ///
    /// let sql = alter.database_forwards(editor.as_ref());
    /// assert!(!sql.is_empty());
    /// ```
    pub fn database_forwards(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        // PostgreSQL: ALTER TABLE table ALTER COLUMN column TYPE type
        // MySQL: ALTER TABLE table MODIFY COLUMN column type
        // SQLite: Requires table recreation

        // For now, we'll generate PostgreSQL-style SQL
        // A proper implementation would check the database type
        vec![format!(
            "ALTER TABLE {} ALTER COLUMN {} TYPE {}",
            schema_editor.quote_name(&self.model_name),
            schema_editor.quote_name(&self.field.name),
            self.field.field_type
        )]
    }
}

/// Rename a field
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::fields::RenameField;
/// use reinhardt_migrations::operations::FieldDefinition;
/// use reinhardt_migrations::operations::models::CreateModel;
/// use reinhardt_migrations::ProjectState;
///
/// let mut state = ProjectState::new();
///
/// // Create a model with a field
/// let create = CreateModel::new(
///     "User",
///     vec![
///         FieldDefinition::new("id", "INTEGER", true, false, None),
///         FieldDefinition::new("email", "VARCHAR(255)", false, false, None),
///     ],
/// );
/// create.state_forwards("myapp", &mut state);
///
/// // Rename the field
/// let rename = RenameField::new("User", "email", "email_address");
/// rename.state_forwards("myapp", &mut state);
///
/// let model = state.get_model("myapp", "User").unwrap();
/// assert!(!model.fields.contains_key("email"));
/// assert!(model.fields.contains_key("email_address"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameField {
    pub model_name: String,
    pub old_name: String,
    pub new_name: String,
}

impl RenameField {
    /// Create a new RenameField operation
    pub fn new(
        model_name: impl Into<String>,
        old_name: impl Into<String>,
        new_name: impl Into<String>,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            old_name: old_name.into(),
            new_name: new_name.into(),
        }
    }

    /// Apply to project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        if let Some(model) = state.get_model_mut(app_label, &self.model_name) {
            model.rename_field(&self.old_name, self.new_name.clone());
        }
    }

    /// Generate SQL using schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::fields::RenameField;
    /// use backends::schema::factory::{SchemaEditorFactory, DatabaseType};
    ///
    /// let rename = RenameField::new("users", "email", "email_address");
    /// let factory = SchemaEditorFactory::new();
    /// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
    ///
    /// let sql = rename.database_forwards(editor.as_ref());
    /// assert_eq!(sql.len(), 1);
    /// assert!(sql[0].contains("ALTER TABLE"));
    /// assert!(sql[0].contains("RENAME COLUMN"));
    /// assert!(sql[0].contains("\"email\""));
    /// assert!(sql[0].contains("\"email_address\""));
    /// ```
    pub fn database_forwards(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        vec![schema_editor.rename_column_sql(&self.model_name, &self.old_name, &self.new_name)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::models::CreateModel;

    #[test]
    fn test_add_field_state_forwards() {
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

        // Add a field
        let add = AddField::new(
            "User",
            FieldDefinition::new("email", "VARCHAR(255)", false, false, None::<String>),
        );
        add.state_forwards("myapp", &mut state);

        let model = state.get_model("myapp", "User").unwrap();
        assert_eq!(model.fields.len(), 2);
        assert!(model.fields.contains_key("email"));
    }

    #[test]
    fn test_remove_field_state_forwards() {
        let mut state = ProjectState::new();

        // Create a model with fields
        let create = CreateModel::new(
            "User",
            vec![
                FieldDefinition::new("id", "INTEGER", true, false, None::<String>),
                FieldDefinition::new("email", "VARCHAR(255)", false, false, None::<String>),
            ],
        );
        create.state_forwards("myapp", &mut state);

        // Remove a field
        let remove = RemoveField::new("User", "email");
        remove.state_forwards("myapp", &mut state);

        let model = state.get_model("myapp", "User").unwrap();
        assert_eq!(model.fields.len(), 1);
        assert!(!model.fields.contains_key("email"));
    }

    #[test]
    fn test_alter_field_state_forwards() {
        let mut state = ProjectState::new();

        // Create a model with a field
        let create = CreateModel::new(
            "User",
            vec![
                FieldDefinition::new("id", "INTEGER", true, false, None::<String>),
                FieldDefinition::new("email", "VARCHAR(100)", false, false, None::<String>),
            ],
        );
        create.state_forwards("myapp", &mut state);

        // Alter the field
        let alter = AlterField::new(
            "User",
            FieldDefinition::new("email", "VARCHAR(255)", false, false, None::<String>),
        );
        alter.state_forwards("myapp", &mut state);

        let model = state.get_model("myapp", "User").unwrap();
        let field = model.fields.get("email").unwrap();
        assert_eq!(field.field_type, "VARCHAR(255)");
    }

    #[test]
    fn test_rename_field_state_forwards() {
        let mut state = ProjectState::new();

        // Create a model with a field
        let create = CreateModel::new(
            "User",
            vec![
                FieldDefinition::new("id", "INTEGER", true, false, None::<String>),
                FieldDefinition::new("email", "VARCHAR(255)", false, false, None::<String>),
            ],
        );
        create.state_forwards("myapp", &mut state);

        // Rename the field
        let rename = RenameField::new("User", "email", "email_address");
        rename.state_forwards("myapp", &mut state);

        let model = state.get_model("myapp", "User").unwrap();
        assert!(!model.fields.contains_key("email"));
        assert!(model.fields.contains_key("email_address"));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_add_field_database_forwards() {
        use backends::schema::factory::{DatabaseType, SchemaEditorFactory};

        let add = AddField::new(
            "users",
            FieldDefinition::new("email", "VARCHAR(255)", false, false, None::<String>),
        );
        let factory = SchemaEditorFactory::new();
        let editor = factory.create_for_database(DatabaseType::PostgreSQL);

        let sql = add.database_forwards(editor.as_ref());
        assert_eq!(sql.len(), 1);
        assert!(sql[0].contains("ALTER TABLE"));
        assert!(sql[0].contains("ADD COLUMN"));
        assert!(sql[0].contains("\"email\""));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_remove_field_database_forwards() {
        use backends::schema::factory::{DatabaseType, SchemaEditorFactory};

        let remove = RemoveField::new("users", "email");
        let factory = SchemaEditorFactory::new();
        let editor = factory.create_for_database(DatabaseType::PostgreSQL);

        let sql = remove.database_forwards(editor.as_ref());
        assert_eq!(sql.len(), 1);
        assert!(sql[0].contains("ALTER TABLE"));
        assert!(sql[0].contains("DROP COLUMN"));
        assert!(sql[0].contains("\"email\""));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_rename_field_database_forwards() {
        use backends::schema::factory::{DatabaseType, SchemaEditorFactory};

        let rename = RenameField::new("users", "email", "email_address");
        let factory = SchemaEditorFactory::new();
        let editor = factory.create_for_database(DatabaseType::PostgreSQL);

        let sql = rename.database_forwards(editor.as_ref());
        assert_eq!(sql.len(), 1);
        assert!(sql[0].contains("ALTER TABLE"));
        assert!(sql[0].contains("RENAME COLUMN"));
        assert!(sql[0].contains("\"email\""));
        assert!(sql[0].contains("\"email_address\""));
    }
}
