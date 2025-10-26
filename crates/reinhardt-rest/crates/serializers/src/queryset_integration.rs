//! QuerySet integration for serializers
//!
//! This module provides integration between serializers and the ORM's QuerySet,
//! enabling seamless saving of validated data to the database.
//!
//! # Features
//!
//! - `SerializerSaveMixin` trait for database operations
//! - Pre-save validation hooks
//! - Error propagation from validators to save operations
//! - Transaction support for atomic operations

use crate::SerializerError;
use async_trait::async_trait;
use reinhardt_orm::Model;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Mixin trait for serializers that can save to database via QuerySet
///
/// This trait provides Django REST Framework-style `save()` and `create()` methods
/// that integrate with the ORM's QuerySet.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_serializers::{Serializer, SerializerSaveMixin};
///
/// #[derive(Serialize, Deserialize)]
/// struct User {
///     id: Option<i64>,
///     username: String,
///     email: String,
/// }
///
/// impl Model for User {
///     type PrimaryKey = i64;
///     fn table_name() -> &'static str { "users" }
///     // ... other implementations
/// }
///
/// struct UserSerializer;
///
/// impl Serializer for UserSerializer {
///     type Model = User;
///     // ... serializer implementation
/// }
///
/// impl SerializerSaveMixin for UserSerializer {}
///
/// // Usage
/// let data = json!({"username": "alice", "email": "alice@example.com"});
/// let user = UserSerializer::create(data).await?;
/// ```
#[async_trait]
pub trait SerializerSaveMixin
where
    Self: Sized,
{
    /// The model type this serializer works with
    type Model: Model + Serialize + DeserializeOwned + Clone + Send + Sync;

    /// Create a new instance in the database
    ///
    /// This method:
    /// 1. Validates the input data
    /// 2. Deserializes to model instance
    /// 3. Saves to database via QuerySet.create()
    /// 4. Returns the created instance
    ///
    /// # Errors
    ///
    /// Returns `SerializerError` if:
    /// - Validation fails
    /// - Deserialization fails
    /// - Database operation fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let data = json!({
    ///     "username": "alice",
    ///     "email": "alice@example.com"
    /// });
    ///
    /// let user = UserSerializer::create(data).await?;
    /// assert_eq!(user.username, "alice");
    /// ```
    async fn create(data: Value) -> Result<Self::Model, SerializerError> {
        // Pre-save validation
        Self::validate_for_create(&data).await?;

        // Deserialize to model
        let _model: Self::Model =
            serde_json::from_value(data).map_err(|e| SerializerError::Serde {
                message: format!("Failed to deserialize: {}", e),
            })?;

        // Save to database
        #[cfg(feature = "django-compat")]
        {
            let queryset = QuerySet::<Self::Model>::new();
            let created = queryset
                .create(model)
                .await
                .map_err(|e| SerializerError::Other {
                    message: format!("Failed to create: {}", e),
                })?;
            Ok(created)
        }

        #[cfg(not(feature = "django-compat"))]
        {
            // Without django-compat feature, we cannot use QuerySet.create()
            // Return error indicating feature is required
            Err(SerializerError::Other {
                message: "django-compat feature is required for save operations".to_string(),
            })
        }
    }

    /// Update an existing instance in the database
    ///
    /// This method:
    /// 1. Validates the input data
    /// 2. Merges with existing instance
    /// 3. Saves to database
    /// 4. Returns the updated instance
    ///
    /// # Errors
    ///
    /// Returns `SerializerError` if:
    /// - Validation fails
    /// - Instance not found
    /// - Database operation fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let data = json!({"email": "newemail@example.com"});
    /// let updated = UserSerializer::update(user, data).await?;
    /// assert_eq!(updated.email, "newemail@example.com");
    /// ```
    async fn update(
        mut instance: Self::Model,
        data: Value,
    ) -> Result<Self::Model, SerializerError> {
        // Pre-save validation with instance
        Self::validate_for_update(&data, Some(&instance)).await?;

        // Merge data into instance
        if let Value::Object(map) = data {
            let instance_value =
                serde_json::to_value(&instance).map_err(|e| SerializerError::Serde {
                    message: format!("Failed to serialize instance: {}", e),
                })?;

            if let Value::Object(mut instance_map) = instance_value {
                for (key, value) in map {
                    instance_map.insert(key, value);
                }

                instance = serde_json::from_value(Value::Object(instance_map)).map_err(|e| {
                    SerializerError::Serde {
                        message: format!("Failed to deserialize updated instance: {}", e),
                    }
                })?;
            }
        }

        // Integrate with Manager.update()
        #[cfg(feature = "django-compat")]
        {
            use reinhardt_orm::manager::Manager;

            let manager = Manager::<Self::Model>::new();
            let updated = manager
                .update(&instance)
                .await
                .map_err(|e| SerializerError::Other {
                    message: format!("Failed to update instance: {}", e),
                })?;

            Ok(updated)
        }

        #[cfg(not(feature = "django-compat"))]
        {
            // Without django-compat, return the merged instance
            // (in-memory update only, no database persistence)
            Ok(instance)
        }
    }

    /// Save the instance to database (create or update)
    ///
    /// This method automatically determines whether to create or update
    /// based on whether the instance has a primary key.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Create new instance
    /// let data = json!({"username": "alice", "email": "alice@example.com"});
    /// let user = UserSerializer::save(data, None).await?;
    ///
    /// // Update existing instance
    /// let update_data = json!({"email": "newemail@example.com"});
    /// let updated = UserSerializer::save(update_data, Some(user)).await?;
    /// ```
    async fn save(
        data: Value,
        instance: Option<Self::Model>,
    ) -> Result<Self::Model, SerializerError> {
        match instance {
            Some(inst) => Self::update(inst, data).await,
            None => Self::create(data).await,
        }
    }

    /// Pre-save validation hook for create operations
    ///
    /// Override this method to add custom validation logic that runs
    /// before creating a new instance.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// impl SerializerSaveMixin for UserSerializer {
    ///     async fn validate_for_create(data: &Value) -> Result<(), SerializerError> {
    ///         // Custom validation
    ///         if let Some(username) = data.get("username") {
    ///             if username.as_str().unwrap().len() < 3 {
    ///                 return Err(SerializerError::validation(
    ///                     ValidatorError::FieldValidation {
    ///                         field_name: "username".to_string(),
    ///                         value: username.to_string(),
    ///                         constraint: "min_length=3".to_string(),
    ///                         message: "Username too short".to_string(),
    ///                     }
    ///                 ));
    ///             }
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    async fn validate_for_create(_data: &Value) -> Result<(), SerializerError> {
        Ok(())
    }

    /// Pre-save validation hook for update operations
    ///
    /// Override this method to add custom validation logic that runs
    /// before updating an existing instance.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// impl SerializerSaveMixin for UserSerializer {
    ///     async fn validate_for_update(
    ///         data: &Value,
    ///         instance: Option<&User>,
    ///     ) -> Result<(), SerializerError> {
    ///         // Custom validation with access to existing instance
    ///         if let Some(user) = instance {
    ///             if user.is_admin && data.get("role").is_some() {
    ///                 return Err(SerializerError::validation(
    ///                     ValidatorError::Custom {
    ///                         message: "Cannot change admin role".to_string(),
    ///                     }
    ///                 ));
    ///             }
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    async fn validate_for_update(
        _data: &Value,
        _instance: Option<&Self::Model>,
    ) -> Result<(), SerializerError> {
        Ok(())
    }
}

