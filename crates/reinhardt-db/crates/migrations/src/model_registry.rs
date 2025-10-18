//! Global model registry for Reinhardt migrations
//!
//! This module provides a Django-like model registration system that allows
//! models to be registered globally and accessed during migration generation.
//!
//! # Django Reference
//! Django's app registry is implemented in `django/apps/registry.py` and provides:
//! - Global model registration via `Apps.register_model()`
//! - Model retrieval via `Apps.get_models()`
//! - Thread-safe access with RwLock
//!
//! # Architecture
//! ```text
//! Django:                         Reinhardt:
//! ┌─────────────┐                ┌──────────────────┐
//! │ Apps        │                │ ModelRegistry    │
//! │ ┌─────────┐ │                │ ┌──────────────┐ │
//! │ │all_models│ │                │ │models        │ │
//! │ └─────────┘ │                │ └──────────────┘ │
//! │ - register_ │                │ - register_model │
//! │   model()   │                │ - get_models()   │
//! │ - get_models│                │ - get_model()    │
//! └─────────────┘                └──────────────────┘
//! ```

use crate::autodetector::{FieldState, ModelState};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Model metadata for registration
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    /// Application label (e.g., "auth", "blog")
    pub app_label: String,
    /// Model name (e.g., "User", "Post")
    pub model_name: String,
    /// Table name (e.g., "auth_user", "blog_post")
    pub table_name: String,
    /// Field definitions
    pub fields: HashMap<String, FieldMetadata>,
    /// Model options (e.g., db_table, ordering)
    pub options: HashMap<String, String>,
}

impl ModelMetadata {
    pub fn new(
        app_label: impl Into<String>,
        model_name: impl Into<String>,
        table_name: impl Into<String>,
    ) -> Self {
        Self {
            app_label: app_label.into(),
            model_name: model_name.into(),
            table_name: table_name.into(),
            fields: HashMap::new(),
            options: HashMap::new(),
        }
    }

    pub fn add_field(&mut self, name: String, field: FieldMetadata) {
        self.fields.insert(name, field);
    }

    pub fn set_option(&mut self, key: String, value: String) {
        self.options.insert(key, value);
    }

    /// Convert to ModelState for migrations
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::model_registry::{ModelMetadata, FieldMetadata};
    ///
    /// let mut metadata = ModelMetadata::new("myapp", "User", "myapp_user");
    /// metadata.add_field(
    ///     "email".to_string(),
    ///     FieldMetadata::new("CharField").with_param("max_length", "255"),
    /// );
    ///
    /// let model_state = metadata.to_model_state();
    /// assert_eq!(model_state.app_label, "myapp");
    /// assert_eq!(model_state.name, "User");
    /// assert!(model_state.has_field("email"));
    /// ```
    pub fn to_model_state(&self) -> ModelState {
        let mut model_state = ModelState::new(&self.app_label, &self.model_name);

        // Convert fields
        for (name, field_meta) in &self.fields {
            let mut field_state = FieldState::new(
                name.clone(),
                field_meta.field_type.clone(),
                false, // nullable - default to false
            );
            for (key, value) in &field_meta.params {
                field_state.params.insert(key.clone(), value.clone());
            }
            model_state.add_field(field_state);
        }

        // Copy options
        model_state.options = self.options.clone();

        model_state
    }
}

/// Field metadata for registration
#[derive(Debug, Clone)]
pub struct FieldMetadata {
    /// Field type (e.g., "CharField", "IntegerField", "ForeignKey")
    pub field_type: String,
    /// Field parameters (max_length, null, blank, default, etc.)
    pub params: HashMap<String, String>,
}

