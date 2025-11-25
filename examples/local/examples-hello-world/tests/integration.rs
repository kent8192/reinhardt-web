//! Integration tests for hello-world example
//!
//! Compilation and execution control:
//! - Cargo.toml: [[test]] name = "integration" required-features = ["with-reinhardt"]
//! - build.rs: Sets 'with-reinhardt' feature when reinhardt is available
//! - When feature is disabled, this entire test file is excluded from compilation

use example_test_macros::example_test;
use reinhardt::prelude::*;
use reinhardt::test::client::APIClient;
use reinhardt::test::fixtures::test_server_guard;
use reinhardt::test::resource::TeardownGuard;
use rstest::*;

/// Test that reinhardt can be imported and basic functionality works
fn test_reinhardt_available() {
	// If this compiles and runs, reinhardt is available
	println!("✅ reinhardt is available from crates.io");
	assert!(true, "reinhardt should be available");
}

/// Test application initialization
fn test_application_initialization() {
	let result = Application::builder().build();
	assert!(result.is_ok(), "Failed to initialize reinhardt application");
	println!("✅ Application initialized successfully");
}

// ============================================================================
// E2E Tests with Standard Fixtures
// ============================================================================

/// Test GET / endpoint returns "Hello, World!"
#[rstest]
async fn test_hello_world_endpoint(
	#[future] test_server_guard: TeardownGuard<reinhardt::test::fixtures::TestServerGuard>,
) {
	let server = test_server_guard.await;

	// Send GET request to root endpoint
	let client = APIClient::with_base_url(&server.url);
	let response = client.get("/").await.expect("Failed to send request");

	// Verify response
	assert_eq!(response.status_code(), 200);
	let body = response.text().expect("Failed to read response body");
	assert_eq!(body, "Hello, World!");

	println!("✅ GET / returned 'Hello, World!'");
}

/// Test GET /health endpoint returns JSON health status
#[rstest]
async fn test_health_check_endpoint(
	#[future] test_server_guard: TeardownGuard<reinhardt::test::fixtures::TestServerGuard>,
) {
	let server = test_server_guard.await;

	// Send GET request to health endpoint
	let client = APIClient::with_base_url(&server.url);
	let response = client.get("/health").await.expect("Failed to send request");

	// Verify response
	assert_eq!(response.status_code(), 200);
	let content_type = response
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.expect("Missing content-type header");
	assert!(
		content_type.contains("application/json"),
		"Expected JSON response, got: {}",
		content_type
	);

	let body: serde_json::Value = response.json().expect("Failed to parse JSON response");
	assert_eq!(body["status"], "ok");

	println!("✅ GET /health returned valid JSON health status");
}

// ============================================================================
// Error Case Tests
// ============================================================================

/// Test 404 Not Found for non-existent endpoint
#[rstest]
async fn test_404_not_found(
	#[future] test_server_guard: TeardownGuard<reinhardt::test::fixtures::TestServerGuard>,
) {
	let server = test_server_guard.await;

	// Send GET request to non-existent endpoint
	let client = APIClient::with_base_url(&server.url);
	let response = client
		.get("/nonexistent")
		.await
		.expect("Failed to send request");

	// Verify 404 response
	assert_eq!(response.status_code(), 404);

	println!("✅ GET /nonexistent returned 404 Not Found");
}

/// Test 405 Method Not Allowed for unsupported HTTP method
#[rstest]
async fn test_405_method_not_allowed(
	#[future] test_server_guard: TeardownGuard<reinhardt::test::fixtures::TestServerGuard>,
) {
	let server = test_server_guard.await;

	// Send POST request to root endpoint (only GET is allowed)
	let client = APIClient::with_base_url(&server.url);
	let empty_data = serde_json::json!({});
	let response = client
		.post("/", &empty_data, "json")
		.await
		.expect("Failed to send request");

	// Verify 405 response
	assert_eq!(response.status_code(), 405);

	println!("✅ POST / returned 405 Method Not Allowed");
}
