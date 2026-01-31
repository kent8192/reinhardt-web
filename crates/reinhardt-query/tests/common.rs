//! Common test utilities and fixtures for DDL integration tests

// Suppress warnings for items that may not be used in all test files.
// Each test binary compiles common.rs separately, causing unused code warnings.
#![allow(dead_code, unreachable_pub)]

use rstest::fixture;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

use reinhardt_query::types::{ColumnDef, ColumnType};
use reinhardt_test::fixtures::{mysql_container, postgres_container};

// Type aliases for container types
pub type PgContainer = ContainerAsync<GenericImage>;
pub type MySqlContainer = ContainerAsync<GenericImage>;

/// PostgreSQL DDL test fixture
#[fixture]
pub async fn postgres_ddl() -> (PgContainer, Arc<sqlx::PgPool>, u16, String) {
	postgres_container().await
}

/// MySQL DDL test fixture
#[fixture]
pub async fn mysql_ddl() -> (MySqlContainer, Arc<sqlx::MySqlPool>, u16, String) {
	mysql_container().await
}

// =============================================================================
// Name Generators
// =============================================================================

/// Generate unique table name with UUID suffix
pub fn unique_table_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique schema name with UUID suffix
pub fn unique_schema_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique view name with UUID suffix
pub fn unique_view_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique index name with UUID suffix
pub fn unique_index_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique sequence name with UUID suffix
pub fn unique_sequence_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique function name with UUID suffix
pub fn unique_function_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique trigger name with UUID suffix
pub fn unique_trigger_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique type name with UUID suffix
pub fn unique_type_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique database name with UUID suffix
pub fn unique_database_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique procedure name with UUID suffix
pub fn unique_procedure_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

/// Generate unique constraint name with UUID suffix
pub fn unique_constraint_name(prefix: &str) -> String {
	format!("{}_{}", prefix, uuid::Uuid::new_v4().as_simple())
}

// =============================================================================
// Column Type Factory
// =============================================================================

/// Factory for creating column type collections for testing
pub struct ColumnTypeFactory;

impl ColumnTypeFactory {
	/// Returns all commonly used column types for cross-database testing
	pub fn all_common_types() -> Vec<ColumnType> {
		vec![
			ColumnType::TinyInteger,
			ColumnType::SmallInteger,
			ColumnType::Integer,
			ColumnType::BigInteger,
			ColumnType::Float,
			ColumnType::Double,
			ColumnType::Decimal(Some((10, 2))),
			ColumnType::Boolean,
			ColumnType::Char(Some(10)),
			ColumnType::String(Some(255)),
			ColumnType::Text,
			ColumnType::Date,
			ColumnType::Time,
			ColumnType::DateTime,
			ColumnType::Timestamp,
			ColumnType::Binary(Some(100)),
			ColumnType::Blob,
		]
	}

	/// Returns PostgreSQL-specific column types
	pub fn postgres_specific() -> Vec<ColumnType> {
		vec![
			ColumnType::Uuid,
			ColumnType::Json,
			ColumnType::JsonBinary,
			ColumnType::TimestampWithTimeZone,
			ColumnType::Array(Box::new(ColumnType::Integer)),
			ColumnType::Array(Box::new(ColumnType::Text)),
		]
	}

	/// Returns MySQL-compatible column types
	pub fn mysql_compatible() -> Vec<ColumnType> {
		vec![
			ColumnType::TinyInteger,
			ColumnType::SmallInteger,
			ColumnType::Integer,
			ColumnType::BigInteger,
			ColumnType::Float,
			ColumnType::Double,
			ColumnType::Decimal(Some((10, 2))),
			ColumnType::Boolean,
			ColumnType::Char(Some(10)),
			ColumnType::String(Some(255)),
			ColumnType::Text,
			ColumnType::Date,
			ColumnType::Time,
			ColumnType::DateTime,
			ColumnType::Timestamp,
			ColumnType::Binary(Some(100)),
			ColumnType::Blob,
			ColumnType::Json,
		]
	}

