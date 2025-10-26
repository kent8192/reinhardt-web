//! Global model registry
//!
//! This module provides a global registry for models, allowing models to be
//! discovered and registered at compile time using the `linkme` crate.
//!
//! # Examples
//!
//! ```rust
//! use reinhardt_apps::registry::{ModelMetadata, get_registered_models};
//!
//! // Register a model (typically done via derive macro)
//! #[linkme::distributed_slice(reinhardt_apps::registry::MODELS)]
//! static MY_MODEL: ModelMetadata = ModelMetadata {
//!     app_label: "myapp",
//!     model_name: "User",
//!     table_name: "users",
//! };
//!
//! // Access registered models
//! let models = get_registered_models();
//! // Note: In doc tests, the model may not be visible due to linkme limitations
//! ```

use linkme::distributed_slice;
use std::sync::RwLock;

/// Metadata for a registered model
///
/// This structure contains essential information about a model that has been
/// registered in the global model registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMetadata {
    /// The label of the application this model belongs to
    pub app_label: &'static str,

    /// The name of the model (e.g., "User", "Post")
    pub model_name: &'static str,

    /// The database table name for this model
    pub table_name: &'static str,
}

impl ModelMetadata {
    /// Create a new model metadata instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_apps::registry::ModelMetadata;
    ///
    /// let metadata = ModelMetadata::new("myapp", "User", "users");
    /// assert_eq!(metadata.app_label, "myapp");
    /// assert_eq!(metadata.model_name, "User");
    /// assert_eq!(metadata.table_name, "users");
    /// ```
    pub const fn new(
        app_label: &'static str,
        model_name: &'static str,
        table_name: &'static str,
    ) -> Self {
        Self {
            app_label,
            model_name,
            table_name,
        }
    }

    /// Get the fully qualified model name (app_label.model_name)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_apps::registry::ModelMetadata;
    ///
    /// let metadata = ModelMetadata::new("myapp", "User", "users");
    /// assert_eq!(metadata.qualified_name(), "myapp.User");
    /// ```
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.app_label, self.model_name)
    }
}

/// Global distributed slice for model registration
///
/// This is the global registry where all models are collected at link time.
/// Models can be registered by adding items to this slice using the `#[distributed_slice]`
/// attribute from the `linkme` crate.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::{MODELS, ModelMetadata};
///
/// #[linkme::distributed_slice(MODELS)]
/// static MY_MODEL: ModelMetadata = ModelMetadata {
///     app_label: "myapp",
///     model_name: "User",
///     table_name: "users",
/// };
/// ```
#[distributed_slice]
pub static MODELS: [ModelMetadata];

/// Cache for model lookups by app label
///
/// This cache is lazily populated the first time models are queried by app label.
static MODEL_CACHE: RwLock<
    Option<std::collections::HashMap<&'static str, Vec<&'static ModelMetadata>>>,
> = RwLock::new(None);

/// Get all registered models
///
/// This function returns a slice of all models that have been registered
/// in the global model registry.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::get_registered_models;
///
/// let models = get_registered_models();
/// println!("Found {} registered models", models.len());
/// ```
pub fn get_registered_models() -> &'static [ModelMetadata] {
    &MODELS
}

/// Get models for a specific application
///
/// This function returns all models that belong to the specified application label.
/// Results are cached for performance on subsequent calls.
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::get_models_for_app;
///
/// let auth_models = get_models_for_app("auth");
/// for model in auth_models {
///     println!("Model: {}", model.model_name);
/// }
/// ```
pub fn get_models_for_app(app_label: &str) -> Vec<&'static ModelMetadata> {
    // Check if cache is initialized
    {
        let cache = MODEL_CACHE.read().unwrap();
        if let Some(ref cache_map) = *cache {
            if let Some(models) = cache_map.get(app_label) {
                return models.clone();
            }
        }
    }

    // Initialize cache if needed
    {
        let mut cache = MODEL_CACHE.write().unwrap();
        if cache.is_none() {
            let mut cache_map = std::collections::HashMap::new();
            for model in MODELS.iter() {
                cache_map
                    .entry(model.app_label)
                    .or_insert_with(Vec::new)
                    .push(model);
            }
            *cache = Some(cache_map);
        }
    }

    // Retrieve from cache
    let cache = MODEL_CACHE.read().unwrap();
    cache
        .as_ref()
        .unwrap()
        .get(app_label)
        .cloned()
        .unwrap_or_default()
}

