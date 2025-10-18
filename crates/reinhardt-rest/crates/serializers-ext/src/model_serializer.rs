//! ModelSerializer - Django REST Framework inspired model serialization
//!
//! This module provides ModelSerializer that automatically generates
//! serialization logic from ORM models.

use crate::{Serializer, SerializerError};
use reinhardt_orm::Model;
use std::marker::PhantomData;

/// ModelSerializer provides automatic serialization for ORM models
///
/// Inspired by Django REST Framework's ModelSerializer, this automatically
/// handles serialization, deserialization, validation, and database operations
/// for models that implement the Model trait.
///
/// # Examples
///
/// ```no_run
/// # use reinhardt_serializers::ModelSerializer;
/// # use reinhardt_orm::{Model, Engine};
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct User {
/// #     id: Option<i64>,
/// #     username: String,
/// #     email: String,
/// # }
/// #
/// # impl Model for User {
/// #     type PrimaryKey = i64;
/// #     fn table_name() -> &'static str { "users" }
/// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
/// # }
/// #
/// # fn example() {
/// let serializer = ModelSerializer::<User>::new();
///
/// // Serialize a user
/// let user = User {
///     id: Some(1),
///     username: "alice".to_string(),
///     email: "alice@example.com".to_string(),
/// };
///
/// // Validate and serialize
/// assert!(serializer.validate(&user).is_ok());
/// let json = serializer.serialize(&user).unwrap();
/// # }
/// ```
pub struct ModelSerializer<M>
where
    M: Model,
{
    _phantom: PhantomData<M>,
}

impl<M> ModelSerializer<M>
where
    M: Model,
{
    /// Create a new ModelSerializer instance
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::ModelSerializer;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct User {
    /// #     id: Option<i64>,
    /// #     username: String,
    /// # }
    /// #
    /// # impl Model for User {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "users" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let serializer = ModelSerializer::<User>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Validate a model instance
    ///
    /// This method can be extended to support custom validators.
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::ModelSerializer;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct User {
    /// #     id: Option<i64>,
    /// #     username: String,
    /// # }
    /// #
    /// # impl Model for User {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "users" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let serializer = ModelSerializer::<User>::new();
    /// let user = User { id: None, username: "alice".to_string() };
    /// assert!(serializer.validate(&user).is_ok());
    /// ```
    pub fn validate(&self, _instance: &M) -> Result<(), SerializerError> {
        // Base validation - can be extended with validators
        // For now, just return Ok
        Ok(())
    }
}

impl<M> Default for ModelSerializer<M>
where
    M: Model,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Serializer for ModelSerializer<M>
where
    M: Model,
{
    type Input = M;
    type Output = String;

    fn serialize(&self, input: &Self::Input) -> Result<Self::Output, SerializerError> {
        serde_json::to_string(input)
            .map_err(|e| SerializerError::new(format!("Serialization error: {}", e)))
    }

    fn deserialize(&self, output: &Self::Output) -> Result<Self::Input, SerializerError> {
        serde_json::from_str(output)
            .map_err(|e| SerializerError::new(format!("Deserialization error: {}", e)))
    }
}
