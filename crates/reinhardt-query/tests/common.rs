//! Common test utilities and fixtures for DDL integration tests

// Suppress warnings for items that may not be used in all test files.
// Each test binary compiles common.rs separately, causing unused code warnings.
#![allow(dead_code, unreachable_pub)]

use rstest::fixture;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

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