/// Find a model by its qualified name (app_label.model_name)
///
/// # Examples
///
/// ```rust
/// use reinhardt_apps::registry::find_model;
///
/// if let Some(model) = find_model("myapp.User") {
///     println!("Found model: {}", model.model_name);
/// } else {
///     println!("Model not found");
/// }
/// ```
pub fn find_model(qualified_name: &str) -> Option<&'static ModelMetadata> {
    let parts: Vec<&str> = qualified_name.split('.').collect();
    if parts.len() != 2 {
        return None;
    }

    let (app_label, model_name) = (parts[0], parts[1]);
    MODELS
        .iter()
        .find(|m| m.app_label == app_label && m.model_name == model_name)
}

/// Clear the model cache (primarily for testing)
///
/// This function clears the internal cache used for model lookups.
/// It should primarily be used in test scenarios.
pub fn clear_model_cache() {
    let mut cache = MODEL_CACHE.write().unwrap();
    *cache = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test model registrations
    #[distributed_slice(MODELS)]
    static TEST_USER_MODEL: ModelMetadata = ModelMetadata {
        app_label: "auth",
        model_name: "User",
        table_name: "auth_users",
    };

    #[distributed_slice(MODELS)]
    static TEST_POST_MODEL: ModelMetadata = ModelMetadata {
        app_label: "blog",
        model_name: "Post",
        table_name: "blog_posts",
    };

    #[distributed_slice(MODELS)]
    static TEST_COMMENT_MODEL: ModelMetadata = ModelMetadata {
        app_label: "blog",
        model_name: "Comment",
        table_name: "blog_comments",
    };

    #[test]
    fn test_model_metadata_new() {
        let metadata = ModelMetadata::new("myapp", "MyModel", "my_table");
        assert_eq!(metadata.app_label, "myapp");
        assert_eq!(metadata.model_name, "MyModel");
        assert_eq!(metadata.table_name, "my_table");
    }

    #[test]
    fn test_qualified_name() {
        let metadata = ModelMetadata::new("auth", "User", "users");
        assert_eq!(metadata.qualified_name(), "auth.User");
    }

    #[test]
    fn test_get_registered_models() {
        let models = get_registered_models();
        // Should have at least our test models
        assert!(models.len() >= 3);

        // Check that our test models are present
        assert!(models.iter().any(|m| m.model_name == "User"));
        assert!(models.iter().any(|m| m.model_name == "Post"));
        assert!(models.iter().any(|m| m.model_name == "Comment"));
    }

    #[test]
    fn test_get_models_for_app() {
        // Clear cache before test
        clear_model_cache();

        let blog_models = get_models_for_app("blog");
        assert_eq!(blog_models.len(), 2);

        let model_names: Vec<&str> = blog_models.iter().map(|m| m.model_name).collect();
        assert!(model_names.contains(&"Post"));
        assert!(model_names.contains(&"Comment"));

        let auth_models = get_models_for_app("auth");
        assert_eq!(auth_models.len(), 1);
        assert_eq!(auth_models[0].model_name, "User");
    }

    #[test]
    fn test_get_models_for_app_cached() {
        // Clear cache before test
        clear_model_cache();

        // First call - populates cache
        let models1 = get_models_for_app("blog");
        assert_eq!(models1.len(), 2);

        // Second call - should use cache
        let models2 = get_models_for_app("blog");
        assert_eq!(models2.len(), 2);

        // Results should be the same
        assert_eq!(models1.len(), models2.len());
    }

    #[test]
    fn test_get_models_for_nonexistent_app() {
        let models = get_models_for_app("nonexistent");
        assert_eq!(models.len(), 0);
    }

    #[test]
    fn test_find_model() {
        let model = find_model("auth.User");
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_name, "User");
        assert_eq!(model.unwrap().table_name, "auth_users");

        let model = find_model("blog.Post");
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_name, "Post");
    }

    #[test]
    fn test_find_model_invalid_format() {
        let model = find_model("InvalidFormat");
        assert!(model.is_none());

        let model = find_model("too.many.parts");
        assert!(model.is_none());
    }

    #[test]
    fn test_find_nonexistent_model() {
        let model = find_model("nonexistent.Model");
        assert!(model.is_none());
    }

    #[test]
    fn test_clear_model_cache() {
        // Populate cache
        let _ = get_models_for_app("blog");

        // Clear cache
        clear_model_cache();

        // Should still work (rebuilds cache)
        let models = get_models_for_app("blog");
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_model_metadata_equality() {
        let meta1 = ModelMetadata::new("app", "Model", "table");
        let meta2 = ModelMetadata::new("app", "Model", "table");
        let meta3 = ModelMetadata::new("app", "Other", "table");

        assert_eq!(meta1, meta2);
        assert_ne!(meta1, meta3);
    }
}
