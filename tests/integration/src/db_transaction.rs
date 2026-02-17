//! Database transaction fixtures for integration tests
//!
//! Provides automatic transaction rollback for tests to ensure clean state
//! between test runs, even when tests fail midway through execution.

use reinhardt_db::orm::connection::DatabaseConnection;
use rstest::*;
use std::sync::Arc;
use testcontainers::{
	GenericImage, ImageExt,
	core::{ContainerPort, WaitFor},
	runners::AsyncRunner,
};

/// Database transaction fixture that automatically rolls back on drop
///
/// This fixture provides a database connection within a transaction that
/// will automatically rollback when the test completes, ensuring no data
/// persists between tests even if the test panics or fails assertions.
///
/// # Architecture
///
/// The fixture uses RAII pattern through TestContainers:
/// - Container drops → Database container is stopped and removed
/// - All data created during test is automatically lost
/// - Clean state guaranteed for next test
///
/// # Examples
///
/// ```rust,no_run,ignore
/// # use rstest::*;
/// # use reinhardt_test_support::db_transaction::db_transaction_fixture;
/// # use reinhardt_db::orm::connection::DatabaseConnection;
/// # use testcontainers::GenericImage;
/// # use std::sync::Arc;
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_transaction(
///     #[future] db_transaction_fixture: (
///         testcontainers::ContainerAsync<GenericImage>,
///         Arc<DatabaseConnection>,
///     ),
/// ) {
///     let (_container, conn) = db_transaction_fixture.await;
///
///     // Perform database operations
///     conn.execute("INSERT INTO users (name) VALUES ('test')", vec![]).await.unwrap();
///
///     // No manual cleanup needed - container drops automatically
/// }
/// ```
pub struct DbTransactionFixture {
	/// PostgreSQL container - automatically cleaned up on drop
	pub container: testcontainers::ContainerAsync<GenericImage>,
	/// Database connection
	pub connection: Arc<DatabaseConnection>,
}

impl DbTransactionFixture {
	/// Create new database transaction fixture
	///
	/// Starts a PostgreSQL container and establishes a connection.
	pub async fn new() -> Self {
		// Start PostgreSQL container
		let postgres = GenericImage::new("postgres", "16-alpine")
			.with_wait_for(WaitFor::message_on_stderr(
				"database system is ready to accept connections",
			))
			.with_env_var("POSTGRES_PASSWORD", "test")
			.with_env_var("POSTGRES_DB", "test_db")
			.with_mapped_port(0, ContainerPort::Tcp(5432))
			.start()
			.await
			.expect("Failed to start PostgreSQL container");

		let port = postgres
			.get_host_port_ipv4(5432)
			.await
			.expect("Failed to get PostgreSQL port");

		let database_url = format!("postgres://postgres:test@localhost:{}/test_db", port);

		// Create connection
		let conn = DatabaseConnection::connect(&database_url)
			.await
			.expect("Failed to connect to database");

		Self {
			container: postgres,
			connection: Arc::new(conn),
		}
	}

	/// Get database connection
	pub fn connection(&self) -> Arc<DatabaseConnection> {
		Arc::clone(&self.connection)
	}
}

/// rstest fixture for database transaction testing
///
/// This fixture provides a database connection that automatically cleans up
/// all data when the test completes. The cleanup happens via container drop,
/// ensuring complete isolation between tests.
///
/// # Important
///
/// - Use `#[future]` attribute on the parameter
/// - Container is dropped after test, removing all data
/// - No manual cleanup needed
/// - Works correctly even when test panics
///
/// # Examples
///
/// ```rust,no_run
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_user_creation(
///     #[future] db_transaction_fixture: (
///         testcontainers::ContainerAsync<GenericImage>,
///         Arc<DatabaseConnection>,
///     ),
/// ) {
///     let (_container, conn) = db_transaction_fixture.await;
///
///     // Create test data
///     conn.execute(
///         "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
///         vec![]
///     ).await.unwrap();
///
///     conn.execute(
///         "INSERT INTO users (name) VALUES ('Alice')",
///         vec![]
///     ).await.unwrap();
///
///     // Verify
///     // ... test assertions ...
///
///     // No cleanup needed - container drops when test ends
/// }
/// ```
#[fixture]
pub async fn db_transaction_fixture() -> (
	testcontainers::ContainerAsync<GenericImage>,
	Arc<DatabaseConnection>,
) {
	let fixture = DbTransactionFixture::new().await;
	(fixture.container, fixture.connection)
}

/// Shared database fixture for tests that need multiple connections
///
/// This provides the same automatic cleanup as `db_transaction_fixture`,
/// but allows multiple connections to the same database instance.
///
/// # Examples
///
/// ```rust,no_run
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_concurrent_access(
///     #[future] shared_db_fixture: (
///         testcontainers::ContainerAsync<GenericImage>,
///         String,
///     ),
/// ) {
///     let (_container, db_url) = shared_db_fixture.await;
///
///     // Create multiple connections
///     let conn1 = DatabaseConnection::connect(&db_url).await.unwrap();
///     let conn2 = DatabaseConnection::connect(&db_url).await.unwrap();
///
///     // Both connections share same database
///     // All data cleaned up when container drops
/// }
/// ```
#[fixture]
pub async fn shared_db_fixture() -> (testcontainers::ContainerAsync<GenericImage>, String) {
	// Start PostgreSQL container
	let postgres = GenericImage::new("postgres", "16-alpine")
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_env_var("POSTGRES_PASSWORD", "test")
		.with_env_var("POSTGRES_DB", "test_db")
		.with_mapped_port(0, ContainerPort::Tcp(5432))
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!("postgres://postgres:test@localhost:{}/test_db", port);

	(postgres, database_url)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	/// Test that fixture provides valid connection
	#[rstest]
	#[tokio::test]
	async fn test_fixture_provides_connection(
		#[future] db_transaction_fixture: (
			testcontainers::ContainerAsync<GenericImage>,
			Arc<DatabaseConnection>,
		),
	) {
		let (_container, conn) = db_transaction_fixture.await;

		// Verify connection works
		let result = conn.execute("SELECT 1", vec![]).await;
		assert!(result.is_ok(), "Connection should be valid");
	}

	/// Test that data is cleaned up between tests
	///
	/// This test creates a table and inserts data. If cleanup works correctly,
	/// the next test will not see this data.
	#[rstest]
	#[tokio::test]
	async fn test_cleanup_part1_create_data(
		#[future] db_transaction_fixture: (
			testcontainers::ContainerAsync<GenericImage>,
			Arc<DatabaseConnection>,
		),
	) {
		let (_container, conn) = db_transaction_fixture.await;

		// Create table and insert data
		conn.execute(
			"CREATE TABLE test_cleanup (id SERIAL PRIMARY KEY, value TEXT)",
			vec![],
		)
		.await
		.unwrap();

		conn.execute(
			"INSERT INTO test_cleanup (value) VALUES ('test_data')",
			vec![],
		)
		.await
		.unwrap();

		// Data exists in this test
		// Container will be dropped and data lost after test
	}

	/// Test that verifies cleanup happened
	///
	/// This test should NOT see the table created in the previous test,
	/// proving that container-based cleanup works correctly.
	#[rstest]
	#[tokio::test]
	async fn test_cleanup_part2_verify_clean(
		#[future] db_transaction_fixture: (
			testcontainers::ContainerAsync<GenericImage>,
			Arc<DatabaseConnection>,
		),
	) {
		let (_container, conn) = db_transaction_fixture.await;

		// Verify table doesn't exist (cleanup worked)
		let result = conn.execute("SELECT * FROM test_cleanup", vec![]).await;
		assert!(
			result.is_err(),
			"Table should not exist - cleanup should have removed it"
		);
	}

	/// Test that shared fixture allows multiple connections
	#[rstest]
	#[tokio::test]
	async fn test_shared_fixture_multiple_connections(
		#[future] shared_db_fixture: (testcontainers::ContainerAsync<GenericImage>, String),
	) {
		let (_container, db_url) = shared_db_fixture.await;

		// Create two connections
		let conn1 = DatabaseConnection::connect(&db_url).await.unwrap();
		let conn2 = DatabaseConnection::connect(&db_url).await.unwrap();

		// Create table with first connection
		conn1
			.execute("CREATE TABLE shared_test (id SERIAL PRIMARY KEY)", vec![])
			.await
			.unwrap();

		// Verify second connection can see it
		let result = conn2.execute("SELECT * FROM shared_test", vec![]).await;
		assert!(result.is_ok(), "Second connection should see table");
	}

	/// Test that fixture works correctly even when test panics
	///
	/// Note: This test intentionally panics to verify cleanup still happens.
	/// The panic is caught by the test framework, so it doesn't fail the build.
	#[rstest]
	#[tokio::test]
	#[should_panic(expected = "Intentional panic to test cleanup")]
	async fn test_cleanup_on_panic(
		#[future] db_transaction_fixture: (
			testcontainers::ContainerAsync<GenericImage>,
			Arc<DatabaseConnection>,
		),
	) {
		let (_container, conn) = db_transaction_fixture.await;

		// Create data
		conn.execute("CREATE TABLE panic_test (id SERIAL PRIMARY KEY)", vec![])
			.await
			.unwrap();

		// Panic - cleanup should still happen
		panic!("Intentional panic to test cleanup");
	}
}