/// Context for serializer save operations
///
/// Provides access to additional context needed during save operations.
#[derive(Debug, Clone, Default)]
pub struct SaveContext {
    /// Additional context data
    pub extra: HashMap<String, Value>,
}

impl SaveContext {
    /// Create a new save context
    pub fn new() -> Self {
        Self {
            extra: HashMap::new(),
        }
    }

    /// Add extra context data
    pub fn with_extra(mut self, key: String, value: Value) -> Self {
        self.extra.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reinhardt_orm::Model;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestUser {
        id: Option<i64>,
        username: String,
        email: String,
    }

    impl Model for TestUser {
        type PrimaryKey = i64;

        fn table_name() -> &'static str {
            "test_users"
        }

        fn primary_key(&self) -> Option<&Self::PrimaryKey> {
            self.id.as_ref()
        }

        fn set_primary_key(&mut self, value: Self::PrimaryKey) {
            self.id = Some(value);
        }
    }

    struct TestUserSerializer;

    impl SerializerSaveMixin for TestUserSerializer {
        type Model = TestUser;
    }

    #[test]
    fn test_save_context_creation() {
        let context = SaveContext::new();
        assert!(context.extra.is_empty());
    }

    #[test]
    fn test_save_context_with_extra() {
        let context = SaveContext::new()
            .with_extra("key1".to_string(), serde_json::json!("value1"))
            .with_extra("key2".to_string(), serde_json::json!(42));

        assert_eq!(context.extra.len(), 2);
        assert_eq!(
            context.extra.get("key1").unwrap(),
            &serde_json::json!("value1")
        );
        assert_eq!(context.extra.get("key2").unwrap(), &serde_json::json!(42));
    }

