//! Model operations for migrations
//!
//! This module provides operations for creating, deleting, and renaming models,
//! inspired by Django's `django/db/migrations/operations/models.py`.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_migrations::operations::models::{CreateModel, DeleteModel};
//! use reinhardt_migrations::operations::FieldDefinition;
//! use reinhardt_migrations::ProjectState;
//!
//! let mut state = ProjectState::new();
//!
//! // Create a model
//! let create = CreateModel::new(
//!     "User",
//!     vec![
//!         FieldDefinition::new("id", "INTEGER", true, false, None),
//!         FieldDefinition::new("email", "VARCHAR(255)", false, false, None),
//!     ],
//! );
//! create.state_forwards("myapp", &mut state);
//! assert!(state.get_model("myapp", "User").is_some());
//!
//! // Delete a model
//! let delete = DeleteModel::new("User");
//! delete.state_forwards("myapp", &mut state);
//! assert!(state.get_model("myapp", "User").is_none());
//! ```

use crate::{FieldState, ModelState, ProjectState};
use backends::schema::BaseDatabaseSchemaEditor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Field definition for model operations
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::FieldDefinition;
///
/// let field = FieldDefinition::new("email", "VARCHAR(255)", false, false, Some("''"));
/// assert_eq!(field.name, "email");
/// assert_eq!(field.field_type, "VARCHAR(255)");
/// assert!(!field.primary_key);
/// assert!(!field.unique);
/// assert_eq!(field.default, Some("''".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: String,
    pub primary_key: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub null: bool,
}

impl FieldDefinition {
    /// Create a new field definition
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::FieldDefinition;
    ///
    /// let field = FieldDefinition::new("id", "INTEGER", true, false, None);
    /// assert_eq!(field.name, "id");
    /// assert!(field.primary_key);
    /// ```
    pub fn new(
        name: impl Into<String>,
        field_type: impl Into<String>,
        primary_key: bool,
        unique: bool,
        default: Option<impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            field_type: field_type.into(),
            primary_key,
            unique,
            default: default.map(|d| d.into()),
            null: false,
        }
    }

    /// Set nullable
    pub fn nullable(mut self, null: bool) -> Self {
        self.null = null;
        self
    }

    /// Generate SQL column definition
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::FieldDefinition;
    ///
    /// let field = FieldDefinition::new("email", "VARCHAR(255)", false, true, Some("''"));
    /// let sql = field.to_sql_definition();
    /// assert!(sql.contains("VARCHAR(255)"));
    /// assert!(sql.contains("UNIQUE"));
    /// assert!(sql.contains("DEFAULT ''"));
    /// ```
    pub fn to_sql_definition(&self) -> String {
        let mut parts = vec![self.field_type.clone()];

        if self.primary_key {
            parts.push("PRIMARY KEY".to_string());
        }

        if self.unique && !self.primary_key {
            parts.push("UNIQUE".to_string());
        }

        if !self.null && !self.primary_key {
            parts.push("NOT NULL".to_string());
        }

        if let Some(ref default) = self.default {
            parts.push(format!("DEFAULT {}", default));
        }

        parts.join(" ")
    }
}

/// Create a new model (table)
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::models::CreateModel;
/// use reinhardt_migrations::operations::FieldDefinition;
/// use reinhardt_migrations::ProjectState;
///
/// let mut state = ProjectState::new();
/// let create = CreateModel::new(
///     "User",
///     vec![
///         FieldDefinition::new("id", "INTEGER", true, false, None),
///         FieldDefinition::new("name", "VARCHAR(100)", false, false, None),
///     ],
/// );
///
/// create.state_forwards("myapp", &mut state);
/// let model = state.get_model("myapp", "User").unwrap();
/// assert_eq!(model.fields.len(), 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub options: HashMap<String, String>,
    pub bases: Vec<String>,
    /// Composite primary key fields (field names)
    pub composite_primary_key: Option<Vec<String>>,
}

impl CreateModel {
    /// Create a new CreateModel operation
    pub fn new(name: impl Into<String>, fields: Vec<FieldDefinition>) -> Self {
        Self {
            name: name.into(),
            fields,
            options: HashMap::new(),
            bases: vec![],
            composite_primary_key: None,
        }
    }

    /// Add model options
    pub fn with_options(mut self, options: HashMap<String, String>) -> Self {
        self.options = options;
        self
    }

    /// Add base classes for inheritance
    pub fn with_bases(mut self, bases: Vec<String>) -> Self {
        self.bases = bases;
        self
    }

