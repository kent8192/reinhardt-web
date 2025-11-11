//! ORM-integrated shortcut functions for database queries with 404 error handling
//!
//! These functions provide direct integration with `reinhardt-orm` for database
//! operations that return 404 errors when objects are not found.
//!
//! This module is only available with the `database` feature enabled.

#[cfg(feature = "database")]
use reinhardt_core::http::Response;
#[cfg(feature = "database")]
use reinhardt_db::prelude::Model;

/// Get a single object from the database or return a 404 response
///
/// This function directly integrates with the ORM to query the database and
/// returns a 404 HTTP response if the object is not found.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::get_object_or_404;
/// use reinhardt_db::orm::Model;
///
/// // In an async view handler:
/// async fn user_detail(user_id: i64) -> Result<Response, Response> {
///     let user = get_object_or_404::<User>(user_id).await?;
///     // user is guaranteed to exist here
///     Ok(render_json(&user))
/// }
/// ```
///
/// # Arguments
///
/// * `pk` - The primary key of the object to retrieve
///
/// # Returns
///
/// Either the queried object or a 404 Response
///
/// # Errors
///
/// Returns `Err(Response)` with HTTP 404 if the object is not found,
/// or HTTP 500 if a database error occurs.
#[cfg(feature = "database")]
pub async fn get_object_or_404<M>(pk: M::PrimaryKey) -> Result<M, Response>
where
	M: Model + serde::de::DeserializeOwned + 'static,
	M::PrimaryKey: ToString,
{
	use reinhardt_db::prelude::Manager;

	// Get the manager for this model
	let manager = Manager::<M>::new();

	// Query by primary key
	let queryset = manager.get(pk);

	// Execute the query - await the async result
	let results = queryset.all().await.map_err(|e| {
		eprintln!("Database query error in get_object_or_404: {:?}", e);
		Response::internal_server_error()
	})?;

	match results.into_iter().next() {
		Some(obj) => Ok(obj),
		None => Err(Response::not_found()),
	}
}

/// Get a list of objects from the database or return a 404 response if empty
///
/// This function queries the database using the provided `QuerySet` and returns
/// a 404 HTTP response if the result list is empty.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::get_list_or_404;
/// use reinhardt_db::orm::{Model, QuerySet};
///
/// // In an async view handler:
/// async fn user_list(status: &str) -> Result<Response, Response> {
///     let queryset = User::objects()
///         .filter("status", FilterOperator::Eq, FilterValue::String(status.to_string()));
///
///     let users = get_list_or_404(queryset).await?;
///     // users is guaranteed to be non-empty here
///     Ok(render_json(&users))
/// }
/// ```
///
/// # Arguments
///
/// * `queryset` - A QuerySet to execute
///
/// # Returns
///
/// Either a non-empty list of objects or a 404 Response
///
/// # Errors
///
/// Returns `Err(Response)` with HTTP 404 if the result list is empty,
/// or HTTP 500 if a database error occurs.
#[cfg(feature = "database")]
pub async fn get_list_or_404<M>(
	queryset: reinhardt_db::prelude::QuerySet<M>,
) -> Result<Vec<M>, Response>
where
	M: Model + 'static,
{
	// Execute the query - await the async result
	let results = queryset.all().await.map_err(|e| {
		eprintln!("Database query error in get_list_or_404: {:?}", e);
		Response::internal_server_error()
	})?;

	if results.is_empty() {
		Err(Response::not_found())
	} else {
		Ok(results)
	}
}

