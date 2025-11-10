//! MongoDB backend module
//!
//! This module provides MongoDB-specific implementations for:
//! - Connection management
//! - BSON query building
//! - Collection operations (schema-less)
//!
//! # Basic Connection Example
//!
//! ```rust,no_run
//! use reinhardt_db::reinhardt_backends::mongodb::MongoDBBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to MongoDB
//! let backend = MongoDBBackend::connect("mongodb://localhost:27017").await?;
//! let backend = backend.with_database("myapp");
//! # Ok(())
//! # }
//! ```
//!
//! # Query Builder Example
//!
//! ```rust
//! use reinhardt_db::reinhardt_backends::mongodb::MongoDBQueryBuilder;
//! use bson::doc;
//!
//! // Build a query
//! let query = MongoDBQueryBuilder::new("users")
//!     .filter(doc! { "age": { "$gte": 18 } })
//!     .sort(doc! { "name": 1 })
//!     .limit(10)
//!     .skip(0);
//!
//! // Get the filter document
//! let filter = query.build_filter();
//! assert!(filter.contains_key("age"));
//! ```
//!
//! # Schema Editor Example
//!
//! ```rust,no_run
//! use reinhardt_db::reinhardt_backends::mongodb::MongoDBSchemaEditor;
//! use bson::doc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let editor = MongoDBSchemaEditor::new("mongodb://localhost:27017", "mydb").await?;
//!
//! // Create collection with validation
//! editor.create_collection("users", Some(doc! {
//!     "$jsonSchema": {
//!         "required": ["name", "email"],
//!         "properties": {
//!             "name": { "bsonType": "string" },
//!             "email": { "bsonType": "string" }
//!         }
//!     }
//! })).await?;
//!
//! // Create an index
//! editor.create_index("users", "idx_email", &["email"], true).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Aggregation Pipeline Example
//!
//! ```rust
//! use reinhardt_db::reinhardt_backends::mongodb::MongoDBQueryBuilder;
//! use bson::doc;
//!
//! let query = MongoDBQueryBuilder::new("users")
//!     .filter(doc! { "active": true })
//!     .sort(doc! { "created_at": -1 })
//!     .limit(100);
//!
//! // Build aggregation pipeline
//! let pipeline = query.build_aggregation_pipeline();
//! assert!(!pipeline.is_empty());
//! assert!(pipeline[0].contains_key("$match"));
//! ```
//!
//! # Connection Pool Configuration
//!
//! ```rust,no_run
//! use reinhardt_db::reinhardt_backends::mongodb::MongoDBBackendBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure connection pool
//! let backend = MongoDBBackendBuilder::new()
//!     .url("mongodb://localhost:27017")
//!     .database("mydb")
//!     .max_pool_size(100)
//!     .min_pool_size(10)
//!     .max_idle_time_secs(300)
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Replica Set Connection
//!
//! ```rust,no_run
//! use reinhardt_db::reinhardt_backends::mongodb::MongoDBBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to replica set
//! let backend = MongoDBBackend::builder()
//!     .url("mongodb://node1:27017,node2:27017,node3:27017/?replicaSet=myReplicaSet")
//!     .database("mydb")
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Sharded Cluster Connection
//!
//! ```rust,no_run
//! use reinhardt_db::reinhardt_backends::mongodb::MongoDBBackend;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to sharded cluster via mongos
//! let backend = MongoDBBackend::builder()
//!     .url("mongodb://mongos1:27017,mongos2:27017")
//!     .database("mydb")
//!     .max_pool_size(200) // Higher pool size for sharded clusters
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```

pub mod connection;
pub mod query_builder;
pub mod schema_editor;

pub use connection::{MongoDBBackend, MongoDBBackendBuilder};
pub use query_builder::MongoDBQueryBuilder;
pub use schema_editor::MongoDBSchemaEditor;