    /// Set composite primary key
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::models::CreateModel;
    /// use reinhardt_migrations::operations::FieldDefinition;
    ///
    /// let create = CreateModel::new(
    ///     "post_tags",
    ///     vec![
    ///         FieldDefinition::new("post_id", "INTEGER", false, false, None),
    ///         FieldDefinition::new("tag_id", "INTEGER", false, false, None),
    ///     ],
    /// )
    /// .with_composite_primary_key(vec!["post_id".to_string(), "tag_id".to_string()]);
    ///
    /// assert!(create.composite_primary_key.is_some());
    /// ```
    pub fn with_composite_primary_key(mut self, fields: Vec<String>) -> Self {
        self.composite_primary_key = Some(fields);
        self
    }

    /// Apply to project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        let mut model = ModelState::new(app_label, &self.name);

        for field_def in &self.fields {
            let field = FieldState::new(
                field_def.name.clone(),
                field_def.field_type.clone(),
                field_def.primary_key,
            );
            model.add_field(field);
        }

        state.add_model(model);
    }

    /// Generate SQL using schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::models::CreateModel;
    /// use reinhardt_migrations::operations::FieldDefinition;
    ///
    /// let create = CreateModel::new(
    ///     "users",
    ///     vec![
    ///         FieldDefinition::new("id", "INTEGER", true, false, None),
    ///         FieldDefinition::new("email", "VARCHAR(255)", false, false, None),
    ///     ],
    /// );
    ///
    // Convert to SQL - actual DB operations would use schema editor
    /// let columns: Vec<(&str, String)> = create.fields
    ///     .iter()
    ///     .map(|f| (f.name.as_str(), f.to_sql_definition()))
    ///     .collect();
    ///
    /// assert_eq!(columns.len(), 2);
    /// assert_eq!(columns[0].0, "id");
    /// assert!(columns[0].1.contains("PRIMARY KEY"));
    /// ```
    pub fn database_forwards(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        let mut sql_statements = Vec::new();

        // If composite primary key is defined, don't mark individual fields as primary keys
        let has_composite_pk = self.composite_primary_key.is_some();

        // Convert field definitions to column specifications for schema editor
        let column_defs: Vec<String> = self
            .fields
            .iter()
            .map(|f| {
                // For composite PKs, don't add PRIMARY KEY to individual field definitions
                if has_composite_pk && f.primary_key {
                    let mut parts = vec![f.field_type.clone()];
                    if f.unique {
                        parts.push("UNIQUE".to_string());
                    }
                    if !f.null {
                        parts.push("NOT NULL".to_string());
                    }
                    if let Some(ref default) = f.default {
                        parts.push(format!("DEFAULT {}", default));
                    }
                    parts.join(" ")
                } else {
                    f.to_sql_definition()
                }
            })
            .collect();

        // Build column pairs: (name, type_definition)
        let columns: Vec<(&str, &str)> = self
            .fields
            .iter()
            .zip(column_defs.iter())
            .map(|(f, def)| (f.name.as_str(), def.as_str()))
            .collect();

        // Generate CREATE TABLE SQL
        let mut create_sql = schema_editor.create_table_sql(&self.name, &columns);

        // Add composite primary key constraint if defined
        if let Some(ref pk_fields) = self.composite_primary_key {
            // Remove the closing parenthesis from create_sql
            if create_sql.ends_with(");") {
                create_sql = create_sql.trim_end_matches(");").to_string();
            } else if create_sql.ends_with(')') {
                create_sql = create_sql.trim_end_matches(')').to_string();
            }

            // Add composite primary key constraint
            let pk_constraint = format!(", PRIMARY KEY ({})", pk_fields.join(", "));
            create_sql.push_str(&pk_constraint);
            create_sql.push_str(");");
        }

        sql_statements.push(create_sql);
        sql_statements
    }
}

/// Delete a model (drop table)
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::models::{CreateModel, DeleteModel};
/// use reinhardt_migrations::operations::FieldDefinition;
/// use reinhardt_migrations::ProjectState;
///
/// let mut state = ProjectState::new();
///
// First create a model
/// let create = CreateModel::new(
///     "User",
///     vec![FieldDefinition::new("id", "INTEGER", true, false, None)],
/// );
/// create.state_forwards("myapp", &mut state);
/// assert!(state.get_model("myapp", "User").is_some());
///
// Then delete it
/// let delete = DeleteModel::new("User");
/// delete.state_forwards("myapp", &mut state);
/// assert!(state.get_model("myapp", "User").is_none());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteModel {
    pub name: String,
}

impl DeleteModel {
    /// Create a new DeleteModel operation
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Apply to project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        state.remove_model(app_label, &self.name);
    }

    /// Generate SQL using schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::models::DeleteModel;
    /// use backends::schema::factory::{SchemaEditorFactory, DatabaseType};
    ///
    /// let delete = DeleteModel::new("users");
    /// let factory = SchemaEditorFactory::new();
    /// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
    ///
    /// let sql = delete.database_forwards(editor.as_ref());
    /// assert_eq!(sql.len(), 1);
    /// assert!(sql[0].contains("DROP TABLE"));
    /// assert!(sql[0].contains("\"users\""));
    /// ```
    pub fn database_forwards(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        vec![schema_editor.drop_table_sql(&self.name, false)]
    }
}

