//! Server fixtures for tests.
//!
//! Provides TestServerGuard and APIClient fixtures for HTTP testing.

use crate::config::urls::url_patterns;
use crate::test_utils::fixtures::{create_test_user, generate_test_token, TestDatabase, TestUserParams};
use crate::apps::auth::models::User;
use reinhardt::db::DatabaseConnection;
use reinhardt::UnifiedRouter;
use reinhardt_test::client::APIClient;
use reinhardt_test::fixtures::TestServerGuard;
use reinhardt_test::server::test_server_guard;
use rstest::*;
use std::sync::Arc;

/// Test context containing client, router, database, and server guard.
///
/// The server guard keeps the test server alive for the duration of the test.
pub struct TestContext {
	pub client: APIClient,
	pub router: Arc<UnifiedRouter>,
	pub db: Arc<DatabaseConnection>,
	pub _guard: TestServerGuard,
}

/// Test context fixture.
///
/// Creates a complete test environment with:
/// - PostgreSQL container with migrations
/// - HTTP test server
/// - API client configured with server URL
/// - Router for URL reversal
///
/// # Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// #[rstest]
/// #[tokio::test]
/// async fn my_test(#[future] test_context: TestContext) {
///     let context = test_context.await;
///
///     // Reverse URL and make request
///     let url = context.router.reverse("auth:register", &[]).unwrap();
///     let response = context.client.post(&url, &body, "json").await.unwrap();
///
///     assert_eq!(response.status(), StatusCode::NO_CONTENT);
/// }
/// ```
#[fixture]
pub async fn test_context(#[future] test_database: TestDatabase) -> TestContext {
	let (_container, db) = test_database.await;

	// Build router with all app routes
	let router = url_patterns();

	// Start test server with database connection
	let guard = test_server_guard((*router).clone(), Some(db.clone())).await;

	// Create API client pointing to test server
	let client = APIClient::with_base_url(&guard.url());

	TestContext {
		client,
		router,
		db,
		_guard: guard,
	}
}

/// Authenticated test context fixture.
///
/// Creates a test environment with an authenticated user.
/// The client has the Authorization header pre-configured with a valid JWT token.
///
/// # Example
///
/// ```rust,no_run
/// # #[tokio::main]
/// # async fn main() {
/// #[rstest]
/// #[tokio::test]
/// async fn my_test(#[future] authenticated_context: (TestContext, User)) {
///     let (context, user) = authenticated_context.await;
///
///     // Client already has Authorization header set
///     let url = context.router.reverse("profile:detail", &[("user_id", &user.id.to_string())]).unwrap();
///     let response = context.client.get(&url).await.unwrap();
///
///     assert_eq!(response.status(), StatusCode::OK);
/// }
/// ```
#[fixture]
pub async fn authenticated_context(#[future] test_context: TestContext) -> (TestContext, User) {
	let mut context = test_context.await;

	// Create test user
	let user = create_test_user(
		&context.db,
		TestUserParams::default()
			.with_username("authuser")
			.with_email("auth@example.com"),
	)
	.await;

	// Generate JWT token
	let token = generate_test_token(&user);

	// Set Authorization header
	context
		.client
		.set_header("Authorization", format!("Bearer {}", token))
		.await
		.expect("Failed to set authorization header");

	(context, user)
}
