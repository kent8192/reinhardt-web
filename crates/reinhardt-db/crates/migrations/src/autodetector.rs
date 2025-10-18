//! Migration autodetector

/// Field state for migration detection
#[derive(Debug, Clone)]
pub struct FieldState {
    pub name: String,
    pub field_type: String,
    pub nullable: bool,
    pub params: std::collections::HashMap<String, String>,
}

impl FieldState {
    pub fn new(name: String, field_type: String, nullable: bool) -> Self {
        Self {
            name,
            field_type,
            nullable,
            params: std::collections::HashMap::new(),
        }
    }
}

/// Model state for migration detection
///
/// Django equivalent: `ModelState` in django/db/migrations/state.py
#[derive(Debug, Clone)]
pub struct ModelState {
    /// Application label (e.g., "auth", "blog")
    pub app_label: String,
    /// Model name (e.g., "User", "Post")
    pub name: String,
    /// Fields: field_name -> FieldState
    pub fields: std::collections::HashMap<String, FieldState>,
    /// Model options (db_table, ordering, etc.)
    pub options: std::collections::HashMap<String, String>,
    /// Base model for inheritance
    pub base_model: Option<String>,
    /// Inheritance type: "single_table" or "joined_table"
    pub inheritance_type: Option<String>,
    /// Discriminator column for single table inheritance
    pub discriminator_column: Option<String>,
    /// Indexes: index_name -> IndexDefinition
    pub indexes: Vec<IndexDefinition>,
    /// Constraints: constraint_name -> ConstraintDefinition
    pub constraints: Vec<ConstraintDefinition>,
}

/// Index definition for a model
#[derive(Debug, Clone, PartialEq)]
pub struct IndexDefinition {
    /// Index name
    pub name: String,
    /// Fields to index (in order)
    pub fields: Vec<String>,
    /// Whether this is a unique index
    pub unique: bool,
}

/// Constraint definition for a model
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDefinition {
    /// Constraint name
    pub name: String,
    /// Constraint type (e.g., "check", "unique", "foreign_key")
    pub constraint_type: String,
    /// Fields involved in the constraint
    pub fields: Vec<String>,
    /// Additional constraint expression (e.g., CHECK condition)
    pub expression: Option<String>,
}

impl ModelState {
    /// Create a new ModelState with app_label and name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::ModelState;
    ///
    /// let model = ModelState::new("myapp", "User");
    /// assert_eq!(model.app_label, "myapp");
    /// assert_eq!(model.name, "User");
    /// assert_eq!(model.fields.len(), 0);
    /// ```
    pub fn new(app_label: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            app_label: app_label.into(),
            name: name.into(),
            fields: std::collections::HashMap::new(),
            options: std::collections::HashMap::new(),
            base_model: None,
            inheritance_type: None,
            discriminator_column: None,
            indexes: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Add a field to this model
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ModelState, FieldState};
    ///
    /// let mut model = ModelState::new("myapp", "User");
    /// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    /// model.add_field(field);
    /// assert_eq!(model.fields.len(), 1);
    /// assert!(model.has_field("email"));
    /// ```
    pub fn add_field(&mut self, field: FieldState) {
        self.fields.insert(field.name.clone(), field);
    }

    /// Get a field by name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ModelState, FieldState};
    ///
    /// let mut model = ModelState::new("myapp", "User");
    /// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    /// model.add_field(field);
    ///
    /// let retrieved = model.get_field("email");
    /// assert!(retrieved.is_some());
    /// assert_eq!(retrieved.unwrap().field_type, "VARCHAR(255)");
    /// ```
    pub fn get_field(&self, name: &str) -> Option<&FieldState> {
        self.fields.get(name)
    }

    /// Check if a field exists
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ModelState, FieldState};
    ///
    /// let mut model = ModelState::new("myapp", "User");
    /// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    /// model.add_field(field);
    ///
    /// assert!(model.has_field("email"));
    /// assert!(!model.has_field("username"));
    /// ```
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    /// Rename a field
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ModelState, FieldState};
    ///
    /// let mut model = ModelState::new("myapp", "User");
    /// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    /// model.add_field(field);
    ///
    /// model.rename_field("email", "email_address".to_string());
    /// assert!(!model.has_field("email"));
    /// assert!(model.has_field("email_address"));
    /// ```
    pub fn rename_field(&mut self, old_name: &str, new_name: String) {
        if let Some(mut field) = self.fields.remove(old_name) {
            field.name = new_name.clone();
            self.fields.insert(new_name, field);
        }
    }
}

