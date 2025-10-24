//! Validators for ModelSerializer
//!
//! This module provides validators for enforcing database constraints
//! such as uniqueness of fields.
//!
//! # Examples
//!
//! ```no_run
//! use reinhardt_serializers::validators::{UniqueValidator, UniqueTogetherValidator};
//! use reinhardt_orm::Model;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct User {
//!     id: Option<i64>,
//!     username: String,
//!     email: String,
//! }
//!
//! impl Model for User {
//!     type PrimaryKey = i64;
//!     fn table_name() -> &'static str { "users" }
//!     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
//!     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = sqlx::PgPool::connect("postgres://localhost/test").await?;
//!
//! // Validate that username is unique
//! let validator = UniqueValidator::<User>::new("username");
//! validator.validate(&pool, "alice", None).await?;
//!
//! // Validate that (username, email) combination is unique
//! let mut values = std::collections::HashMap::new();
//! values.insert("username".to_string(), "alice".to_string());
//! values.insert("email".to_string(), "alice@example.com".to_string());
//!
//! let validator = UniqueTogetherValidator::<User>::new(vec!["username", "email"]);
//! validator.validate(&pool, &values, None).await?;
//! # Ok(())
//! # }
//! ```

use crate::SerializerError;
use reinhardt_orm::Model;
use sqlx::{Pool, Postgres, Row};
use std::marker::PhantomData;
use thiserror::Error;

/// Errors that can occur during database validation
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DatabaseValidatorError {
    /// A unique constraint was violated for a single field
    #[error("Unique constraint violated: {field} = '{value}' already exists in table {table}")]
    UniqueConstraintViolation {
        /// The field name that violated the constraint
        field: String,
        /// The value that caused the violation
        value: String,
        /// The table name
        table: String,
        /// Optional custom message
        message: Option<String>,
    },

    /// A unique together constraint was violated for multiple fields
    #[error("Unique together constraint violated: fields ({fields:?}) with values ({values:?}) already exist in table {table}")]
    UniqueTogetherViolation {
        /// The field names that violated the constraint
        fields: Vec<String>,
        /// The values that caused the violation
        values: Vec<String>,
        /// The table name
        table: String,
        /// Optional custom message
        message: Option<String>,
    },

    /// A database error occurred during validation
    #[error("Database error during validation: {message}")]
    DatabaseError {
        /// The error message from the database
        message: String,
        /// The SQL query that failed (optional, for debugging)
        query: Option<String>,
    },

    /// A required field was not found in the data
    #[error("Required field '{field}' not found in validation data")]
    FieldNotFound {
        /// The field name that was missing
        field: String,
    },
}

impl From<DatabaseValidatorError> for SerializerError {
    fn from(err: DatabaseValidatorError) -> Self {
        SerializerError::new(err.to_string())
    }
}

/// UniqueValidator ensures that a field value is unique in the database
///
/// This validator checks that a given field value doesn't already exist
/// in the database table, with optional support for excluding the current
/// instance during updates.
///
/// # Examples
///
/// ```no_run
/// # use reinhardt_serializers::validators::UniqueValidator;
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
/// #
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = sqlx::PgPool::connect("postgres://localhost/test").await?;
/// let validator = UniqueValidator::<User>::new("username");
///
/// // Check if "alice" is unique
/// validator.validate(&pool, "alice", None).await?;
///
/// // Check if "alice" is unique, excluding user with id=1
/// let user_id = 1i64;
/// validator.validate(&pool, "alice", Some(&user_id)).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct UniqueValidator<M: Model> {
    field_name: String,
    message: Option<String>,
    _phantom: PhantomData<M>,
}

impl<M: Model> UniqueValidator<M> {
    /// Create a new UniqueValidator for the specified field
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::validators::UniqueValidator;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct User { id: Option<i64>, username: String }
    /// #
    /// # impl Model for User {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "users" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let validator = UniqueValidator::<User>::new("username");
    /// ```
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            message: None,
            _phantom: PhantomData,
        }
    }

    /// Set a custom error message
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::validators::UniqueValidator;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct User { id: Option<i64>, username: String }
    /// #
    /// # impl Model for User {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "users" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let validator = UniqueValidator::<User>::new("username")
    ///     .with_message("Username must be unique");
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Get the field name being validated
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    pub async fn validate(
        &self,
        pool: &Pool<Postgres>,
        value: &str,
        instance_pk: Option<&M::PrimaryKey>,
    ) -> Result<(), DatabaseValidatorError>
    where
        M::PrimaryKey: std::fmt::Display,
    {
        let table_name = M::table_name();
        let pk_field = M::primary_key_field();

        let query = if let Some(_pk) = instance_pk {
            format!(
                "SELECT COUNT(*) as count FROM {} WHERE {} = $1 AND {} != $2",
                table_name, self.field_name, pk_field
            )
        } else {
            format!(
                "SELECT COUNT(*) as count FROM {} WHERE {} = $1",
                table_name, self.field_name
            )
        };

        let count: i64 = if let Some(pk) = instance_pk {
            let pk_str = pk.to_string();
            sqlx::query(&query)
                .bind(value)
                .bind(pk_str)
                .fetch_one(pool)
                .await
                .map_err(|e| DatabaseValidatorError::DatabaseError {
                    message: e.to_string(),
                    query: Some(query.clone()),
                })?
                .get("count")
        } else {
            sqlx::query(&query)
                .bind(value)
                .fetch_one(pool)
                .await
                .map_err(|e| DatabaseValidatorError::DatabaseError {
                    message: e.to_string(),
                    query: Some(query.clone()),
                })?
                .get("count")
        };

        if count > 0 {
            Err(DatabaseValidatorError::UniqueConstraintViolation {
                field: self.field_name.clone(),
                value: value.to_string(),
                table: table_name.to_string(),
                message: self.message.clone(),
            })
        } else {
            Ok(())
        }
    }
}

