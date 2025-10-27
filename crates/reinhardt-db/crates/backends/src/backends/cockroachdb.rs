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
//! use reinhardt_db_backends::backends::cockroachdb::{
//!     CockroachDBBackend,
//!     schema::CockroachDBSchemaEditor,
//! };
//! use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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
pub use distributed_tx::{CockroachDBTransactionManager, ClusterInfo};
pub use schema::CockroachDBSchemaEditor;

use crate::backends::postgresql::schema::PostgreSQLSchemaEditor;

/// CockroachDB Backend
///
/// Main backend structure that combines PostgreSQL compatibility
/// with CockroachDB-specific features.
///
/// # Examples
///
/// ```rust
/// use reinhardt_db_backends::backends::cockroachdb::CockroachDBBackend;
/// use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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
    /// use reinhardt_db_backends::backends::cockroachdb::CockroachDBBackend;
    /// use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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
    /// use reinhardt_db_backends::backends::cockroachdb::CockroachDBBackend;
    /// use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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
    /// use reinhardt_db_backends::backends::cockroachdb::CockroachDBBackend;
    /// use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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
    /// use reinhardt_db_backends::backends::cockroachdb::CockroachDBBackend;
    /// use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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
    /// use reinhardt_db_backends::backends::cockroachdb::CockroachDBBackend;
    /// use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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
    /// use reinhardt_db_backends::backends::cockroachdb::CockroachDBBackend;
    /// use reinhardt_db_backends::backends::postgresql::schema::PostgreSQLSchemaEditor;
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

impl Default for CockroachDBBackend {
    fn default() -> Self {
        Self::new(PostgreSQLSchemaEditor::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let pg_editor = PostgreSQLSchemaEditor::new();
        let backend = CockroachDBBackend::new(pg_editor);
        assert_eq!(backend.database_name(), "cockroachdb");
    }

    #[test]
    fn test_backend_default() {
        let backend = CockroachDBBackend::default();
        assert_eq!(backend.database_name(), "cockroachdb");
    }

    #[test]
    fn test_schema_editor_access() {
        let backend = CockroachDBBackend::default();
        let editor = backend.schema_editor();
        let sql = editor.show_regions_sql();
        assert_eq!(sql, "SHOW REGIONS");
    }

    #[test]
    fn test_schema_editor_mut_access() {
        let mut backend = CockroachDBBackend::default();
        let editor = backend.schema_editor_mut();
        let sql = editor.show_survival_goal_sql();
        assert_eq!(sql, "SHOW SURVIVAL GOAL");
    }

    #[test]
    fn test_supports_feature() {
        let backend = CockroachDBBackend::default();

        assert!(backend.supports_feature("multi_region"));
        assert!(backend.supports_feature("distributed_transactions"));
        assert!(backend.supports_feature("as_of_system_time"));
        assert!(backend.supports_feature("table_partitioning"));
        assert!(backend.supports_feature("regional_by_row"));
        assert!(backend.supports_feature("regional_by_table"));
        assert!(backend.supports_feature("global_tables"));
        assert!(backend.supports_feature("storing_indexes"));
        assert!(backend.supports_feature("transaction_priorities"));

        assert!(!backend.supports_feature("unknown_feature"));
    }

    #[test]
    fn test_supported_features() {
        let backend = CockroachDBBackend::default();
        let features = backend.supported_features();

        assert_eq!(features.len(), 9);
        assert!(features.contains(&"multi_region"));
        assert!(features.contains(&"distributed_transactions"));
        assert!(features.contains(&"as_of_system_time"));
        assert!(features.contains(&"table_partitioning"));
    }

    #[test]
    fn test_create_table_with_locality() {
        let backend = CockroachDBBackend::default();
        let editor = backend.schema_editor();

        let sql = editor.create_table_with_locality_sql(
            "users",
            &[("id", "UUID PRIMARY KEY"), ("email", "VARCHAR(255)")],
            "REGIONAL BY ROW",
        );

        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("LOCALITY REGIONAL BY ROW"));
    }

    #[test]
    fn test_as_of_system_time() {
        let backend = CockroachDBBackend::default();
        let editor = backend.schema_editor();

        let sql = editor.as_of_system_time_sql("SELECT * FROM users", "-10s");

        assert!(sql.contains("AS OF SYSTEM TIME"));
        assert!(sql.contains("-10s"));
    }

    #[test]
    fn test_index_with_storing() {
        let backend = CockroachDBBackend::default();
        let editor = backend.schema_editor();

        let sql = editor.create_index_with_storing_sql(
            "idx_email",
            "users",
            &["email"],
            &["name", "age"],
            false,
            None,
        );

        assert!(sql.contains("CREATE INDEX"));
        assert!(sql.contains("STORING"));
        assert!(sql.contains("\"name\""));
        assert!(sql.contains("\"age\""));
    }
}