	/// Returns integer types for equivalence partitioning tests
	pub fn integer_types() -> Vec<ColumnType> {
		vec![
			ColumnType::TinyInteger,
			ColumnType::SmallInteger,
			ColumnType::Integer,
			ColumnType::BigInteger,
		]
	}

	/// Returns string types for equivalence partitioning tests
	pub fn string_types() -> Vec<ColumnType> {
		vec![
			ColumnType::Char(Some(50)),
			ColumnType::String(Some(255)),
			ColumnType::Text,
		]
	}

	/// Returns numeric precision variants for boundary value testing
	pub fn decimal_variants() -> Vec<ColumnType> {
		vec![
			ColumnType::Decimal(None),
			ColumnType::Decimal(Some((10, 0))),
			ColumnType::Decimal(Some((18, 2))),
			ColumnType::Decimal(Some((38, 10))),
		]
	}

	/// Returns temporal types for equivalence partitioning tests
	pub fn temporal_types() -> Vec<ColumnType> {
		vec![
			ColumnType::Date,
			ColumnType::Time,
			ColumnType::DateTime,
			ColumnType::Timestamp,
			ColumnType::TimestampWithTimeZone,
		]
	}
}

// =============================================================================
// Table Factory
// =============================================================================

/// Factory for creating common table definitions for testing
pub struct TableFactory;

impl TableFactory {
	/// Creates a simple users table definition
	pub fn simple_users_columns() -> Vec<ColumnDef> {
		vec![
			ColumnDef::new("id")
				.column_type(ColumnType::Integer)
				.not_null(true)
				.primary_key(true),
			ColumnDef::new("name")
				.column_type(ColumnType::String(Some(255)))
				.not_null(true),
			ColumnDef::new("email")
				.column_type(ColumnType::String(Some(255)))
				.not_null(false),
		]
	}

	/// Creates columns for a table that references another table
	pub fn referencing_table_columns() -> Vec<ColumnDef> {
		vec![
			ColumnDef::new("id")
				.column_type(ColumnType::Integer)
				.not_null(true)
				.primary_key(true),
			ColumnDef::new("user_id")
				.column_type(ColumnType::Integer)
				.not_null(true),
			ColumnDef::new("content")
				.column_type(ColumnType::Text)
				.not_null(false),
		]
	}

	/// Creates columns with all common constraints
	pub fn constrained_table_columns() -> Vec<ColumnDef> {
		vec![
			ColumnDef::new("id")
				.column_type(ColumnType::Integer)
				.not_null(true)
				.primary_key(true),
			ColumnDef::new("code")
				.column_type(ColumnType::String(Some(50)))
				.not_null(true)
				.unique(true),
			ColumnDef::new("name")
				.column_type(ColumnType::String(Some(255)))
				.not_null(true),
			ColumnDef::new("status")
				.column_type(ColumnType::String(Some(20)))
				.not_null(true),
		]
	}

	/// Creates a table with composite primary key columns
	pub fn composite_pk_columns() -> Vec<ColumnDef> {
		vec![
			ColumnDef::new("tenant_id")
				.column_type(ColumnType::Integer)
				.not_null(true),
			ColumnDef::new("entity_id")
				.column_type(ColumnType::Integer)
				.not_null(true),
			ColumnDef::new("value")
				.column_type(ColumnType::String(Some(255)))
				.not_null(false),
		]
	}
}

// =============================================================================
// Test Data Helpers
// =============================================================================

/// Helper to format PostgreSQL identifier (double-quoted)
pub fn pg_ident(name: &str) -> String {
	format!(r#""{}""#, name)
}

/// Helper to format MySQL identifier (backtick-quoted)
pub fn mysql_ident(name: &str) -> String {
	format!("`{}`", name)
}