/// UniqueTogetherValidator ensures that a combination of fields is unique
///
/// This validator checks that a combination of field values doesn't already exist
/// in the database table, with optional support for excluding the current
/// instance during updates.
///
/// # Examples
///
/// ```no_run
/// # use reinhardt_serializers::validators::UniqueTogetherValidator;
/// # use reinhardt_orm::Model;
/// # use serde::{Serialize, Deserialize};
/// # use std::collections::HashMap;
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
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = sqlx::PgPool::connect("postgres://localhost/test").await?;
/// let validator = UniqueTogetherValidator::<User>::new(vec!["username", "email"]);
///
/// let mut values = HashMap::new();
/// values.insert("username".to_string(), "alice".to_string());
/// values.insert("email".to_string(), "alice@example.com".to_string());
///
/// validator.validate(&pool, &values, None).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct UniqueTogetherValidator<M: Model> {
    field_names: Vec<String>,
    message: Option<String>,
    _phantom: PhantomData<M>,
}

impl<M: Model> UniqueTogetherValidator<M> {
    /// Create a new UniqueTogetherValidator for the specified fields
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::validators::UniqueTogetherValidator;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct User { id: Option<i64>, username: String, email: String }
    /// #
    /// # impl Model for User {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "users" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let validator = UniqueTogetherValidator::<User>::new(vec!["username", "email"]);
    /// ```
    pub fn new(field_names: Vec<impl Into<String>>) -> Self {
        Self {
            field_names: field_names.into_iter().map(|f| f.into()).collect(),
            message: None,
            _phantom: PhantomData,
        }
    }

    /// Set a custom error message
    ///
    /// # Examples
    ///
    /// ```
    /// # use reinhardt_serializers::validators::UniqueTogetherValidator;
    /// # use reinhardt_orm::Model;
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct User { id: Option<i64>, username: String, email: String }
    /// #
    /// # impl Model for User {
    /// #     type PrimaryKey = i64;
    /// #     fn table_name() -> &'static str { "users" }
    /// #     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
    /// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
    /// # }
    /// let validator = UniqueTogetherValidator::<User>::new(vec!["username", "email"])
    ///     .with_message("Username and email combination must be unique");
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Get the field names being validated
    pub fn field_names(&self) -> &[String] {
        &self.field_names
    }

    pub async fn validate(
        &self,
        pool: &Pool<Postgres>,
        values: &std::collections::HashMap<String, String>,
        instance_pk: Option<&M::PrimaryKey>,
    ) -> Result<(), DatabaseValidatorError>
    where
        M::PrimaryKey: std::fmt::Display,
    {
        let table_name = M::table_name();
        let pk_field = M::primary_key_field();

        let mut where_clauses = Vec::new();
        for (i, field_name) in self.field_names.iter().enumerate() {
            where_clauses.push(format!("{} = ${}", field_name, i + 1));
        }
        let where_clause = where_clauses.join(" AND ");

        let query = if let Some(_pk) = instance_pk {
            format!(
                "SELECT COUNT(*) as count FROM {} WHERE {} AND {} != ${}",
                table_name,
                where_clause,
                pk_field,
                self.field_names.len() + 1
            )
        } else {
            format!(
                "SELECT COUNT(*) as count FROM {} WHERE {}",
                table_name, where_clause
            )
        };

        let mut query_builder = sqlx::query(&query);
        let mut field_values = Vec::new();
        for field_name in &self.field_names {
            let value =
                values
                    .get(field_name)
                    .ok_or_else(|| DatabaseValidatorError::FieldNotFound {
                        field: field_name.clone(),
                    })?;
            field_values.push(value.clone());
            query_builder = query_builder.bind(value);
        }

        let count: i64 = if let Some(pk) = instance_pk {
            let pk_str = pk.to_string();
            query_builder = query_builder.bind(pk_str);
            query_builder
                .fetch_one(pool)
                .await
                .map_err(|e| DatabaseValidatorError::DatabaseError {
                    message: e.to_string(),
                    query: Some(query.clone()),
                })?
                .get("count")
        } else {
            query_builder
                .fetch_one(pool)
                .await
                .map_err(|e| DatabaseValidatorError::DatabaseError {
                    message: e.to_string(),
                    query: Some(query.clone()),
                })?
                .get("count")
        };

        if count > 0 {
            Err(DatabaseValidatorError::UniqueTogetherViolation {
                fields: self.field_names.clone(),
                values: field_values,
                table: table_name.to_string(),
                message: self.message.clone(),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    #[test]
    fn test_unique_validator_new() {
        let validator = UniqueValidator::<TestUser>::new("username");
        assert_eq!(validator.field_name(), "username");
    }

    #[test]
    fn test_unique_validator_with_message() {
        let validator =
            UniqueValidator::<TestUser>::new("username").with_message("Custom error message");
        assert_eq!(validator.field_name(), "username");
        assert!(validator.message.is_some());
    }

    #[test]
    fn test_unique_together_validator_new() {
        let validator = UniqueTogetherValidator::<TestUser>::new(vec!["username", "email"]);
        assert_eq!(validator.field_names().len(), 2);
        assert_eq!(validator.field_names()[0], "username");
        assert_eq!(validator.field_names()[1], "email");
    }

    #[test]
    fn test_unique_together_validator_with_message() {
        let validator = UniqueTogetherValidator::<TestUser>::new(vec!["username", "email"])
            .with_message("Custom combination message");
        assert_eq!(validator.field_names().len(), 2);
        assert!(validator.message.is_some());
    }
}
