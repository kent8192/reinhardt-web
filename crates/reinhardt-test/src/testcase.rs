//! Base test case with common setup and assertions
//!
//! Similar to DRF's APITestCase

use crate::client::APIClient;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Base test case for API testing
///
/// Provides:
/// - Pre-configured APIClient
/// - Common setup/teardown hooks
/// - Assertion helpers
/// - Optional TestContainer database integration
///
/// # Example
/// ```ignore
/// struct MyTest {
///     case: APITestCase,
/// }
///
/// impl MyTest {
///     async fn test_list_users(&self) {
///         let response = self.case.client().get("/api/users/").await.unwrap();
///         response.assert_ok();
///     }
/// }
/// ```
pub struct APITestCase {
    client: Arc<RwLock<APIClient>>,
    setup_called: Arc<RwLock<bool>>,
    teardown_called: Arc<RwLock<bool>>,
    #[cfg(feature = "testcontainers")]
    database_url: Arc<RwLock<Option<String>>>,
}

impl APITestCase {
    /// Create a new test case
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(APIClient::new())),
            setup_called: Arc::new(RwLock::new(false)),
            teardown_called: Arc::new(RwLock::new(false)),
            #[cfg(feature = "testcontainers")]
            database_url: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a test case with a custom client
    pub fn with_client(client: APIClient) -> Self {
        Self {
            client: Arc::new(RwLock::new(client)),
            setup_called: Arc::new(RwLock::new(false)),
            teardown_called: Arc::new(RwLock::new(false)),
            #[cfg(feature = "testcontainers")]
            database_url: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a test case with a database connection URL
    #[cfg(feature = "testcontainers")]
    pub fn with_database_url(url: String) -> Self {
        Self {
            client: Arc::new(RwLock::new(APIClient::new())),
            setup_called: Arc::new(RwLock::new(false)),
            teardown_called: Arc::new(RwLock::new(false)),
            database_url: Arc::new(RwLock::new(Some(url))),
        }
    }

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

    /// Setup method called before each test
    pub async fn setup(&self) {
        let mut setup = self.setup_called.write().await;
        *setup = true;
    }

    /// Teardown method called after each test
    pub async fn teardown(&self) {
        let mut teardown = self.teardown_called.write().await;
        *teardown = true;

        // Clean up client state
        let client = self.client.read().await;
        let _ = client.logout().await;
    }

    /// Check if setup was called
    pub async fn is_setup_called(&self) -> bool {
        *self.setup_called.read().await
    }

    /// Check if teardown was called
    pub async fn is_teardown_called(&self) -> bool {
        *self.teardown_called.read().await
    }
}

impl Default for APITestCase {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for defining test cases
///
/// # Example
/// ```ignore
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
        #[tokio::test]
        async fn $name() {
            let $case = APITestCase::new();
            $case.setup().await;

            // Run test
            $body

            $case.teardown().await;
        }
    };
}

/// Helper macro for defining authenticated test cases
#[macro_export]
macro_rules! authenticated_test_case {
    (
        async fn $name:ident($case:ident: &APITestCase, $user:ident: serde_json::Value) $body:block
    ) => {
        #[tokio::test]
        async fn $name() {
            let $case = APITestCase::new();
            $case.setup().await;

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

            $case.teardown().await;
        }
    };
}

/// Helper macro for defining test cases with database containers
///
/// Requires `testcontainers` feature to be enabled.
///
/// # Example
/// ```ignore
/// test_case_with_db! {
///     postgres,
///     async fn test_users_with_db(case: &APITestCase) {
///         let db_url = case.database_url().await.unwrap();
///         // Use database...
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
        #[tokio::test]
        #[ignore] // Requires Docker
        async fn $name() {
            use $crate::containers::{with_postgres, PostgresContainer};

            with_postgres(|db| async move {
                let $case = APITestCase::with_database_url(db.connection_url());
                $case.setup().await;

                // Run test
                $body

                $case.teardown().await;
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
        #[tokio::test]
        #[ignore] // Requires Docker
        async fn $name() {
            use $crate::containers::{with_mysql, MySqlContainer};

            with_mysql(|db| async move {
                let $case = APITestCase::with_database_url(db.connection_url());
                $case.setup().await;

                // Run test
                $body

                $case.teardown().await;
                Ok(())
            })
            .await
            .unwrap();
        }
    };
}
