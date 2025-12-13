//! Base test case with common setup and assertions
//!
//! Similar to DRF's APITestCase

use crate::client::APIClient;
use crate::resource::AsyncTestResource;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Base test case for API testing
///
/// Provides:
/// - Pre-configured APIClient
/// - Automatic setup/teardown via AsyncTestResource
/// - Assertion helpers
/// - Optional TestContainer database integration
///
/// # Example
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// use reinhardt_test::testcase::APITestCase;
/// use reinhardt_test::resource::AsyncTeardownGuard;
/// use rstest::*;
///
/// #[fixture]
/// async fn api_test() -> AsyncTeardownGuard<APITestCase> {
///     AsyncTeardownGuard::new().await
/// }
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_list_users(#[future] api_test: AsyncTeardownGuard<APITestCase>) {
///     let case = api_test.await;
///     let response = case.client().await.get("/api/users/").await.unwrap();
///     response.assert_ok();
/// }
/// ```
pub struct APITestCase {
	client: Arc<RwLock<APIClient>>,
	#[cfg(feature = "testcontainers")]
	database_url: Arc<RwLock<Option<String>>>,
}

impl APITestCase {
	/// Get the database connection URL (if configured)
	#[cfg(feature = "testcontainers")]
	pub async fn database_url(&self) -> Option<String> {
		self.database_url.read().await.clone()
	}

	/// Get the test client
	pub async fn client(&self) -> tokio::sync::RwLockReadGuard<'_, APIClient> {
		self.client.read().await
	}

	/// Get mutable access to the test client
	pub async fn client_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, APIClient> {
		self.client.write().await
	}

	/// Set the database URL (useful for TestContainers integration)
	#[cfg(feature = "testcontainers")]
	pub async fn set_database_url(&self, url: String) {
		let mut db_url = self.database_url.write().await;
		*db_url = Some(url);
	}
}

#[async_trait::async_trait]
impl AsyncTestResource for APITestCase {
	async fn setup() -> Self {
		Self {
			client: Arc::new(RwLock::new(APIClient::new())),
			#[cfg(feature = "testcontainers")]
			database_url: Arc::new(RwLock::new(None)),
		}
	}

	async fn teardown(self) {
		// Clean up client state
		let client = self.client.read().await;
		let _ = client.logout().await;
	}
}

/// Helper macro for defining test cases with automatic setup/teardown
///
/// # Example
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// test_case! {
///     async fn test_get_users(case: &APITestCase) {
///         let client = case.client().await;
///         let response = client.get("/api/users/").await.unwrap();
///         response.assert_ok();
///     }
/// }
/// ```
#[macro_export]
macro_rules! test_case {
	(
        async fn $name:ident($case:ident: &APITestCase) $body:block
    ) => {
		#[rstest::rstest]
		#[tokio::test]
		async fn $name() {
			use $crate::resource::AsyncTeardownGuard;
			use $crate::testcase::APITestCase;

			let guard = AsyncTeardownGuard::<APITestCase>::new().await;
			let $case = &*guard;

			// Run test
			$body

			// guard is dropped here, teardown() is automatically called
		}
	};
}

/// Helper macro for defining authenticated test cases
#[macro_export]
macro_rules! authenticated_test_case {
    (
        async fn $name:ident($case:ident: &APITestCase, $user:ident: serde_json::Value) $body:block
    ) => {
        #[rstest::rstest]
        #[tokio::test]
        async fn $name() {
            use $crate::resource::AsyncTeardownGuard;
            use $crate::testcase::APITestCase;

            let guard = AsyncTeardownGuard::<APITestCase>::new().await;
            let $case = &*guard;

            // Setup authentication
            let $user = serde_json::json!({
                "id": 1,
                "username": "testuser",
            });
            {
                let client = $case.client().await;
                client.force_authenticate(Some($user.clone())).await;
            }

            // Run test
            $body

            // guard is dropped here, teardown() is automatically called
        }
    };
}

/// Helper macro for defining test cases with database containers
///
/// Requires `testcontainers` feature to be enabled.
///
/// This macro automatically sets up a PostgreSQL or MySQL container via TestContainers,
/// initializes an `APITestCase` with the database URL, and ensures proper cleanup.
///
/// # Examples
///
/// ## PostgreSQL Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// use reinhardt_test::test_case_with_db;
/// use reinhardt_test::testcase::APITestCase;
///
/// test_case_with_db! {
///     postgres,
///     async fn test_users_with_postgres(case: &APITestCase) {
///         let db_url = case.database_url().await.unwrap();
///         // Database URL is automatically set
///         assert!(db_url.starts_with("postgres://"));
///
///         // Perform database operations...
///     }
/// }
/// ```
///
/// ## MySQL Example
///
/// ```rust,no_run
/// use reinhardt_test::test_case_with_db;
/// use reinhardt_test::testcase::APITestCase;
///
/// test_case_with_db! {
///     mysql,
///     async fn test_users_with_mysql(case: &APITestCase) {
///         let db_url = case.database_url().await.unwrap();
///         // Database URL is automatically set
///         assert!(db_url.starts_with("mysql://"));
///
///         // Perform database operations...
///     }
/// }
/// ```
#[cfg(feature = "testcontainers")]
#[macro_export]
macro_rules! test_case_with_db {
    (
        postgres,
        async fn $name:ident($case:ident: &APITestCase) $body:block
    ) => {
        #[rstest::rstest]
        #[tokio::test]
        async fn $name() {
            use $crate::containers::{with_postgres, PostgresContainer};
            use $crate::resource::AsyncTeardownGuard;
            use $crate::testcase::APITestCase;

            with_postgres(|db| async move {
                let guard = AsyncTeardownGuard::<APITestCase>::new().await;
                let $case = &*guard;
                $case.set_database_url(db.connection_url()).await;

                // Run test
                $body

                // guard is dropped here, teardown() is automatically called
                Ok(())
            })
            .await
            .unwrap();
        }
    };
    (
        mysql,
        async fn $name:ident($case:ident: &APITestCase) $body:block
    ) => {
        #[rstest::rstest]
        #[tokio::test]
        async fn $name() {
            use $crate::containers::{with_mysql, MySqlContainer};
            use $crate::resource::AsyncTeardownGuard;
            use $crate::testcase::APITestCase;

            with_mysql(|db| async move {
                let guard = AsyncTeardownGuard::<APITestCase>::new().await;
                let $case = &*guard;
                $case.set_database_url(db.connection_url()).await;

                // Run test
                $body

                // guard is dropped here, teardown() is automatically called
                Ok(())
            })
            .await
            .unwrap();
        }
    };
}
