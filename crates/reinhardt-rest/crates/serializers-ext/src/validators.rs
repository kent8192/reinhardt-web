//! Validators for ModelSerializer
//!
//! This module provides validators for enforcing database constraints
//! such as uniqueness of fields.

use crate::SerializerError;
use reinhardt_orm::Model;
use sqlx::{Pool, Postgres, Row};
use std::marker::PhantomData;

/// UniqueValidator ensures that a field value is unique in the database
pub struct UniqueValidator<M: Model> {
    field_name: String,
    _phantom: PhantomData<M>,
}

impl<M: Model> UniqueValidator<M> {
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            _phantom: PhantomData,
        }
    }

    pub async fn validate(
        &self,
        pool: &Pool<Postgres>,
        value: &str,
        instance_pk: Option<&M::PrimaryKey>,
    ) -> Result<(), SerializerError>
    where
        M::PrimaryKey: std::fmt::Display,
    {
        let table_name = M::table_name();
        let pk_field = M::primary_key_field();

        let query = if let Some(pk) = instance_pk {
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
                .map_err(|e| SerializerError::new(format!("Database error: {}", e)))?
                .get("count")
        } else {
            sqlx::query(&query)
                .bind(value)
                .fetch_one(pool)
                .await
                .map_err(|e| SerializerError::new(format!("Database error: {}", e)))?
                .get("count")
        };

        if count > 0 {
            Err(SerializerError::new(format!(
                "{} with this {} already exists",
                table_name, self.field_name
            )))
        } else {
            Ok(())
        }
    }
}

/// UniqueTogetherValidator ensures that a combination of fields is unique
pub struct UniqueTogetherValidator<M: Model> {
    field_names: Vec<String>,
    _phantom: PhantomData<M>,
}

impl<M: Model> UniqueTogetherValidator<M> {
    pub fn new(field_names: Vec<impl Into<String>>) -> Self {
        Self {
            field_names: field_names.into_iter().map(|f| f.into()).collect(),
            _phantom: PhantomData,
        }
    }

    pub async fn validate(
        &self,
        pool: &Pool<Postgres>,
        values: &std::collections::HashMap<String, String>,
        instance_pk: Option<&M::PrimaryKey>,
    ) -> Result<(), SerializerError>
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

        let query = if let Some(pk) = instance_pk {
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
        for field_name in &self.field_names {
            let value = values.get(field_name).ok_or_else(|| {
                SerializerError::new(format!("Missing value for field: {}", field_name))
            })?;
            query_builder = query_builder.bind(value);
        }

        let count: i64 = if let Some(pk) = instance_pk {
            let pk_str = pk.to_string();
            query_builder = query_builder.bind(pk_str);
            query_builder
                .fetch_one(pool)
                .await
                .map_err(|e| SerializerError::new(format!("Database error: {}", e)))?
                .get("count")
        } else {
            query_builder
                .fetch_one(pool)
                .await
                .map_err(|e| SerializerError::new(format!("Database error: {}", e)))?
                .get("count")
        };

        if count > 0 {
            Err(SerializerError::new(format!(
                "The fields {} must make a unique set",
                self.field_names.join(", ")
            )))
        } else {
            Ok(())
        }
    }
}