    #[tokio::test]
    async fn test_validate_for_create_default() {
        let data = serde_json::json!({
            "username": "testuser",
            "email": "test@example.com"
        });

        let result = TestUserSerializer::validate_for_create(&data).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_for_update_default() {
        let data = serde_json::json!({"email": "newemail@example.com"});
        let instance = TestUser {
            id: Some(1),
            username: "testuser".to_string(),
            email: "old@example.com".to_string(),
        };

        let result = TestUserSerializer::validate_for_update(&data, Some(&instance)).await;
        assert!(result.is_ok());
    }

    // Note: Integration tests with actual database would go in tests/ directory
}

/// Cache-aware save context
///
/// Extends SaveContext with automatic cache invalidation support.
///
/// # Examples
///
/// ```
/// use reinhardt_serializers::queryset_integration::CacheAwareSaveContext;
/// use reinhardt_serializers::{CacheInvalidator, InvalidationStrategy};
///
/// let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);
/// let context = CacheAwareSaveContext::with_invalidator(invalidator);
///
/// // After save, cache will be automatically invalidated
/// ```
#[derive(Debug, Clone)]
pub struct CacheAwareSaveContext {
    /// Base save context
    pub context: SaveContext,
    /// Cache invalidator (optional)
    pub invalidator: Option<crate::CacheInvalidator>,
}

impl CacheAwareSaveContext {
    /// Create a new cache-aware context without invalidator
    pub fn new() -> Self {
        Self {
            context: SaveContext::new(),
            invalidator: None,
        }
    }

    /// Create a cache-aware context with invalidator
    pub fn with_invalidator(invalidator: crate::CacheInvalidator) -> Self {
        Self {
            context: SaveContext::new(),
            invalidator: Some(invalidator),
        }
    }

    /// Invalidate cache for a model instance
    ///
    /// Call this after successful save/update operations.
    pub fn invalidate_cache(&self, model_name: &str, pk: &str) -> Vec<String> {
        if let Some(ref invalidator) = self.invalidator {
            invalidator.invalidate(model_name, pk)
        } else {
            Vec::new()
        }
    }

    /// Add cache dependency before save
    ///
    /// Register that a cache key depends on this model instance.
    pub fn add_cache_dependency(&self, cache_key: &str, model_name: &str, pk: &str) {
        if let Some(ref invalidator) = self.invalidator {
            invalidator.add_dependency(cache_key, model_name, pk);
        }
    }

    /// Get the underlying SaveContext
    pub fn inner(&self) -> &SaveContext {
        &self.context
    }

    /// Convert to SaveContext
    pub fn into_inner(self) -> SaveContext {
        self.context
    }
}

impl Default for CacheAwareSaveContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod cache_aware_tests {
    use super::*;
    use crate::InvalidationStrategy;

    #[test]
    fn test_cache_aware_save_context_without_invalidator() {
        let context = CacheAwareSaveContext::new();
        assert!(context.invalidator.is_none());

        // Should not panic, just return empty vec
        let keys = context.invalidate_cache("User", "123");
        assert_eq!(keys.len(), 0);
    }

    #[test]
    fn test_cache_aware_save_context_with_invalidator() {
        let invalidator = crate::CacheInvalidator::new(InvalidationStrategy::Immediate);
        let context = CacheAwareSaveContext::with_invalidator(invalidator);

        assert!(context.invalidator.is_some());

        // Add dependency
        context.add_cache_dependency("user:123:profile", "User", "123");

        // Invalidate
        let keys = context.invalidate_cache("User", "123");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "user:123:profile");
    }

    #[test]
    fn test_cache_aware_save_context_inner() {
        let context = CacheAwareSaveContext::new();
        let inner = context.inner();
        assert!(inner.extra.is_empty());
    }

    #[test]
    fn test_cache_aware_save_context_into_inner() {
        let context = CacheAwareSaveContext::new();
        let inner = context.into_inner();
        assert!(inner.extra.is_empty());
    }
}
