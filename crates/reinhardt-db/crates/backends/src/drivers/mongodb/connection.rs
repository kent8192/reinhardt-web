//! MongoDB connection and backend implementation
//!
//! This module provides the MongoDB database backend that implements
//! the `DatabaseBackend` trait.
//!
//! # Example
//!
//! ```rust,no_run
//! use reinhardt_db::backends::mongodb::MongoDBBackend;
//! use reinhardt_db::backends::backend::DatabaseBackend;
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
use bson::{Bson, Document};
use mongodb::{Client, ClientSession, Database};
use std::sync::Arc;

use crate::{
	backend::DatabaseBackend,
	error::Result,
	types::{DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor},
};

/// MongoDB backend implementation
///
/// Supports connection pooling, replica sets, and sharded clusters.
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_db::backends::mongodb::MongoDBBackend;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Basic connection
/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?;
/// let backend = backend.with_database("mydb");
///
/// // Connection with options
/// let backend = MongoDBBackend::builder()
///     .url("mongodb://localhost:27017")
///     .database("mydb")
///     .max_pool_size(100)
///     .min_pool_size(10)
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MongoDBBackend {
	client: Arc<Client>,
	database_name: String,
}

/// MongoDB transaction executor
///
/// This struct wraps a MongoDB `ClientSession` to ensure all operations
/// within a transaction run within the same session context.
///
/// # Note
///
/// MongoDB transactions require a replica set or sharded cluster.
/// Standalone MongoDB instances do not support transactions.
pub struct MongoDBTransactionExecutor {
	/// The MongoDB session (Option for consume-on-commit/rollback pattern)
	session: Option<ClientSession>,
	/// Reference to the client for accessing database/collections
	#[allow(dead_code)]
	client: Arc<Client>,
	/// Database name for operations
	#[allow(dead_code)]
	database_name: String,
}

impl MongoDBTransactionExecutor {
	/// Create a new MongoDB transaction executor
	pub fn new(session: ClientSession, client: Arc<Client>, database_name: String) -> Self {
		Self {
			session: Some(session),
			client,
			database_name,
		}
	}
}

#[async_trait]
impl TransactionExecutor for MongoDBTransactionExecutor {
	async fn execute(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
		Err(crate::error::DatabaseError::UnsupportedFeature {
			database: "MongoDB".to_string(),
			feature: "SQL queries in transactions".to_string(),
		})
	}

	async fn fetch_one(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
		Err(crate::error::DatabaseError::UnsupportedFeature {
			database: "MongoDB".to_string(),
			feature: "SQL queries in transactions".to_string(),
		})
	}

	async fn fetch_all(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
		Err(crate::error::DatabaseError::UnsupportedFeature {
			database: "MongoDB".to_string(),
			feature: "SQL queries in transactions".to_string(),
		})
	}

	async fn fetch_optional(
		&mut self,
		_sql: &str,
		_params: Vec<QueryValue>,
	) -> Result<Option<Row>> {
		Err(crate::error::DatabaseError::UnsupportedFeature {
			database: "MongoDB".to_string(),
			feature: "SQL queries in transactions".to_string(),
		})
	}

