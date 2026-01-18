//! MongoDB backend implementation
//!
//! This module provides a complete MongoDB implementation for the Reinhardt framework,
//! including connection management, query operations, and schema editing.
//!
//! # Features
//!
//! - **Connection Management**: Connection pooling, replica sets, and sharded clusters
//! - **Document Operations**: Full CRUD operations with MongoDB's native API
//! - **Schema Management**: Collection and index management
//! - **Transaction Support**: Multi-document ACID transactions (requires replica set)
//!
//! # Example
//!
//! ```rust,no_run
//! use reinhardt_db::nosql::backends::mongodb::MongoDBBackend;
//! use reinhardt_db::nosql::traits::DocumentBackend;
//! use bson::doc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to MongoDB
//! let backend = MongoDBBackend::builder()
//!     .url("mongodb://localhost:27017")
//!     .database("myapp")
//!     .max_pool_size(100)
//!     .build()
//!     .await?;
//!
//! // Insert a document
//! let id = backend.insert_one("users", doc! {
//!     "name": "Alice",
//!     "email": "alice@example.com"
//! }).await?;
//!
//! // Find the document
//! let user = backend.find_one("users", doc! { "_id": id }).await?;
//! # Ok(())
//! # }
//! ```

mod connection;

// Re-export for convenience
pub use connection::{MongoDBBackend, MongoDBBackendBuilder, MongoDBTransactionExecutor};
