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
//! use reinhardt_database::backends::mongodb::MongoDBBackend;
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
//! use reinhardt_database::backends::mongodb::MongoDBQueryBuilder;
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
//! use reinhardt_database::backends::mongodb::MongoDBSchemaEditor;
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
//! use reinhardt_database::backends::mongodb::MongoDBQueryBuilder;
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

pub mod connection;
pub mod query_builder;
pub mod schema_editor;

pub use connection::MongoDBBackend;
pub use query_builder::MongoDBQueryBuilder;
pub use schema_editor::MongoDBSchemaEditor;