	async fn commit(mut self: Box<Self>) -> Result<()> {
		let mut session = self.session.take().ok_or_else(|| {
			crate::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		session.commit_transaction().await.map_err(|e| {
			crate::error::DatabaseError::TransactionError(format!(
				"Failed to commit MongoDB transaction: {}",
				e
			))
		})
	}

	async fn rollback(mut self: Box<Self>) -> Result<()> {
		let mut session = self.session.take().ok_or_else(|| {
			crate::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		session.abort_transaction().await.map_err(|e| {
			crate::error::DatabaseError::TransactionError(format!(
				"Failed to rollback MongoDB transaction: {}",
				e
			))
		})
	}
}

/// Builder for configuring MongoDB connections
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = MongoDBBackendBuilder::new()
///     .url("mongodb://localhost:27017")
///     .database("mydb")
///     .max_pool_size(100)
///     .min_pool_size(10)
///     .max_idle_time_secs(300)
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct MongoDBBackendBuilder {
	url: String,
	database: String,
	max_pool_size: Option<u32>,
	min_pool_size: Option<u32>,
	max_idle_time_secs: Option<u64>,
}

impl Default for MongoDBBackendBuilder {
	fn default() -> Self {
		Self::new()
	}
}

impl MongoDBBackendBuilder {
	/// Create a new builder with default settings
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
	///
	/// let builder = MongoDBBackendBuilder::new();
	/// ```
	pub fn new() -> Self {
		Self {
			url: "mongodb://localhost:27017".to_string(),
			database: "test".to_string(),
			max_pool_size: None,
			min_pool_size: None,
			max_idle_time_secs: None,
		}
	}

	/// Set the MongoDB connection URL
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
	///
	/// let builder = MongoDBBackendBuilder::new()
	///     .url("mongodb://localhost:27017");
	/// ```
	pub fn url(mut self, url: impl Into<String>) -> Self {
		self.url = url.into();
		self
	}

	/// Set the database name
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
	///
	/// let builder = MongoDBBackendBuilder::new()
	///     .database("mydb");
	/// ```
	pub fn database(mut self, database: impl Into<String>) -> Self {
		self.database = database.into();
		self
	}

	/// Set the maximum connection pool size
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
	///
	/// let builder = MongoDBBackendBuilder::new()
	///     .max_pool_size(100);
	/// ```
	pub fn max_pool_size(mut self, size: u32) -> Self {
		self.max_pool_size = Some(size);
		self
	}

	/// Set the minimum connection pool size
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
	///
	/// let builder = MongoDBBackendBuilder::new()
	///     .min_pool_size(10);
	/// ```
	pub fn min_pool_size(mut self, size: u32) -> Self {
		self.min_pool_size = Some(size);
		self
	}

	/// Set the maximum idle time for connections in seconds
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
	///
	/// let builder = MongoDBBackendBuilder::new()
	///     .max_idle_time_secs(300);
	/// ```
	pub fn max_idle_time_secs(mut self, secs: u64) -> Self {
		self.max_idle_time_secs = Some(secs);
		self
	}

	/// Build the MongoDB backend
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackendBuilder;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackendBuilder::new()
	///     .url("mongodb://localhost:27017")
	///     .database("mydb")
	///     .build()
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn build(self) -> Result<MongoDBBackend> {
		use mongodb::options::ClientOptions;
		use std::time::Duration;

		let mut options = ClientOptions::parse(&self.url)
			.await
			.map_err(|e| crate::error::DatabaseError::ConnectionError(e.to_string()))?;

		// Configure connection pool
		if let Some(max_size) = self.max_pool_size {
			options.max_pool_size = Some(max_size);
		}

		if let Some(min_size) = self.min_pool_size {
			options.min_pool_size = Some(min_size);
		}

		if let Some(idle_time) = self.max_idle_time_secs {
			options.max_idle_time = Some(Duration::from_secs(idle_time));
		}

		let client = Client::with_options(options)
			.map_err(|e| crate::error::DatabaseError::ConnectionError(e.to_string()))?;

		Ok(MongoDBBackend {
			client: Arc::new(client),
			database_name: self.database,
		})
	}
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
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
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

	/// Create a builder for configuring the MongoDB connection
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::builder()
	///     .url("mongodb://localhost:27017")
	///     .database("mydb")
	///     .max_pool_size(100)
	///     .build()
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn builder() -> MongoDBBackendBuilder {
		MongoDBBackendBuilder::new()
	}

	/// Set the database name to use
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
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
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
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

	/// Insert a document into a collection
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let doc = doc! { "name": "Alice", "age": 30 };
	/// let id = backend.insert_one("users", doc).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert_one(&self, collection: &str, document: Document) -> Result<Bson> {
		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let result = coll
			.insert_one(document)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		Ok(result.inserted_id)
	}

	/// Insert multiple documents into a collection
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let docs = vec![
	///     doc! { "name": "Alice", "age": 30 },
	///     doc! { "name": "Bob", "age": 25 },
	/// ];
	/// let ids = backend.insert_many("users", docs).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert_many(
		&self,
		collection: &str,
		documents: Vec<Document>,
	) -> Result<Vec<Bson>> {
		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let result = coll
			.insert_many(documents)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		Ok(result.inserted_ids.into_values().collect())
	}

	/// Find one document matching the filter
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let filter = doc! { "name": "Alice" };
	/// if let Some(user) = backend.find_one("users", filter).await? {
	///     println!("Found user: {:?}", user);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn find_one(&self, collection: &str, filter: Document) -> Result<Option<Document>> {
		let db = self.database();
		let coll = db.collection::<Document>(collection);

		coll.find_one(filter)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))
	}

