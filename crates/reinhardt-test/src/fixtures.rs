//! Test fixtures and utilities for Reinhardt framework testing
//!
//! This module provides reusable test fixtures, mock implementations, and
//! TestContainers-based infrastructure for testing Reinhardt applications.
//!
//! ## Module Organization
//!
//! - `loader` - Fixture data loading from JSON, factory patterns
//! - `mock` - mockall-based mock implementations for database backends
//! - `testcontainers` - Docker container fixtures (PostgreSQL, Redis, LocalStack)
//! - `resources` - Suite-wide shared resources with automatic lifecycle management
//! - `validator` - Validator integration test fixtures
//! - `auth` - Authentication integration test fixtures
//! - `admin` - Admin panel integration test fixtures
//!
//! ## Usage Examples
//!
//! ### Using Mock Database Backend
//!
//! ```rust,no_run
//! use reinhardt_test::fixtures::*;
//! use rstest::*;
//!
//! #[rstest]
//! fn test_with_mock(mut mock_database: MockDatabaseBackend) {
//!     use reinhardt_db::backends::types::{QueryResult, QueryValue};
//!
//!     // Set expectations
//!     mock_database.expect_execute()
//!         .withf(|sql, params| sql.contains("INSERT") && params.len() == 2)
//!         .times(1)
//!         .returning(|_, _| Ok(QueryResult { rows_affected: 1 }));
//!
//!     // Test code...
//! }
//! ```
//!
//! ### Using TestContainers PostgreSQL
//!
//! ```rust,no_run
//! use reinhardt_test::fixtures::*;
//! use rstest::*;
//!
//! #[rstest]
//! #[tokio::test]
//! async fn test_with_postgres(
//!     #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String)
//! ) {
//!     let (_container, pool, _port, _url) = postgres_container.await;
//!     let result = sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await;
//!     assert!(result.is_ok());
//! }
//! ```
//!
//! ### Using Suite-Wide Resources
//!
//! ```rust,no_run
//! use reinhardt_test::fixtures::*;
//! use rstest::*;
//!
//! #[rstest]
//! #[tokio::test]
//! async fn test_shared_postgres(postgres_suite: SuiteGuard<PostgresSuiteResource>) {
//!     let pool = &postgres_suite.pool;
//!     // Pool is shared across all tests in suite
//! }
//! ```

// Module declarations
pub mod loader;
pub mod mock;
pub mod server;

#[cfg(feature = "testcontainers")]
pub mod resources;
#[cfg(feature = "testcontainers")]
pub mod testcontainers;

// New fixture modules for integration tests
pub mod admin;
pub mod auth;

#[cfg(feature = "testcontainers")]
pub mod validator;

// Re-export commonly used items from submodules

// From loader module
pub use loader::{
	Factory, FactoryBuilder, FixtureError, FixtureLoader, FixtureResult, api_client,
	fixture_loader, random_test_key, temp_dir, test_config_value,
};

// From mock module
pub use mock::{MockDatabaseBackend, mock_connection, mock_database};

// From server module
pub use server::{TestServerGuard, test_server_guard};

// From testcontainers module (conditional on feature)
#[cfg(feature = "testcontainers")]
pub use testcontainers::{
	FileLockGuard, RedisClusterContainer, cockroachdb_container, localstack_fixture,
	mongodb_container, postgres_container, redis_cluster, redis_cluster_cleanup,
	redis_cluster_client, redis_cluster_container, redis_cluster_fixture, redis_cluster_lock,
	redis_cluster_ports_ready, redis_cluster_urls, redis_container,
};

// From resources module (conditional on feature)
#[cfg(feature = "testcontainers")]
pub use resources::{MySqlSuiteResource, PostgresSuiteResource, mysql_suite, postgres_suite};

// Re-export testcontainers types for convenience
#[cfg(feature = "testcontainers")]
pub use testcontainers::ContainerAsync;
#[cfg(feature = "testcontainers")]
pub use testcontainers::GenericImage;