impl FieldMetadata {
    pub fn new(field_type: impl Into<String>) -> Self {
        Self {
            field_type: field_type.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
}

/// Global model registry
///
/// This registry is thread-safe and can be accessed from anywhere in the application.
/// Models should register themselves during initialization, typically via derive macros.
///
/// # Django Equivalent
/// ```python
/// # Django: django/apps/registry.py
/// class Apps:
///     def __init__(self):
///         self.all_models = defaultdict(dict)  # {app_label: {model_name: model_class}}
///
///     def register_model(self, app_label, model):
///         model_name = model._meta.model_name
///         self.all_models[app_label][model_name] = model
///
///     def get_models(self, include_auto_created=False, include_swapped=False):
///         result = []
///         for app_config in self.app_configs.values():
///             result.extend(app_config.get_models(include_auto_created, include_swapped))
///         return result
/// ```
#[derive(Debug, Clone)]
pub struct ModelRegistry {
    /// Models: (app_label, model_name) -> ModelMetadata
    models: Arc<RwLock<HashMap<(String, String), ModelMetadata>>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a model in the registry
    ///
    /// # Django Reference
    /// From: django/apps/registry.py:215-240
    /// ```python
    /// def register_model(self, app_label, model):
    ///     model_name = model._meta.model_name
    ///     app_models = self.all_models[app_label]
    ///     if model_name in app_models:
    ///         # Handle conflicts...
    ///     app_models[model_name] = model
    /// ```
    pub fn register_model(&self, metadata: ModelMetadata) {
        let key = (metadata.app_label.clone(), metadata.model_name.clone());
        if let Ok(mut models) = self.models.write() {
            models.insert(key, metadata);
        }
    }

    /// Get all registered models
    ///
    /// # Django Reference
    /// From: django/apps/registry.py:169-186
    /// ```python
    /// def get_models(self, include_auto_created=False, include_swapped=False):
    ///     result = []
    ///     for app_config in self.app_configs.values():
    ///         result.extend(app_config.get_models(include_auto_created, include_swapped))
    ///     return result
    /// ```
    pub fn get_models(&self) -> Vec<ModelMetadata> {
        if let Ok(models) = self.models.read() {
            models.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get a specific model by app_label and model_name
    ///
    /// # Django Reference
    /// From: django/apps/registry.py:188-213
    /// ```python
    /// def get_model(self, app_label, model_name=None, require_ready=True):
    ///     if model_name is None:
    ///         app_label, model_name = app_label.split(".")
    ///     app_config = self.get_app_config(app_label)
    ///     return app_config.get_model(model_name, require_ready=require_ready)
    /// ```
    pub fn get_model(&self, app_label: &str, model_name: &str) -> Option<ModelMetadata> {
        if let Ok(models) = self.models.read() {
            models
                .get(&(app_label.to_string(), model_name.to_string()))
                .cloned()
        } else {
            None
        }
    }

    /// Get all models for a specific app
    pub fn get_app_models(&self, app_label: &str) -> Vec<ModelMetadata> {
        if let Ok(models) = self.models.read() {
            models
                .iter()
                .filter(|((app, _), _)| app == app_label)
                .map(|(_, meta)| meta.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Remove a model from the registry
    pub fn remove_model(&self, app_label: &str, model_name: &str) -> bool {
        if let Ok(mut models) = self.models.write() {
            models
                .remove(&(app_label.to_string(), model_name.to_string()))
                .is_some()
        } else {
            false
        }
    }

    /// Clear all registered models
    pub fn clear(&self) {
        if let Ok(mut models) = self.models.write() {
            models.clear();
        }
    }

    /// Get the count of registered models
    pub fn count(&self) -> usize {
        if let Ok(models) = self.models.read() {
            models.len()
        } else {
            0
        }
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global model registry instance
///
/// This is the primary way to access the model registry from anywhere in the application.
pub fn global_registry() -> &'static ModelRegistry {
    use once_cell::sync::Lazy;
    static REGISTRY: Lazy<ModelRegistry> = Lazy::new(ModelRegistry::new);
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_registry_new() {
        let registry = ModelRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_register_model() {
        let registry = ModelRegistry::new();
        let metadata = ModelMetadata::new("blog", "Post", "blog_post");
        registry.register_model(metadata);
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_get_model() {
        let registry = ModelRegistry::new();
        let metadata = ModelMetadata::new("auth", "User", "auth_user");
        registry.register_model(metadata);

        let retrieved = registry.get_model("auth", "User");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().table_name, "auth_user");
    }

    #[test]
    fn test_get_models() {
        let registry = ModelRegistry::new();
        registry.register_model(ModelMetadata::new("auth", "User", "auth_user"));
        registry.register_model(ModelMetadata::new("blog", "Post", "blog_post"));

        let models = registry.get_models();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_get_app_models() {
        let registry = ModelRegistry::new();
        registry.register_model(ModelMetadata::new("auth", "User", "auth_user"));
        registry.register_model(ModelMetadata::new("auth", "Group", "auth_group"));
        registry.register_model(ModelMetadata::new("blog", "Post", "blog_post"));

        let auth_models = registry.get_app_models("auth");
        assert_eq!(auth_models.len(), 2);

        let blog_models = registry.get_app_models("blog");
        assert_eq!(blog_models.len(), 1);
    }

    #[test]
    fn test_remove_model() {
        let registry = ModelRegistry::new();
        registry.register_model(ModelMetadata::new("auth", "User", "auth_user"));

        assert!(registry.remove_model("auth", "User"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_migrations_registry_clear() {
        let registry = ModelRegistry::new();
        registry.register_model(ModelMetadata::new("auth", "User", "auth_user"));
        registry.register_model(ModelMetadata::new("blog", "Post", "blog_post"));

        registry.clear();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_model_metadata_to_model_state() {
        let mut metadata = ModelMetadata::new("blog", "Post", "blog_post");

        let mut title_field = FieldMetadata::new("CharField");
        title_field
            .params
            .insert("max_length".to_string(), "200".to_string());
        metadata.add_field("title".to_string(), title_field);

        let model_state = metadata.to_model_state();
        assert_eq!(model_state.name, "Post");
        assert_eq!(model_state.fields.len(), 1);
        assert!(model_state.fields.contains_key("title"));
    }

    #[test]
    fn test_field_metadata_builder() {
        let field = FieldMetadata::new("CharField")
            .with_param("max_length", "100")
            .with_param("null", "False");

        assert_eq!(field.field_type, "CharField");
        assert_eq!(field.params.get("max_length").unwrap(), "100");
        assert_eq!(field.params.get("null").unwrap(), "False");
    }
}
