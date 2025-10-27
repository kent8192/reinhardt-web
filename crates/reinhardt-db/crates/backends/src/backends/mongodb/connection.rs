//! MongoDB connection and backend implementation
//!
//! This module provides the MongoDB database backend that implements
//! the `DatabaseBackend` trait.
//!
//! # Example
//!
//! ```rust,no_run
//! use reinhardt_database::backends::mongodb::MongoDBBackend;
//! use reinhardt_database::backend::DatabaseBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to MongoDB
//! let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?;
//!
//! // Use a specific database
//! let backend_with_db = backend.with_database("myapp");
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use bson::{doc, Bson};
use mongodb::{Client, Database};
use std::sync::Arc;

use crate::{
    backend::DatabaseBackend,
    error::Result,
    types::{DatabaseType, QueryResult, QueryValue, Row},
};

/// MongoDB backend implementation
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_database::backends::mongodb::MongoDBBackend;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?;
/// let backend = backend.with_database("mydb");
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MongoDBBackend {
    client: Arc<Client>,
    database_name: String,
}

impl MongoDBBackend {
    /// Connect to MongoDB using a connection string
    ///
    /// # Arguments
    ///
    /// * `url` - MongoDB connection string (e.g., "mongodb://localhost:27017")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use reinhardt_database::backends::mongodb::MongoDBBackend;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(url: &str) -> Result<Self> {
        let client = Client::with_uri_str(url)
            .await
            .map_err(|e| crate::error::DatabaseError::ConnectionError(e.to_string()))?;

        Ok(Self {
            client: Arc::new(client),
            database_name: "test".to_string(), // Default database
        })
    }

    /// Set the database name to use
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use reinhardt_database::backends::mongodb::MongoDBBackend;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?;
    /// let backend = backend.with_database("myapp");
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_database(mut self, database_name: &str) -> Self {
        self.database_name = database_name.to_string();
        self
    }

    /// Get the MongoDB database instance
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use reinhardt_database::backends::mongodb::MongoDBBackend;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?;
    /// let db = backend.database();
    /// # Ok(())
    /// # }
    /// ```
    pub fn database(&self) -> Database {
        self.client.database(&self.database_name)
    }

    /// Convert QueryValue to BSON
    #[allow(dead_code)]
    fn query_value_to_bson(value: &QueryValue) -> Bson {
        match value {
            QueryValue::Null => Bson::Null,
            QueryValue::Bool(b) => Bson::Boolean(*b),
            QueryValue::Int(i) => Bson::Int64(*i),
            QueryValue::Float(f) => Bson::Double(*f),
            QueryValue::String(s) => Bson::String(s.clone()),
            QueryValue::Bytes(b) => Bson::Binary(bson::Binary {
                subtype: bson::spec::BinarySubtype::Generic,
                bytes: b.clone(),
            }),
            QueryValue::Timestamp(dt) => {
                // Convert chrono DateTime to BSON DateTime
                Bson::DateTime(bson::DateTime::from_millis(dt.timestamp_millis()))
            }
        }
    }

    /// Convert BSON to QueryValue
    #[allow(dead_code)]
    fn bson_to_query_value(bson: Bson) -> QueryValue {
        match bson {
            Bson::Null => QueryValue::Null,
            Bson::Boolean(b) => QueryValue::Bool(b),
            Bson::Int32(i) => QueryValue::Int(i as i64),
            Bson::Int64(i) => QueryValue::Int(i),
            Bson::Double(f) => QueryValue::Float(f),
            Bson::String(s) => QueryValue::String(s),
            Bson::Binary(b) => QueryValue::Bytes(b.bytes),
            Bson::DateTime(dt) => {
                // Convert BSON DateTime to chrono DateTime
                use chrono::TimeZone;
                QueryValue::Timestamp(chrono::Utc.timestamp_millis_opt(dt.timestamp_millis()).unwrap())
            }
            _ => QueryValue::Null, // For unsupported types
        }
    }
}

#[async_trait]
impl DatabaseBackend for MongoDBBackend {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::MongoDB
    }

    fn placeholder(&self, _index: usize) -> String {
        // MongoDB doesn't use SQL-style placeholders
        // Instead, it uses BSON documents
        String::from("$")
    }

    fn supports_returning(&self) -> bool {
        // MongoDB doesn't support SQL RETURNING clause
        // It returns the inserted document by default
        true
    }

    fn supports_on_conflict(&self) -> bool {
        // MongoDB supports upsert operations
        true
    }

    async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
        // MongoDB doesn't execute SQL
        // This would parse the "sql" as a collection name and operation
        // For now, we'll return an error indicating this needs to be implemented
        // via the query builder
        Err(crate::error::DatabaseError::QueryError(
            "MongoDB requires using the query builder instead of raw SQL".to_string(),
        ))
    }

    async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
        // Similar to execute, this would need to be implemented via query builder
        Err(crate::error::DatabaseError::QueryError(
            "MongoDB requires using the query builder instead of raw SQL".to_string(),
        ))
    }

    async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
        // Similar to execute, this would need to be implemented via query builder
        Err(crate::error::DatabaseError::QueryError(
            "MongoDB requires using the query builder instead of raw SQL".to_string(),
        ))
    }

    async fn fetch_optional(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Option<Row>> {
        // Similar to execute, this would need to be implemented via query builder
        Err(crate::error::DatabaseError::QueryError(
            "MongoDB requires using the query builder instead of raw SQL".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_value_to_bson() {
        let null_value = QueryValue::Null;
        assert_eq!(MongoDBBackend::query_value_to_bson(&null_value), Bson::Null);

        let bool_value = QueryValue::Bool(true);
        assert_eq!(
            MongoDBBackend::query_value_to_bson(&bool_value),
            Bson::Boolean(true)
        );

        let int_value = QueryValue::Int(42);
        assert_eq!(
            MongoDBBackend::query_value_to_bson(&int_value),
            Bson::Int64(42)
        );

        let float_value = QueryValue::Float(3.14);
        assert_eq!(
            MongoDBBackend::query_value_to_bson(&float_value),
            Bson::Double(3.14)
        );

        let string_value = QueryValue::String("hello".to_string());
        assert_eq!(
            MongoDBBackend::query_value_to_bson(&string_value),
            Bson::String("hello".to_string())
        );
    }

    #[test]
    fn test_bson_to_query_value() {
        let null_bson = Bson::Null;
        match MongoDBBackend::bson_to_query_value(null_bson) {
            QueryValue::Null => (),
            _ => panic!("Expected Null"),
        }

        let bool_bson = Bson::Boolean(true);
        match MongoDBBackend::bson_to_query_value(bool_bson) {
            QueryValue::Bool(true) => (),
            _ => panic!("Expected Bool(true)"),
        }

        let int_bson = Bson::Int64(42);
        match MongoDBBackend::bson_to_query_value(int_bson) {
            QueryValue::Int(42) => (),
            _ => panic!("Expected Int(42)"),
        }
    }
}
