//! # Reinhardt Database Backends
//!
//! Low-level database backend abstractions, schema editors, and query optimization
//! for the Reinhardt framework.
//!
//! ## Overview
//!
//! This crate provides the foundational database layer for Reinhardt, including:
//!
//! - Database-agnostic backend trait abstractions
//! - Connection pooling with optimization strategies
//! - Schema editing and migration support
//! - Query building and caching
//! - Batch operations for high-performance writes
//!
//! ## Supported Databases
//!
//! | Database | Feature Flag | Backend Type |
//! |----------|--------------|--------------|
//! | PostgreSQL | `postgres` | [`PostgresBackend`] |
//! | MySQL/MariaDB | `mysql` | [`MySqlBackend`] |
//! | SQLite | `sqlite` | [`SqliteBackend`] |
//! | CockroachDB | `cockroachdb-backend` | `CockroachDBBackend` |
//!
//! ## Core Traits
//!
//! - **[`DatabaseBackend`]**: Main trait for database operations (execute, fetch, etc.)
//! - **[`DatabaseConnection`]**: Connection management and transaction handling
//! - **[`BaseDatabaseSchemaEditor`]**: Schema modification operations (DDL)
//!
//! ## Optimization Features
//!
//! ### Query Cache
//!
//! Cache prepared statements and query results for improved performance:
//!
//! ```rust,ignore
//! use reinhardt_db::backends::{QueryCache, QueryCacheConfig};
//!
//! let config = QueryCacheConfig {
//!     max_entries: 1000,
//!     ttl_seconds: 300,
//!     enable_metrics: true,
//! };
//!
//! let cache = QueryCache::new(config);
//!
//! // Cache a prepared query
//! let cached = cache.get_or_insert("SELECT * FROM users WHERE id = $1", || {
//!     // Prepare the query
//! });
//! ```
//!
//! ### Batch Operations
//!
//! Efficiently insert or update multiple records:
//!
//! ```rust,ignore
//! use reinhardt_db::backends::{BatchOperations, BatchInsertBuilder};
//!
//! // Build a batch insert
//! let batch = BatchInsertBuilder::new("users")
//!     .columns(&["name", "email"])
//!     .values(&["Alice", "alice@example.com"])
//!     .values(&["Bob", "bob@example.com"])
//!     .build();
//!
//! // Execute with automatic chunking for large datasets
//! backend.batch_insert(batch).await?;
//! ```
//!
//! ## Two-Phase Commit (Distributed Transactions)
//!
//! For distributed transaction support across multiple databases:
//!
//! - **PostgreSQL**: [`PostgresTwoPhaseParticipant`] with `PREPARE TRANSACTION`
//! - **MySQL**: [`MySqlTwoPhaseParticipant`] with XA transactions
//!
//! ```rust,ignore
//! use reinhardt_db::backends::PostgresTwoPhaseParticipant;
//!
//! // Prepare a distributed transaction
//! let participant = PostgresTwoPhaseParticipant::new(&connection);
//! participant.prepare("tx_001").await?;
//!
//! // ... coordinate with other participants ...
//!
//! participant.commit().await?; // or rollback()
//! ```
//!
//! ## Connection Pooling
//!
//! Optimized connection pool configuration:
//!
//! ```rust,ignore
//! use reinhardt_db::backends::{OptimizedPoolBuilder, PoolOptimizationConfig};
//!
//! let config = PoolOptimizationConfig {
//!     min_connections: 5,
//!     max_connections: 20,
//!     acquire_timeout_secs: 30,
//!     idle_timeout_secs: 600,
//! };
//!
//! let pool = OptimizedPoolBuilder::new(database_url)
//!     .with_config(config)
//!     .build()
//!     .await?;
//! ```
//!
//! ## Query Builder
//!
//! Type-safe query construction:
//!
//! ```rust,ignore
//! use reinhardt_db::backends::{SelectBuilder, InsertBuilder, UpdateBuilder};
//!
//! // SELECT query
//! let query = SelectBuilder::new("users")
//!     .columns(&["id", "name", "email"])
//!     .where_clause("active = $1")
//!     .order_by("created_at DESC")
//!     .limit(10)
//!     .build();
//!
//! // INSERT query
//! let query = InsertBuilder::new("users")
//!     .columns(&["name", "email"])
//!     .values(&["Alice", "alice@example.com"])
//!     .returning(&["id"])
//!     .build();
//! ```
//!
//! ## Feature Flags
//!
//! - **`postgres`**: PostgreSQL support with advanced features
//! - **`mysql`**: MySQL/MariaDB support with XA transactions
//! - **`sqlite`**: SQLite support for embedded databases
//! - **`cockroachdb-backend`**: CockroachDB distributed SQL support

pub mod backend;
pub mod connection;
pub mod dialect;
pub mod drivers;
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
pub use types::{
	DatabaseType, IsolationLevel, QueryResult, QueryValue, Row, Savepoint, TransactionExecutor,
};

// Re-export optimization features
pub use optimization::{
	BatchInsertBuilder, BatchOperations, BatchUpdateBuilder, CachedQuery, OptimizedPoolBuilder,
	PoolOptimizationConfig, QueryCache, QueryCacheConfig,
};

// Re-export database-specific schema editors
#[cfg(feature = "postgres")]
pub use drivers::postgresql::schema::PostgreSQLSchemaEditor;

#[cfg(feature = "mysql")]
pub use drivers::mysql::schema::MySQLSchemaEditor;

#[cfg(feature = "sqlite")]
pub use drivers::sqlite::schema::SQLiteSchemaEditor;

// Re-export two-phase commit implementations
#[cfg(feature = "postgres")]
pub use drivers::postgresql::two_phase::{PostgresTwoPhaseParticipant, PreparedTransactionInfo};

#[cfg(feature = "mysql")]
pub use drivers::mysql::two_phase::{
	MySqlTwoPhaseParticipant, XaSessionEnded, XaSessionPrepared, XaSessionStarted,
	XaTransactionInfo,
};

// Re-export dialect backends
#[cfg(feature = "postgres")]
pub use dialect::PostgresBackend;

#[cfg(feature = "sqlite")]
pub use dialect::SqliteBackend;

#[cfg(feature = "mysql")]
pub use dialect::MySqlBackend;

#[cfg(feature = "cockroachdb-backend")]
pub use drivers::cockroachdb::{
	ClusterInfo, CockroachDBBackend, CockroachDBConnection, CockroachDBConnectionConfig,
	CockroachDBSchemaEditor, CockroachDBTransactionManager,
};