/// Project state for migration detection
///
/// Django equivalent: `ProjectState` in django/db/migrations/state.py
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::{ProjectState, ModelState, FieldState};
///
/// let mut state = ProjectState::new();
/// let mut model = ModelState::new("myapp", "User");
/// model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
/// state.add_model(model);
///
/// assert!(state.get_model("myapp", "User").is_some());
/// ```
#[derive(Debug, Clone)]
pub struct ProjectState {
    /// Models: (app_label, model_name) -> ModelState
    pub models: std::collections::HashMap<(String, String), ModelState>,
}

impl ProjectState {
    /// Create a new empty ProjectState
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::ProjectState;
    ///
    /// let state = ProjectState::new();
    /// assert_eq!(state.models.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            models: std::collections::HashMap::new(),
        }
    }

    /// Add a model to this project state
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ProjectState, ModelState};
    ///
    /// let mut state = ProjectState::new();
    /// let model = ModelState::new("myapp", "User");
    /// state.add_model(model);
    ///
    /// assert_eq!(state.models.len(), 1);
    /// assert!(state.get_model("myapp", "User").is_some());
    /// ```
    pub fn add_model(&mut self, model: ModelState) {
        let key = (model.app_label.clone(), model.name.clone());
        self.models.insert(key, model);
    }

    /// Get a model by app_label and model_name
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ProjectState, ModelState};
    ///
    /// let mut state = ProjectState::new();
    /// let model = ModelState::new("myapp", "User");
    /// state.add_model(model);
    ///
    /// let retrieved = state.get_model("myapp", "User");
    /// assert!(retrieved.is_some());
    /// assert_eq!(retrieved.unwrap().name, "User");
    /// ```
    pub fn get_model(&self, app_label: &str, model_name: &str) -> Option<&ModelState> {
        self.models
            .get(&(app_label.to_string(), model_name.to_string()))
    }

    /// Get a mutable reference to a model
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ProjectState, ModelState, FieldState};
    ///
    /// let mut state = ProjectState::new();
    /// let model = ModelState::new("myapp", "User");
    /// state.add_model(model);
    ///
    /// if let Some(model) = state.get_model_mut("myapp", "User") {
    ///     let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    ///     model.add_field(field);
    /// }
    ///
    /// assert!(state.get_model("myapp", "User").unwrap().has_field("email"));
    /// ```
    pub fn get_model_mut(&mut self, app_label: &str, model_name: &str) -> Option<&mut ModelState> {
        self.models
            .get_mut(&(app_label.to_string(), model_name.to_string()))
    }

    /// Remove a model from this project state
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ProjectState, ModelState};
    ///
    /// let mut state = ProjectState::new();
    /// let model = ModelState::new("myapp", "User");
    /// state.add_model(model);
    ///
    /// state.remove_model("myapp", "User");
    /// assert!(state.get_model("myapp", "User").is_none());
    /// ```
    pub fn remove_model(&mut self, app_label: &str, model_name: &str) {
        self.models
            .remove(&(app_label.to_string(), model_name.to_string()));
    }

    /// Rename a model
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ProjectState, ModelState};
    ///
    /// let mut state = ProjectState::new();
    /// let model = ModelState::new("myapp", "User");
    /// state.add_model(model);
    ///
    /// state.rename_model("myapp", "User", "Account".to_string());
    /// assert!(state.get_model("myapp", "User").is_none());
    /// assert!(state.get_model("myapp", "Account").is_some());
    /// ```
    pub fn rename_model(&mut self, app_label: &str, old_name: &str, new_name: String) {
        if let Some(mut model) = self
            .models
            .remove(&(app_label.to_string(), old_name.to_string()))
        {
            model.name = new_name.clone();
            self.models.insert((app_label.to_string(), new_name), model);
        }
    }

    /// Load ProjectState from the global model registry
    ///
    /// Django equivalent: `ProjectState.from_apps()` in django/db/migrations/state.py:594-600
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::ProjectState;
    ///
    /// let state = ProjectState::from_global_registry();
    /// // state will contain all models registered in the global registry
    /// ```
    pub fn from_global_registry() -> Self {
        use crate::model_registry::global_registry;

        let registry = global_registry();
        let models_metadata = registry.get_models();

        let mut state = ProjectState::new();

        for metadata in models_metadata {
            let model_state = metadata.to_model_state();
            state.add_model(model_state);
        }

        state
    }
}

