//! Database backend abstractions and schema editors
//!
//! This module provides low-level database operations, schema editing,
//! and query building capabilities.

pub mod backend;
pub mod backends;
pub mod connection;
pub mod dialect;
pub mod error;
pub mod optimization;
pub mod query_builder;
pub mod schema;
pub mod types;

// Re-export commonly used types
pub use error::DatabaseError as QueryDatabaseError;
pub use error::{DatabaseError, Result};
pub use schema::{BaseDatabaseSchemaEditor, SchemaEditorError, SchemaEditorResult};

// Re-export query abstraction types
pub use backend::DatabaseBackend;
pub use connection::DatabaseConnection;
pub use query_builder::{InsertBuilder, SelectBuilder, UpdateBuilder};
pub use types::{DatabaseType, QueryResult, QueryValue, Row};

// Re-export database-specific schema editors
#[cfg(feature = "postgres")]
pub use backends::postgresql::schema::PostgreSQLSchemaEditor;

#[cfg(feature = "mysql")]
pub use backends::mysql::schema::MySQLSchemaEditor;

#[cfg(feature = "sqlite")]
pub use backends::sqlite::schema::SQLiteSchemaEditor;

#[cfg(feature = "mongodb-backend")]
pub use backends::mongodb::{MongoDBBackend, MongoDBQueryBuilder, MongoDBSchemaEditor};

// Re-export two-phase commit implementations
#[cfg(feature = "postgres")]
pub use backends::postgresql::two_phase::{PostgresTwoPhaseParticipant, PreparedTransactionInfo};

#[cfg(feature = "mysql")]
pub use backends::mysql::two_phase::{MySqlTwoPhaseParticipant, XaTransactionInfo};

// Re-export dialect backends
#[cfg(feature = "postgres")]
pub use dialect::PostgresBackend;

#[cfg(feature = "sqlite")]
pub use dialect::SqliteBackend;

#[cfg(feature = "mysql")]
pub use dialect::MySqlBackend;