/// Rename a model (rename table)
///
/// # Example
///
/// ```rust
/// use reinhardt_migrations::operations::models::{CreateModel, RenameModel};
/// use reinhardt_migrations::operations::FieldDefinition;
/// use reinhardt_migrations::ProjectState;
///
/// let mut state = ProjectState::new();
///
// Create a model
/// let create = CreateModel::new(
///     "User",
///     vec![FieldDefinition::new("id", "INTEGER", true, false, None)],
/// );
/// create.state_forwards("myapp", &mut state);
///
// Rename it
/// let rename = RenameModel::new("User", "Customer");
/// rename.state_forwards("myapp", &mut state);
///
/// assert!(state.get_model("myapp", "User").is_none());
/// assert!(state.get_model("myapp", "Customer").is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameModel {
    pub old_name: String,
    pub new_name: String,
}

impl RenameModel {
    /// Create a new RenameModel operation
    pub fn new(old_name: impl Into<String>, new_name: impl Into<String>) -> Self {
        Self {
            old_name: old_name.into(),
            new_name: new_name.into(),
        }
    }

    /// Apply to project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        state.rename_model(app_label, &self.old_name, self.new_name.clone());
    }

    /// Generate SQL using schema editor
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_migrations::operations::models::RenameModel;
    /// use backends::schema::factory::{SchemaEditorFactory, DatabaseType};
    ///
    /// let rename = RenameModel::new("users", "customers");
    /// let factory = SchemaEditorFactory::new();
    /// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
    ///
    /// let sql = rename.database_forwards(editor.as_ref());
    /// assert_eq!(sql.len(), 1);
    /// assert!(sql[0].contains("ALTER TABLE"));
    /// assert!(sql[0].contains("\"users\""));
    /// assert!(sql[0].contains("\"customers\""));
    /// ```
    pub fn database_forwards(&self, schema_editor: &dyn BaseDatabaseSchemaEditor) -> Vec<String> {
        // Note: BaseDatabaseSchemaEditor doesn't have rename_table_sql yet
        // We'll need to add that method or use a different approach
        vec![format!(
            "ALTER TABLE {} RENAME TO {}",
            schema_editor.quote_name(&self.old_name),
            schema_editor.quote_name(&self.new_name)
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_definition_to_sql() {
        let field = FieldDefinition::new("id", "INTEGER", true, false, None::<String>);
        let sql = field.to_sql_definition();
        assert!(sql.contains("INTEGER"));
        assert!(sql.contains("PRIMARY KEY"));

        let field2 = FieldDefinition::new("email", "VARCHAR(255)", false, true, Some("''"));
        let sql2 = field2.to_sql_definition();
        assert!(sql2.contains("VARCHAR(255)"));
        assert!(sql2.contains("UNIQUE"));
        assert!(sql2.contains("DEFAULT ''"));
        assert!(sql2.contains("NOT NULL"));
    }

    #[test]
    fn test_create_model_state_forwards() {
        let mut state = ProjectState::new();
        let create = CreateModel::new(
            "User",
            vec![
                FieldDefinition::new("id", "INTEGER", true, false, None::<String>),
                FieldDefinition::new("name", "VARCHAR(100)", false, false, None::<String>),
            ],
        );

        create.state_forwards("myapp", &mut state);

        let model = state.get_model("myapp", "User").unwrap();
        assert_eq!(model.name, "User");
        assert_eq!(model.fields.len(), 2);
        assert!(model.fields.contains_key("id"));
        assert!(model.fields.contains_key("name"));
    }

    #[test]
    fn test_delete_model_state_forwards() {
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

        // Delete it
        let delete = DeleteModel::new("User");
        delete.state_forwards("myapp", &mut state);
        assert!(state.get_model("myapp", "User").is_none());
    }

    #[test]
    fn test_rename_model_state_forwards() {
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

        // Rename it
        let rename = RenameModel::new("User", "Customer");
        rename.state_forwards("myapp", &mut state);

        assert!(state.get_model("myapp", "User").is_none());
        let model = state.get_model("myapp", "Customer").unwrap();
        assert_eq!(model.name, "Customer");
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_delete_model_database_forwards() {
        use backends::schema::factory::{DatabaseType, SchemaEditorFactory};

        let delete = DeleteModel::new("users");
        let factory = SchemaEditorFactory::new();
        let editor = factory.create_for_database(DatabaseType::PostgreSQL);

        let sql = delete.database_forwards(editor.as_ref());
        assert_eq!(sql.len(), 1);
        assert!(sql[0].contains("DROP TABLE"));
        assert!(sql[0].contains("\"users\""));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_rename_model_database_forwards() {
        use backends::schema::factory::{DatabaseType, SchemaEditorFactory};

        let rename = RenameModel::new("users", "customers");
        let factory = SchemaEditorFactory::new();
        let editor = factory.create_for_database(DatabaseType::PostgreSQL);

        let sql = rename.database_forwards(editor.as_ref());
        assert_eq!(sql.len(), 1);
        assert!(sql[0].contains("ALTER TABLE"));
        assert!(sql[0].contains("\"users\""));
        assert!(sql[0].contains("\"customers\""));
    }

    #[test]
    fn test_field_definition_nullable() {
        let field = FieldDefinition::new("email", "VARCHAR(255)", false, false, None::<String>)
            .nullable(true);

        assert!(field.null);
        let sql = field.to_sql_definition();
        assert!(!sql.contains("NOT NULL"));
    }

    #[test]
    fn test_create_model_with_options() {
        let mut options = HashMap::new();
        options.insert("db_table".to_string(), "custom_users".to_string());

        let create = CreateModel::new(
            "User",
            vec![FieldDefinition::new(
                "id",
                "INTEGER",
                true,
                false,
                None::<String>,
            )],
        )
        .with_options(options.clone());

        assert_eq!(create.options, options);
        assert_eq!(
            create.options.get("db_table"),
            Some(&"custom_users".to_string())
        );
    }

    #[test]
    fn test_create_model_with_bases() {
        let bases = vec!["BaseModel".to_string(), "Timestamped".to_string()];

        let create = CreateModel::new(
            "User",
            vec![FieldDefinition::new(
                "id",
                "INTEGER",
                true,
                false,
                None::<String>,
            )],
        )
        .with_bases(bases.clone());

        assert_eq!(create.bases, bases);
        assert_eq!(create.bases.len(), 2);
    }

    #[test]
    fn test_create_model_multiple_fields() {
        let mut state = ProjectState::new();

        let create = CreateModel::new(
            "User",
            vec![
                FieldDefinition::new("id", "INTEGER", true, false, None::<String>),
                FieldDefinition::new("username", "VARCHAR(50)", false, true, None::<String>),
                FieldDefinition::new("email", "VARCHAR(255)", false, true, None::<String>),
                FieldDefinition::new("is_active", "BOOLEAN", false, false, Some("true")),
            ],
        );

        create.state_forwards("myapp", &mut state);

        let model = state.get_model("myapp", "User").unwrap();
        assert_eq!(model.fields.len(), 4);
        assert!(model.fields.contains_key("id"));
        assert!(model.fields.contains_key("username"));
        assert!(model.fields.contains_key("email"));
        assert!(model.fields.contains_key("is_active"));
    }

    #[test]
    fn test_field_definition_with_default() {
        let field = FieldDefinition::new("status", "VARCHAR(20)", false, false, Some("'pending'"));

        assert_eq!(field.default, Some("'pending'".to_string()));

        let sql = field.to_sql_definition();
        assert!(sql.contains("DEFAULT 'pending'"));
    }

    #[test]
    fn test_delete_model_removes_from_state() {
        let mut state = ProjectState::new();

        // Create multiple models
        let create1 = CreateModel::new(
            "User",
            vec![FieldDefinition::new(
                "id",
                "INTEGER",
                true,
                false,
                None::<String>,
            )],
        );
        let create2 = CreateModel::new(
            "Post",
            vec![FieldDefinition::new(
                "id",
                "INTEGER",
                true,
                false,
                None::<String>,
            )],
        );

        create1.state_forwards("myapp", &mut state);
        create2.state_forwards("myapp", &mut state);

        assert!(state.get_model("myapp", "User").is_some());
        assert!(state.get_model("myapp", "Post").is_some());

        // Delete only User
        let delete = DeleteModel::new("User");
        delete.state_forwards("myapp", &mut state);

        assert!(state.get_model("myapp", "User").is_none());
        assert!(state.get_model("myapp", "Post").is_some());
    }

    #[test]
    fn test_rename_model_preserves_fields() {
        let mut state = ProjectState::new();

        // Create a model with multiple fields
        let create = CreateModel::new(
            "User",
            vec![
                FieldDefinition::new("id", "INTEGER", true, false, None::<String>),
                FieldDefinition::new("name", "VARCHAR(100)", false, false, None::<String>),
            ],
        );
        create.state_forwards("myapp", &mut state);

        // Rename it
        let rename = RenameModel::new("User", "Account");
        rename.state_forwards("myapp", &mut state);

        // Check that fields are preserved
        let model = state.get_model("myapp", "Account").unwrap();
        assert_eq!(model.fields.len(), 2);
        assert!(model.fields.contains_key("id"));
        assert!(model.fields.contains_key("name"));
    }
}