#[cfg(all(test, feature = "database"))]
mod tests {
	use super::*;
	use reinhardt_db::prelude::{Model, QuerySet};
	use reinhardt_test::resource::{AsyncTeardownGuard, AsyncTestResource};
	use rstest::*;
	use serde::{Deserialize, Serialize};
	use serial_test::serial;
	use std::sync::Arc;
	use testcontainers::{ContainerAsync, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct TestUser {
		id: Option<i64>,
		username: String,
		email: String,
	}

	impl Model for TestUser {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"test_users"
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	/// Suite-wide PostgreSQL database resource
	struct PostgresSuite {
		_container: Arc<ContainerAsync<Postgres>>,
		url: String,
	}

	/// Global suite instance (shared across all tests)
	static POSTGRES_SUITE: tokio::sync::OnceCell<Arc<PostgresSuite>> =
		tokio::sync::OnceCell::const_new();

	/// Async fixture to initialize and get PostgreSQL suite
	#[fixture]
	async fn postgres_suite() -> Arc<PostgresSuite> {
		POSTGRES_SUITE
			.get_or_init(|| async {
				// Start PostgreSQL container
				let container = Postgres::default()
					.start()
					.await
					.expect("Failed to start PostgreSQL container");

				let port = container
					.get_host_port_ipv4(5432)
					.await
					.expect("Failed to get PostgreSQL port");

				let url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

				// Set larger connection pool for tests to prevent pool exhaustion
				// SAFETY: Setting environment variable before database initialization
				// This is only called once during test suite setup, before any other threads access the environment
				unsafe {
					std::env::set_var("DATABASE_POOL_MAX_CONNECTIONS", "100");
				}

				// Initialize global database connection
				reinhardt_db::prelude::init_database(&url)
					.await
					.expect("Failed to initialize database");

				// Create test_users table
				let conn = reinhardt_db::prelude::get_connection()
					.await
					.expect("Failed to get connection");
				conn.execute(
					"CREATE TABLE IF NOT EXISTS test_users (
						id SERIAL PRIMARY KEY,
						username VARCHAR(255) NOT NULL,
						email VARCHAR(255) NOT NULL
					)",
					vec![],
				)
				.await
				.expect("Failed to create test_users table");

				Arc::new(PostgresSuite {
					_container: Arc::new(container),
					url: url.clone(),
				})
			})
			.await
			.clone()
	}

	/// Resource for connection pool cleanup after each test
	struct ConnectionPoolCleanup;

	#[async_trait::async_trait]
	impl AsyncTestResource for ConnectionPoolCleanup {
		async fn setup() -> Self {
			Self
		}

		async fn teardown(self) {
			// Force connection release by getting and immediately dropping a connection
			// This ensures any Arc references are decremented
			if let Ok(_conn) = reinhardt_db::prelude::get_connection().await {
				drop(_conn);
			}

			// Allow time for sqlx pool to process the release
			tokio::time::sleep(std::time::Duration::from_millis(200)).await;
		}
	}

	/// Fixture for automatic connection pool cleanup
	#[fixture]
	async fn pool_cleanup() -> AsyncTeardownGuard<ConnectionPoolCleanup> {
		AsyncTeardownGuard::new().await
	}

	#[rstest]
	#[serial(db)]
	#[tokio::test(flavor = "multi_thread")]
	async fn test_get_object_or_404_not_found(
		#[future] postgres_suite: Arc<PostgresSuite>,
		#[future] _pool_cleanup: AsyncTeardownGuard<ConnectionPoolCleanup>,
	) {
		// Initialize suite (awaits the fixture)
		let suite = postgres_suite.await;
		let _cleanup = _pool_cleanup.await;

		// Reinitialize database connection pool for this test
		reinhardt_db::prelude::reinitialize_database(&suite.url)
			.await
			.expect("Failed to reinitialize database");

		// Clean test data before test
		// clean_test_data()
		// 	.await
		// 	.expect("Failed to clean test data");

		// Query for non-existent record
		let result = get_object_or_404::<TestUser>(999).await;
		assert!(result.is_err());

		let response = result.unwrap_err();
		// Debug: print response body if not 404
		if response.status != hyper::StatusCode::NOT_FOUND {
			let body_str = String::from_utf8_lossy(&response.body);
			eprintln!("Expected 404, got {}: {}", response.status, body_str);
		}
		assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);

		// Cleanup automatically called by AsyncTeardownGuard's Drop (via async-dropper)
	}

	#[rstest]
	#[serial(db)]
	#[tokio::test(flavor = "multi_thread")]
	async fn test_get_list_or_404_empty(
		#[future] postgres_suite: Arc<PostgresSuite>,
		#[future] _pool_cleanup: AsyncTeardownGuard<ConnectionPoolCleanup>,
	) {
		// Initialize suite (awaits the fixture)
		let suite = postgres_suite.await;
		let _cleanup = _pool_cleanup.await;

		// Reinitialize database connection pool for this test
		reinhardt_db::prelude::reinitialize_database(&suite.url)
			.await
			.expect("Failed to reinitialize database");

		// Clean test data before test
		// clean_test_data()
		// 	.await
		// 	.expect("Failed to clean test data");

		// Query empty table
		let queryset = QuerySet::<TestUser>::new();
		let result = get_list_or_404(queryset).await;
		assert!(result.is_err());

		let response = result.unwrap_err();
		assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);

		// Cleanup automatically called by AsyncTeardownGuard's Drop (via async-dropper)
	}
}
