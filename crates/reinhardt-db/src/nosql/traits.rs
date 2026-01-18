//! NoSQL backend traits
//!
//! This module provides trait definitions for different NoSQL paradigms:
//! - `NoSQLBackend`: Base trait for all NoSQL backends
//! - `DocumentBackend`: Trait for document-oriented databases (MongoDB, CouchDB) - Phase 1 (Implemented)
//! - `KeyValueBackend`: Trait for key-value stores (Redis, DynamoDB) - Phase 2 (Planned)
//! - `ColumnBackend`: Trait for column-family stores (Cassandra) - Phase 3 (Planned)
//! - `GraphBackend`: Trait for graph databases (Neo4j) - Phase 4 (Planned)

mod base;
mod document;

pub use base::NoSQLBackend;
pub use document::DocumentBackend;

// Future trait implementations (Phase 2-4):
// - KeyValueBackend: For Redis, DynamoDB
// - ColumnBackend: For Cassandra
// - GraphBackend: For Neo4j
// Uncommenting these modules requires defining the corresponding trait APIs.
// mod key_value;
// mod column;
// mod graph;
// pub use key_value::KeyValueBackend;
// pub use column::ColumnBackend;
// pub use graph::GraphBackend;