	/// Find all documents matching the filter
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let filter = doc! { "age": { "$gte": 18 } };
	/// let users = backend.find("users", filter).await?;
	/// for user in users {
	///     println!("User: {:?}", user);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn find(&self, collection: &str, filter: Document) -> Result<Vec<Document>> {
		use futures::stream::TryStreamExt;

		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let cursor = coll
			.find(filter)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		cursor
			.try_collect()
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))
	}

	/// Update one document matching the filter
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let filter = doc! { "name": "Alice" };
	/// let update = doc! { "$set": { "age": 31 } };
	/// let count = backend.update_one("users", filter, update).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn update_one(
		&self,
		collection: &str,
		filter: Document,
		update: Document,
	) -> Result<u64> {
		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let result = coll
			.update_one(filter, update)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		Ok(result.modified_count)
	}

	/// Update all documents matching the filter
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let filter = doc! { "age": { "$lt": 18 } };
	/// let update = doc! { "$set": { "status": "minor" } };
	/// let count = backend.update_many("users", filter, update).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn update_many(
		&self,
		collection: &str,
		filter: Document,
		update: Document,
	) -> Result<u64> {
		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let result = coll
			.update_many(filter, update)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		Ok(result.modified_count)
	}

	/// Delete one document matching the filter
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let filter = doc! { "name": "Alice" };
	/// let count = backend.delete_one("users", filter).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn delete_one(&self, collection: &str, filter: Document) -> Result<u64> {
		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let result = coll
			.delete_one(filter)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		Ok(result.deleted_count)
	}

	/// Delete all documents matching the filter
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let filter = doc! { "age": { "$lt": 18 } };
	/// let count = backend.delete_many("users", filter).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn delete_many(&self, collection: &str, filter: Document) -> Result<u64> {
		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let result = coll
			.delete_many(filter)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		Ok(result.deleted_count)
	}

	/// Execute an aggregation pipeline
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::backends::mongodb::MongoDBBackend;
	/// use bson::doc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?
	///     .with_database("mydb");
	///
	/// let pipeline = vec![
	///     doc! { "$match": { "age": { "$gte": 18 } } },
	///     doc! { "$group": { "_id": "$status", "count": { "$sum": 1 } } },
	/// ];
	/// let results = backend.aggregate("users", pipeline).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn aggregate(
		&self,
		collection: &str,
		pipeline: Vec<Document>,
	) -> Result<Vec<Document>> {
		use futures::stream::TryStreamExt;

		let db = self.database();
		let coll = db.collection::<Document>(collection);

		let cursor = coll
			.aggregate(pipeline)
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))?;

		cursor
			.try_collect()
			.await
			.map_err(|e| crate::error::DatabaseError::QueryError(e.to_string()))
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
			QueryValue::Now => {
				// MongoDB doesn't have SQL NOW(), use current UTC time
				let now = chrono::Utc::now();
				Bson::DateTime(bson::DateTime::from_millis(now.timestamp_millis()))
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
				QueryValue::Timestamp(
					chrono::Utc
						.timestamp_millis_opt(dt.timestamp_millis())
						.unwrap(),
				)
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

	async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
		// Start a new session from the client
		let mut session = self.client.start_session().await.map_err(|e| {
			crate::error::DatabaseError::ConnectionError(format!(
				"Failed to start MongoDB session: {}",
				e
			))
		})?;

		// Begin the transaction on the session
		session.start_transaction().await.map_err(|e| {
			crate::error::DatabaseError::TransactionError(format!(
				"Failed to start MongoDB transaction: {}",
				e
			))
		})?;

		Ok(Box::new(MongoDBTransactionExecutor::new(
			session,
			Arc::clone(&self.client),
			self.database_name.clone(),
		)))
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
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

	#[test]
	fn test_builder_default() {
		let builder = MongoDBBackendBuilder::new();
		assert_eq!(builder.url, "mongodb://localhost:27017");
		assert_eq!(builder.database, "test");
		assert_eq!(builder.max_pool_size, None);
		assert_eq!(builder.min_pool_size, None);
	}

	#[test]
	fn test_builder_configuration() {
		let builder = MongoDBBackendBuilder::new()
			.url("mongodb://example.com:27017")
			.database("mydb")
			.max_pool_size(100)
			.min_pool_size(10)
			.max_idle_time_secs(300);

		assert_eq!(builder.url, "mongodb://example.com:27017");
		assert_eq!(builder.database, "mydb");
		assert_eq!(builder.max_pool_size, Some(100));
		assert_eq!(builder.min_pool_size, Some(10));
		assert_eq!(builder.max_idle_time_secs, Some(300));
	}

	#[test]
	fn test_backend_builder_method() {
		let builder = MongoDBBackend::builder();
		assert_eq!(builder.url, "mongodb://localhost:27017");
	}
}
