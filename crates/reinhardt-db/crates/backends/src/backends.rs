/// Database backend modules
///
/// This module contains database-specific implementations of schema editors
/// and other backend-specific functionality.

#[cfg(feature = "postgres")]
pub mod postgresql;

#[cfg(feature = "mysql")]
pub mod mysql;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "mongodb-backend")]
pub mod mongodb;

#[cfg(feature = "mongodb-backend")]
pub use mongodb::{MongoDBBackend, MongoDBQueryBuilder, MongoDBSchemaEditor};

#[cfg(feature = "cockroachdb-backend")]
pub mod cockroachdb;

#[cfg(feature = "cockroachdb-backend")]
pub use cockroachdb::{
    CockroachDBBackend, CockroachDBConnection, CockroachDBConnectionConfig,
    CockroachDBSchemaEditor, CockroachDBTransactionManager, ClusterInfo,
};