/// Migration autodetector
///
/// Django equivalent: `MigrationAutodetector` in django/db/migrations/autodetector.py
///
/// Detects schema changes between two ProjectStates and generates migrations.
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
///
/// let mut from_state = ProjectState::new();
/// let mut to_state = ProjectState::new();
///
/// // Add a new model to to_state
/// let mut model = ModelState::new("myapp", "User");
/// model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
/// to_state.add_model(model);
///
/// let detector = MigrationAutodetector::new(from_state, to_state);
/// let changes = detector.detect_changes();
///
/// // Should detect the new model creation
/// assert_eq!(changes.created_models.len(), 1);
/// ```
pub struct MigrationAutodetector {
    from_state: ProjectState,
    to_state: ProjectState,
}

/// Detected changes between two project states
#[derive(Debug, Clone, Default)]
pub struct DetectedChanges {
    /// Models that were created: (app_label, model_name)
    pub created_models: Vec<(String, String)>,
    /// Models that were deleted: (app_label, model_name)
    pub deleted_models: Vec<(String, String)>,
    /// Fields that were added: (app_label, model_name, field_name)
    pub added_fields: Vec<(String, String, String)>,
    /// Fields that were removed: (app_label, model_name, field_name)
    pub removed_fields: Vec<(String, String, String)>,
    /// Fields that were altered: (app_label, model_name, field_name)
    pub altered_fields: Vec<(String, String, String)>,
    /// Models that were renamed: (app_label, old_name, new_name)
    pub renamed_models: Vec<(String, String, String)>,
    /// Fields that were renamed: (app_label, model_name, old_name, new_name)
    pub renamed_fields: Vec<(String, String, String, String)>,
    /// Indexes that were added: (app_label, model_name, IndexDefinition)
    pub added_indexes: Vec<(String, String, IndexDefinition)>,
    /// Indexes that were removed: (app_label, model_name, index_name)
    pub removed_indexes: Vec<(String, String, String)>,
    /// Constraints that were added: (app_label, model_name, ConstraintDefinition)
    pub added_constraints: Vec<(String, String, ConstraintDefinition)>,
    /// Constraints that were removed: (app_label, model_name, constraint_name)
    pub removed_constraints: Vec<(String, String, String)>,
}

impl MigrationAutodetector {
    /// Create a new migration autodetector
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{MigrationAutodetector, ProjectState};
    ///
    /// let from_state = ProjectState::new();
    /// let to_state = ProjectState::new();
    ///
    /// let detector = MigrationAutodetector::new(from_state, to_state);
    /// ```
    pub fn new(from_state: ProjectState, to_state: ProjectState) -> Self {
        Self {
            from_state,
            to_state,
        }
    }

