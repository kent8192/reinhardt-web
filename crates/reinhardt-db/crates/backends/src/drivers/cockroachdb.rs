//! CockroachDB Backend Module
//!
//! This module provides CockroachDB-specific database backend functionality,
//! extending PostgreSQL compatibility with distributed database features.
//!
//! CockroachDB is a distributed SQL database built on PostgreSQL wire protocol,
//! offering ACID transactions, strong consistency, and horizontal scalability.
//!
//! # Features
//!
//! - **Schema Operations**: Multi-region table creation, partitioning, and region-aware indexes
//! - **Distributed Transactions**: Automatic retry logic for serialization conflicts
//! - **AS OF SYSTEM TIME**: Historical query support for consistent snapshots
//! - **Connection Management**: PostgreSQL-compatible connection pooling with CockroachDB optimizations
//!
//! # Example
//!
//! ```rust
//! use reinhardt_db::backends::cockroachdb::{
//!     CockroachDBBackend,
//!     schema::CockroachDBSchemaEditor,
//! };
//! use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
//!
//! // Create a CockroachDB backend
//! let pg_editor = PostgreSQLSchemaEditor::new();
//! let backend = CockroachDBBackend::new(pg_editor);
//!
//! // Create a multi-region table
//! let editor = backend.schema_editor();
//! let sql = editor.create_table_with_locality_sql(
//!     "users",
//!     &[("id", "UUID PRIMARY KEY"), ("name", "VARCHAR(100)")],
//!     "REGIONAL BY ROW"
//! );
//! assert!(sql.contains("LOCALITY REGIONAL BY ROW"));
//! ```

pub mod connection;
pub mod distributed_tx;
pub mod schema;

pub use connection::{CockroachDBConnection, CockroachDBConnectionConfig};
pub use distributed_tx::{ClusterInfo, CockroachDBTransactionManager};
pub use schema::CockroachDBSchemaEditor;

use crate::drivers::postgresql::schema::PostgreSQLSchemaEditor;

/// CockroachDB Backend
///
/// Main backend structure that combines PostgreSQL compatibility
/// with CockroachDB-specific features.
///
/// # Examples
///
/// ```rust
/// use reinhardt_db::backends::cockroachdb::CockroachDBBackend;
/// use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
///
/// let pg_editor = PostgreSQLSchemaEditor::new();
/// let backend = CockroachDBBackend::new(pg_editor);
/// ```
pub struct CockroachDBBackend {
	schema_editor: CockroachDBSchemaEditor,
}

impl CockroachDBBackend {
	/// Create a new CockroachDB backend
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_db::backends::cockroachdb::CockroachDBBackend;
	/// use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
	///
	/// let pg_editor = PostgreSQLSchemaEditor::new();
	/// let backend = CockroachDBBackend::new(pg_editor);
	/// ```
	pub fn new(pg_editor: PostgreSQLSchemaEditor) -> Self {
		Self {
			schema_editor: CockroachDBSchemaEditor::new(pg_editor),
		}
	}

	/// Get a reference to the schema editor
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_db::backends::cockroachdb::CockroachDBBackend;
	/// use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
	///
	/// let pg_editor = PostgreSQLSchemaEditor::new();
	/// let backend = CockroachDBBackend::new(pg_editor);
	/// let editor = backend.schema_editor();
	///
	/// let sql = editor.show_regions_sql();
	/// assert_eq!(sql, "SHOW REGIONS");
	/// ```
	pub fn schema_editor(&self) -> &CockroachDBSchemaEditor {
		&self.schema_editor
	}

	/// Get a mutable reference to the schema editor
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_db::backends::cockroachdb::CockroachDBBackend;
	/// use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
	///
	/// let pg_editor = PostgreSQLSchemaEditor::new();
	/// let mut backend = CockroachDBBackend::new(pg_editor);
	/// let editor = backend.schema_editor_mut();
	/// ```
	pub fn schema_editor_mut(&mut self) -> &mut CockroachDBSchemaEditor {
		&mut self.schema_editor
	}

	/// Get the database name identifier
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_db::backends::cockroachdb::CockroachDBBackend;
	/// use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
	///
	/// let pg_editor = PostgreSQLSchemaEditor::new();
	/// let backend = CockroachDBBackend::new(pg_editor);
	/// assert_eq!(backend.database_name(), "cockroachdb");
	/// ```
	pub fn database_name(&self) -> &str {
		"cockroachdb"
	}

	/// Check if a feature is supported
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_db::backends::cockroachdb::CockroachDBBackend;
	/// use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
	///
	/// let pg_editor = PostgreSQLSchemaEditor::new();
	/// let backend = CockroachDBBackend::new(pg_editor);
	///
	/// assert!(backend.supports_feature("multi_region"));
	/// assert!(backend.supports_feature("distributed_transactions"));
	/// assert!(backend.supports_feature("as_of_system_time"));
	/// assert!(!backend.supports_feature("unknown_feature"));
	/// ```
	pub fn supports_feature(&self, feature: &str) -> bool {
		matches!(
			feature,
			"multi_region"
				| "distributed_transactions"
				| "as_of_system_time"
				| "table_partitioning"
				| "regional_by_row"
				| "regional_by_table"
				| "global_tables"
				| "storing_indexes"
				| "transaction_priorities"
		)
	}

	/// Get a list of all supported features
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_db::backends::cockroachdb::CockroachDBBackend;
	/// use reinhardt_db::backends::postgresql::schema::PostgreSQLSchemaEditor;
	///
	/// let pg_editor = PostgreSQLSchemaEditor::new();
	/// let backend = CockroachDBBackend::new(pg_editor);
	/// let features = backend.supported_features();
	///
	/// assert!(features.contains(&"multi_region"));
	/// assert!(features.contains(&"distributed_transactions"));
	/// ```
	pub fn supported_features(&self) -> Vec<&str> {
		vec![
			"multi_region",
			"distributed_transactions",
			"as_of_system_time",
			"table_partitioning",
			"regional_by_row",
			"regional_by_table",
			"global_tables",
			"storing_indexes",
			"transaction_priorities",
		]
	}
}