    /// Detect all changes between from_state and to_state
    ///
    /// Django equivalent: `_detect_changes()` in django/db/migrations/autodetector.py
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState};
    ///
    /// let from_state = ProjectState::new();
    /// let mut to_state = ProjectState::new();
    ///
    /// // Add a new model
    /// let model = ModelState::new("myapp", "User");
    /// to_state.add_model(model);
    ///
    /// let detector = MigrationAutodetector::new(from_state, to_state);
    /// let changes = detector.detect_changes();
    ///
    /// assert_eq!(changes.created_models.len(), 1);
    /// ```
    pub fn detect_changes(&self) -> DetectedChanges {
        let mut changes = DetectedChanges::default();

        // Detect model-level changes
        self.detect_created_models(&mut changes);
        self.detect_deleted_models(&mut changes);
        self.detect_renamed_models(&mut changes);

        // Detect field-level changes (only for models that exist in both states)
        self.detect_added_fields(&mut changes);
        self.detect_removed_fields(&mut changes);
        self.detect_altered_fields(&mut changes);
        self.detect_renamed_fields(&mut changes);

        // Detect index and constraint changes
        self.detect_added_indexes(&mut changes);
        self.detect_removed_indexes(&mut changes);
        self.detect_added_constraints(&mut changes);
        self.detect_removed_constraints(&mut changes);

        changes
    }

    /// Detect newly created models
    ///
    /// Django reference: `generate_created_models()` in django/db/migrations/autodetector.py:800
    fn detect_created_models(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), _model) in &self.to_state.models {
            if !self
                .from_state
                .models
                .contains_key(&(app_label.clone(), model_name.clone()))
            {
                changes
                    .created_models
                    .push((app_label.clone(), model_name.clone()));
            }
        }
    }

    /// Detect deleted models
    ///
    /// Django reference: `generate_deleted_models()` in django/db/migrations/autodetector.py:900
    fn detect_deleted_models(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), _model) in &self.from_state.models {
            if !self
                .to_state
                .models
                .contains_key(&(app_label.clone(), model_name.clone()))
            {
                changes
                    .deleted_models
                    .push((app_label.clone(), model_name.clone()));
            }
        }
    }

    /// Detect added fields
    ///
    /// Django reference: `generate_added_fields()` in django/db/migrations/autodetector.py:1000
    fn detect_added_fields(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), to_model) in &self.to_state.models {
            // Only check models that exist in both states
            if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
                for (field_name, _field) in &to_model.fields {
                    if !from_model.fields.contains_key(field_name) {
                        changes.added_fields.push((
                            app_label.clone(),
                            model_name.clone(),
                            field_name.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Detect removed fields
    ///
    /// Django reference: `generate_removed_fields()` in django/db/migrations/autodetector.py:1100
    fn detect_removed_fields(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), from_model) in &self.from_state.models {
            // Only check models that exist in both states
            if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
                for (field_name, _field) in &from_model.fields {
                    if !to_model.fields.contains_key(field_name) {
                        changes.removed_fields.push((
                            app_label.clone(),
                            model_name.clone(),
                            field_name.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Detect altered fields
    ///
    /// Django reference: `generate_altered_fields()` in django/db/migrations/autodetector.py:1200
    fn detect_altered_fields(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), to_model) in &self.to_state.models {
            // Only check models that exist in both states
            if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
                for (field_name, to_field) in &to_model.fields {
                    if let Some(from_field) = from_model.fields.get(field_name) {
                        // Check if field definition has changed
                        if self.has_field_changed(from_field, to_field) {
                            changes.altered_fields.push((
                                app_label.clone(),
                                model_name.clone(),
                                field_name.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Check if a field has changed
    fn has_field_changed(&self, from_field: &FieldState, to_field: &FieldState) -> bool {
        // Check if field type changed
        if from_field.field_type != to_field.field_type {
            return true;
        }

        // Check if nullable changed
        if from_field.nullable != to_field.nullable {
            return true;
        }

        // Check if params changed
        if from_field.params != to_field.params {
            return true;
        }

        false
    }

    /// Detect renamed models
    ///
    /// This method attempts to detect model renames by comparing deleted and created models.
    /// It uses field similarity to determine if a model was renamed rather than deleted/created.
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:620-750
    /// ```python
    /// def generate_renamed_models(self):
    ///     # Find models that were deleted and created with similar fields
    ///     for (app_label, old_model_name) in self.old_model_keys - self.new_model_keys:
    ///         for (app_label, new_model_name) in self.new_model_keys - self.old_model_keys:
    ///             if self._is_renamed_model(old_model_name, new_model_name):
    ///                 self.add_operation(
    ///                     app_label,
    ///                     operations.RenameModel(
    ///                         old_name=old_model_name,
    ///                         new_name=new_model_name,
    ///                     ),
    ///                 )
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
    ///
    /// let mut from_state = ProjectState::new();
    /// let mut old_model = ModelState::new("myapp", "OldUser");
    /// old_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
    /// old_model.add_field(FieldState::new("name".to_string(), "VARCHAR".to_string(), false));
    /// from_state.add_model(old_model);
    ///
    /// let mut to_state = ProjectState::new();
    /// let mut new_model = ModelState::new("myapp", "NewUser");
    /// new_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
    /// new_model.add_field(FieldState::new("name".to_string(), "VARCHAR".to_string(), false));
    /// to_state.add_model(new_model);
    ///
    /// let detector = MigrationAutodetector::new(from_state, to_state);
    /// let changes = detector.detect_changes();
    ///
    /// // With high field similarity, should detect as rename
    /// assert!(changes.renamed_models.len() <= 1);
    /// ```
    fn detect_renamed_models(&self, changes: &mut DetectedChanges) {
        // Get deleted and created models
        let deleted: Vec<_> = self
            .from_state
            .models
            .keys()
            .filter(|k| !self.to_state.models.contains_key(k))
            .collect();

        let created: Vec<_> = self
            .to_state
            .models
            .keys()
            .filter(|k| !self.from_state.models.contains_key(k))
            .collect();

        // Try to match deleted models with created models
        for deleted_key in &deleted {
            if let Some(from_model) = self.from_state.models.get(deleted_key) {
                for created_key in &created {
                    // Only match within the same app
                    if deleted_key.0 != created_key.0 {
                        continue;
                    }

                    if let Some(to_model) = self.to_state.models.get(created_key) {
                        // Calculate field similarity (simple heuristic: matching field names)
                        let similarity = self.calculate_model_similarity(from_model, to_model);

                        // If similarity is high (>70%), consider it a rename
                        if similarity > 0.7 {
                            changes.renamed_models.push((
                                deleted_key.0.clone(),
                                deleted_key.1.clone(),
                                created_key.1.clone(),
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Detect renamed fields
    ///
    /// This method attempts to detect field renames by comparing removed and added fields.
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:1300-1400
    /// ```python
    /// def generate_renamed_fields(self):
    ///     for app_label, model_name in sorted(self.kept_model_keys):
    ///         old_model_state = self.from_state.models[app_label, model_name]
    ///         new_model_state = self.to_state.models[app_label, model_name]
    ///
    ///         # Find fields that were removed and added with same type
    ///         for old_field_name, old_field in old_model_state.fields:
    ///             for new_field_name, new_field in new_model_state.fields:
    ///                 if self._is_renamed_field(old_field, new_field):
    ///                     self.add_operation(...)
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
    ///
    /// let mut from_state = ProjectState::new();
    /// let mut old_model = ModelState::new("myapp", "User");
    /// old_model.add_field(FieldState::new("old_email".to_string(), "VARCHAR".to_string(), false));
    /// from_state.add_model(old_model);
    ///
    /// let mut to_state = ProjectState::new();
    /// let mut new_model = ModelState::new("myapp", "User");
    /// new_model.add_field(FieldState::new("new_email".to_string(), "VARCHAR".to_string(), false));
    /// to_state.add_model(new_model);
    ///
    /// let detector = MigrationAutodetector::new(from_state, to_state);
    /// let changes = detector.detect_changes();
    ///
    /// // With matching type, might detect as rename
    /// assert!(changes.renamed_fields.len() <= 1);
    /// ```
    fn detect_renamed_fields(&self, changes: &mut DetectedChanges) {
        // Only check models that exist in both states
        for ((app_label, model_name), from_model) in &self.from_state.models {
            if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
                // Get removed and added fields for this model
                let removed_fields: Vec<_> = from_model
                    .fields
                    .iter()
                    .filter(|(name, _)| !to_model.fields.contains_key(*name))
                    .collect();

                let added_fields: Vec<_> = to_model
                    .fields
                    .iter()
                    .filter(|(name, _)| !from_model.fields.contains_key(*name))
                    .collect();

                // Try to match removed fields with added fields
                for (removed_name, removed_field) in &removed_fields {
                    for (added_name, added_field) in &added_fields {
                        // If field types match, consider it a rename
                        if removed_field.field_type == added_field.field_type
                            && removed_field.nullable == added_field.nullable
                        {
                            changes.renamed_fields.push((
                                app_label.clone(),
                                model_name.clone(),
                                removed_name.to_string(),
                                added_name.to_string(),
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Calculate similarity between two models based on their fields
    ///
    /// Returns a value between 0.0 and 1.0, where 1.0 means identical field sets.
    fn calculate_model_similarity(&self, from_model: &ModelState, to_model: &ModelState) -> f64 {
        if from_model.fields.is_empty() && to_model.fields.is_empty() {
            return 1.0;
        }

        if from_model.fields.is_empty() || to_model.fields.is_empty() {
            return 0.0;
        }

        // Count matching field names and types
        let mut matching_fields = 0;
        let mut total_fields = 0;

        for (field_name, from_field) in &from_model.fields {
            total_fields += 1;
            if let Some(to_field) = to_model.fields.get(field_name) {
                if from_field.field_type == to_field.field_type {
                    matching_fields += 1;
                }
            }
        }

        // Also count fields in to_model that aren't in from_model
        for field_name in to_model.fields.keys() {
            if !from_model.fields.contains_key(field_name) {
                total_fields += 1;
            }
        }

        if total_fields == 0 {
            return 1.0;
        }

        matching_fields as f64 / total_fields as f64
    }

    /// Detect added indexes
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:1500-1600
    fn detect_added_indexes(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), to_model) in &self.to_state.models {
            if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
                for to_index in &to_model.indexes {
                    // Check if this index exists in from_model
                    if !from_model
                        .indexes
                        .iter()
                        .any(|idx| idx.name == to_index.name)
                    {
                        changes.added_indexes.push((
                            app_label.clone(),
                            model_name.clone(),
                            to_index.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Detect removed indexes
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:1600-1700
    fn detect_removed_indexes(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), from_model) in &self.from_state.models {
            if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
                for from_index in &from_model.indexes {
                    // Check if this index still exists in to_model
                    if !to_model
                        .indexes
                        .iter()
                        .any(|idx| idx.name == from_index.name)
                    {
                        changes.removed_indexes.push((
                            app_label.clone(),
                            model_name.clone(),
                            from_index.name.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Detect added constraints
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:1700-1800
    fn detect_added_constraints(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), to_model) in &self.to_state.models {
            if let Some(from_model) = self.from_state.get_model(app_label, model_name) {
                for to_constraint in &to_model.constraints {
                    // Check if this constraint exists in from_model
                    if !from_model
                        .constraints
                        .iter()
                        .any(|c| c.name == to_constraint.name)
                    {
                        changes.added_constraints.push((
                            app_label.clone(),
                            model_name.clone(),
                            to_constraint.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Detect removed constraints
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:1800-1900
    fn detect_removed_constraints(&self, changes: &mut DetectedChanges) {
        for ((app_label, model_name), from_model) in &self.from_state.models {
            if let Some(to_model) = self.to_state.get_model(app_label, model_name) {
                for from_constraint in &from_model.constraints {
                    // Check if this constraint still exists in to_model
                    if !to_model
                        .constraints
                        .iter()
                        .any(|c| c.name == from_constraint.name)
                    {
                        changes.removed_constraints.push((
                            app_label.clone(),
                            model_name.clone(),
                            from_constraint.name.clone(),
                        ));
                    }
                }
            }
        }
    }

    /// Generate operations from detected changes
    ///
    /// Converts DetectedChanges into a list of Operation objects that can be
    /// executed to migrate the database schema.
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:1063-1164
    /// ```python
    /// def generate_created_models(self):
    ///     for app_label, model_name in sorted(self.new_model_keys):
    ///         model_state = self.to_state.models[app_label, model_name]
    ///         self.add_operation(
    ///             app_label,
    ///             operations.CreateModel(
    ///                 name=model_name,
    ///                 fields=model_state.fields,
    ///                 options=model_state.options,
    ///                 bases=model_state.bases,
    ///             ),
    ///         )
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
    ///
    /// let mut from_state = ProjectState::new();
    /// let mut to_state = ProjectState::new();
    ///
    /// // Add a new model to the target state
    /// let mut model = ModelState::new("myapp", "User");
    /// model.add_field(FieldState::new("id".to_string(), "IntegerField".to_string(), false));
    /// to_state.add_model(model);
    ///
    /// let detector = MigrationAutodetector::new(from_state, to_state);
    /// let operations = detector.generate_operations();
    ///
    /// assert!(!operations.is_empty());
    /// ```
    pub fn generate_operations(&self) -> Vec<crate::Operation> {
        let changes = self.detect_changes();
        let mut operations = Vec::new();

        // Generate CreateTable operations for new models
        for (app_label, model_name) in &changes.created_models {
            if let Some(model) = self.to_state.get_model(app_label, model_name) {
                let mut columns = Vec::new();
                for (field_name, field_state) in &model.fields {
                    columns.push(crate::ColumnDefinition {
                        name: field_name.clone(),
                        type_definition: field_state.field_type.clone(),
                    });
                }

                operations.push(crate::Operation::CreateTable {
                    name: model.name.clone(),
                    columns,
                    constraints: Vec::new(),
                });
            }
        }

        // Generate AddColumn operations for new fields
        for (app_label, model_name, field_name) in &changes.added_fields {
            if let Some(model) = self.to_state.get_model(app_label, model_name) {
                if let Some(field) = model.get_field(field_name) {
                    operations.push(crate::Operation::AddColumn {
                        table: model.name.clone(),
                        column: crate::ColumnDefinition {
                            name: field_name.clone(),
                            type_definition: field.field_type.clone(),
                        },
                    });
                }
            }
        }

        // Generate AlterColumn operations for changed fields
        for (app_label, model_name, field_name) in &changes.altered_fields {
            if let Some(model) = self.to_state.get_model(app_label, model_name) {
                if let Some(field) = model.get_field(field_name) {
                    operations.push(crate::Operation::AlterColumn {
                        table: model.name.clone(),
                        column: field_name.clone(),
                        new_definition: crate::ColumnDefinition {
                            name: field_name.clone(),
                            type_definition: field.field_type.clone(),
                        },
                    });
                }
            }
        }

        // Generate DropColumn operations for removed fields
        for (app_label, model_name, field_name) in &changes.removed_fields {
            if let Some(model) = self.from_state.get_model(app_label, model_name) {
                operations.push(crate::Operation::DropColumn {
                    table: model.name.clone(),
                    column: field_name.clone(),
                });
            }
        }

        // Generate DropTable operations for deleted models
        for (app_label, model_name) in &changes.deleted_models {
            if let Some(model) = self.from_state.get_model(app_label, model_name) {
                operations.push(crate::Operation::DropTable {
                    name: model.name.clone(),
                });
            }
        }

        operations
    }

    /// Generate migrations from detected changes
    ///
    /// Groups operations by app_label and creates Migration objects for each app.
    /// This is the final step in the migration autodetection process.
    ///
    /// # Django Reference
    /// From: django/db/migrations/autodetector.py:95-141
    /// ```python
    /// def changes(self, graph, trim_to_apps=None, convert_apps=None, migration_name=None):
    ///     # Generate operations
    ///     self._generate_through_model_map()
    ///     self.generate_renamed_models()
    ///     # ... all other generate_* methods
    ///
    ///     # Group operations by app
    ///     self.arrange_for_graph(changes, graph, trim_to_apps)
    ///
    ///     # Create Migration objects
    ///     return changes
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{MigrationAutodetector, ProjectState, ModelState, FieldState};
    ///
    /// let mut from_state = ProjectState::new();
    /// let mut to_state = ProjectState::new();
    ///
    /// // Add a new model
    /// let mut model = ModelState::new("blog", "Post");
    /// model.add_field(FieldState::new("title".to_string(), "CharField".to_string(), false));
    /// to_state.add_model(model);
    ///
    /// let detector = MigrationAutodetector::new(from_state, to_state);
    /// let migrations = detector.generate_migrations();
    ///
    /// assert_eq!(migrations.len(), 1);
    /// assert_eq!(migrations[0].app_label, "blog");
    /// assert!(!migrations[0].operations.is_empty());
    /// ```
    pub fn generate_migrations(&self) -> Vec<crate::Migration> {
        let changes = self.detect_changes();
        let mut migrations_by_app: std::collections::HashMap<String, Vec<crate::Operation>> =
            std::collections::HashMap::new();

        // Group created models by app
        for (app_label, model_name) in &changes.created_models {
            if let Some(model) = self.to_state.get_model(app_label, model_name) {
                let mut columns = Vec::new();
                for (field_name, field_state) in &model.fields {
                    columns.push(crate::ColumnDefinition {
                        name: field_name.clone(),
                        type_definition: field_state.field_type.clone(),
                    });
                }

                migrations_by_app
                    .entry(app_label.clone())
                    .or_default()
                    .push(crate::Operation::CreateTable {
                        name: model.name.clone(),
                        columns,
                        constraints: Vec::new(),
                    });
            }
        }

        // Group added fields by app
        for (app_label, model_name, field_name) in &changes.added_fields {
            if let Some(model) = self.to_state.get_model(app_label, model_name) {
                if let Some(field) = model.get_field(field_name) {
                    migrations_by_app
                        .entry(app_label.clone())
                        .or_default()
                        .push(crate::Operation::AddColumn {
                            table: model.name.clone(),
                            column: crate::ColumnDefinition {
                                name: field_name.clone(),
                                type_definition: field.field_type.clone(),
                            },
                        });
                }
            }
        }

        // Group altered fields by app
        for (app_label, model_name, field_name) in &changes.altered_fields {
            if let Some(model) = self.to_state.get_model(app_label, model_name) {
                if let Some(field) = model.get_field(field_name) {
                    migrations_by_app
                        .entry(app_label.clone())
                        .or_default()
                        .push(crate::Operation::AlterColumn {
                            table: model.name.clone(),
                            column: field_name.clone(),
                            new_definition: crate::ColumnDefinition {
                                name: field_name.clone(),
                                type_definition: field.field_type.clone(),
                            },
                        });
                }
            }
        }

        // Group removed fields by app
        for (app_label, model_name, field_name) in &changes.removed_fields {
            if let Some(model) = self.from_state.get_model(app_label, model_name) {
                migrations_by_app
                    .entry(app_label.clone())
                    .or_default()
                    .push(crate::Operation::DropColumn {
                        table: model.name.clone(),
                        column: field_name.clone(),
                    });
            }
        }

        // Group deleted models by app
        for (app_label, model_name) in &changes.deleted_models {
            if let Some(model) = self.from_state.get_model(app_label, model_name) {
                migrations_by_app
                    .entry(app_label.clone())
                    .or_default()
                    .push(crate::Operation::DropTable {
                        name: model.name.clone(),
                    });
            }
        }

        // Create Migration objects for each app
        let mut migrations = Vec::new();
        for (app_label, operations) in migrations_by_app {
            // Generate a simple migration name based on the first operation
            let migration_name = if let Some(op) = operations.first() {
                match op {
                    crate::Operation::CreateTable { name, .. } => {
                        format!("0001_initial_{}", name.to_lowercase())
                    }
                    crate::Operation::AddColumn { table, column } => {
                        format!(
                            "0001_add_{}_{}",
                            column.name.to_lowercase(),
                            table.to_lowercase()
                        )
                    }
                    crate::Operation::AlterColumn { table, column, .. } => format!(
                        "0001_alter_{}_{}",
                        column.to_lowercase(),
                        table.to_lowercase()
                    ),
                    crate::Operation::DropColumn { table, column } => {
                        format!(
                            "0001_remove_{}_{}",
                            column.to_lowercase(),
                            table.to_lowercase()
                        )
                    }
                    crate::Operation::DropTable { name } => {
                        format!("0001_delete_{}", name.to_lowercase())
                    }
                    _ => "0001_auto".to_string(),
                }
            } else {
                "0001_auto".to_string()
            };

            let mut migration = crate::Migration::new(&migration_name, &app_label);
            for operation in operations {
                migration = migration.add_operation(operation);
            }
            migrations.push(migration);
        }

        migrations
    }

    /// Legacy method for backward compatibility
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{MigrationAutodetector, ProjectState};
    ///
    /// let from_state = ProjectState::new();
    /// let to_state = ProjectState::new();
    ///
    /// let detector = MigrationAutodetector::new(from_state, to_state);
    /// let changes = detector.changes();
    /// ```
    #[deprecated(note = "Use detect_changes() instead")]
    pub fn changes(&self) -> Vec<String> {
        let detected = self.detect_changes();
        let mut result = Vec::new();

        for (app_label, model_name) in &detected.created_models {
            result.push(format!("Create model {}.{}", app_label, model_name));
        }

        for (app_label, model_name) in &detected.deleted_models {
            result.push(format!("Delete model {}.{}", app_label, model_name));
        }

        for (app_label, model_name, field_name) in &detected.added_fields {
            result.push(format!(
                "Add field {} to {}.{}",
                field_name, app_label, model_name
            ));
        }

        for (app_label, model_name, field_name) in &detected.removed_fields {
            result.push(format!(
                "Remove field {} from {}.{}",
                field_name, app_label, model_name
            ));
        }

        for (app_label, model_name, field_name) in &detected.altered_fields {
            result.push(format!(
                "Alter field {} on {}.{}",
                field_name, app_label, model_name
            ));
        }

        result
    }
}

impl ModelState {
    /// Remove a field from this model
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ModelState, FieldState};
    ///
    /// let mut model = ModelState::new("myapp", "User");
    /// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    /// model.add_field(field);
    /// assert!(model.has_field("email"));
    ///
    /// model.remove_field("email");
    /// assert!(!model.has_field("email"));
    /// ```
    pub fn remove_field(&mut self, name: &str) {
        self.fields.remove(name);
    }

    /// Alter a field definition
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{ModelState, FieldState};
    ///
    /// let mut model = ModelState::new("myapp", "User");
    /// let field = FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false);
    /// model.add_field(field);
    ///
    /// let new_field = FieldState::new("email".to_string(), "TEXT".to_string(), true);
    /// model.alter_field("email", new_field);
    ///
    /// let altered = model.get_field("email").unwrap();
    /// assert_eq!(altered.field_type, "TEXT");
    /// assert_eq!(altered.nullable, true);
    /// ```
    pub fn alter_field(&mut self, name: &str, new_field: FieldState) {
        self.fields.insert(name.to_string(), new_field);
    }
}
