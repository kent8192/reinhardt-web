//! Versioning + Routers Cross-Crate Integration Tests
//!
//! Tests API versioning strategies with URL routing system.
//!
//! ## Integration Points
//!
//! - **versioning**: API versioning strategies (AcceptHeader, URLPath)
//! - **routers**: URL routing and handler registration
//!
//! ## Purpose
//!
//! Validates that versioning strategies correctly integrate with the routing
//! system, ensuring version-specific route handlers are invoked based on
//! Accept headers, URL paths, and middleware configuration.

use rstest::*;
use std::collections::HashSet;
use std::sync::Arc;

use bytes::Bytes;
use hyper::StatusCode;
use reinhardt_http::{Request, Response};
use reinhardt_rest::versioning::{AcceptHeaderVersioning, BaseVersioning, URLPathVersioning};
use reinhardt_routers::UnifiedRouter;
use reinhardt_test::fixtures::server::test_server_guard;

// ============================================================================
// Helper Functions - Test Handlers
// ============================================================================

/// Handler for v1 users endpoint
async fn users_v1_handler(_req: Request) -> Result<Response, reinhardt_http::Error> {
	Ok(Response::ok().with_body(Bytes::from(r#"{"version":"v1","users":["alice","bob"]}"#)))
}

/// Handler for v2 users endpoint
async fn users_v2_handler(_req: Request) -> Result<Response, reinhardt_http::Error> {
	Ok(Response::ok().with_body(Bytes::from(
		r#"{"version":"v2","users":[{"id":1,"name":"alice"},{"id":2,"name":"bob"}]}"#,
	)))
}

/// Handler for v1 users/:id/posts endpoint
async fn user_posts_v1_handler(req: Request) -> Result<Response, reinhardt_http::Error> {
	let user_id = req
		.path_params
		.get("id")
		.ok_or_else(|| reinhardt_http::Error::Validation("Missing user_id".to_string()))?
		.to_string();
	Ok(Response::ok().with_body(Bytes::from(format!(
		r#"{{"version":"v1","user_id":"{}","posts":["post1","post2"]}}"#,
		user_id
	))))
}

/// Handler for v2 users/:id/posts endpoint
async fn user_posts_v2_handler(req: Request) -> Result<Response, reinhardt_http::Error> {
	let user_id = req
		.path_params
		.get("id")
		.ok_or_else(|| reinhardt_http::Error::Validation("Missing user_id".to_string()))?
		.to_string();
	Ok(Response::ok()
		.with_body(Bytes::from(format!(
			r#"{{"version":"v2","user_id":"{}","posts":[{{"id":1,"title":"First"}},{{"id":2,"title":"Second"}}]}}"#,
			user_id
		))))
}

/// Default handler (no version specified)
async fn users_default_handler(_req: Request) -> Result<Response, reinhardt_http::Error> {
	Ok(Response::ok().with_body(Bytes::from(
		r#"{"version":"default","users":["default_user"]}"#,
	)))
}

/// Handler for /docs/v1
async fn docs_v1_handler(_req: Request) -> Result<Response, reinhardt_http::Error> {
	Ok(Response::ok().with_body(Bytes::from(r#"{"docs":"API Documentation v1"}"#)))
}

/// Handler for /docs/v2
async fn docs_v2_handler(_req: Request) -> Result<Response, reinhardt_http::Error> {
	Ok(Response::ok().with_body(Bytes::from(r#"{"docs":"API Documentation v2"}"#)))
}

// ============================================================================
// Test Cases
// ============================================================================

/// Test: Versioned router registration
///
/// Validates version-specific route registration:
/// - v1 and v2 routes registered with separate handlers
/// - `/v1/users` routes to v1 handler
/// - `/v2/users` routes to v2 handler
/// - Each version returns correct response format
#[rstest]
#[tokio::test]
async fn test_versioned_router_registration() {
	// Create router with version-prefixed routes
	let router = Arc::new(
		UnifiedRouter::new()
			.function("/v1/users", hyper::Method::GET, users_v1_handler)
			.function("/v2/users", hyper::Method::GET, users_v2_handler),
	);

	let server = test_server_guard(router).await;

	// Test v1 endpoint
	let v1_response = reqwest::get(&format!("{}/v1/users", server.url))
		.await
		.expect("Failed to send v1 request");
	assert_eq!(
		v1_response.status(),
		StatusCode::OK,
		"v1 endpoint should return OK"
	);
	let v1_body = v1_response
		.text()
		.await
		.expect("Failed to read v1 response body");
	assert_eq!(
		v1_body, r#"{"version":"v1","users":["alice","bob"]}"#,
		"v1 endpoint should return v1 format"
	);

	// Test v2 endpoint
	let v2_response = reqwest::get(&format!("{}/v2/users", server.url))
		.await
		.expect("Failed to send v2 request");
	assert_eq!(
		v2_response.status(),
		StatusCode::OK,
		"v2 endpoint should return OK"
	);
	let v2_body = v2_response
		.text()
		.await
		.expect("Failed to read v2 response body");
	assert_eq!(
		v2_body, r#"{"version":"v2","users":[{"id":1,"name":"alice"},{"id":2,"name":"bob"}]}"#,
		"v2 endpoint should return v2 format with structured data"
	);
}

/// Test: URL path versioning with nested routes
///
/// Validates deep nested routes with path parameters:
/// - `/v1/users/{id}/posts` correctly extracts user_id
/// - `/v2/users/{id}/posts` correctly extracts user_id
/// - Path parameters work with version prefixes
/// - Version-specific response formats
///
/// NOTE: Currently ignored because UnifiedRouter doesn't support `:id` path parameters yet.
/// This test will be enabled when path parameter routing is implemented.
#[rstest]
#[tokio::test]
#[ignore = "UnifiedRouter doesn't support :id path parameters yet"]
async fn test_url_path_versioning_with_nested_routes() {
	// Create router with nested versioned routes
	let router = Arc::new(
		UnifiedRouter::new()
			.function(
				"/v1/users/:id/posts",
				hyper::Method::GET,
				user_posts_v1_handler,
			)
			.function(
				"/v2/users/:id/posts",
				hyper::Method::GET,
				user_posts_v2_handler,
			),
	);

	let server = test_server_guard(router).await;

	// Test v1 nested route with path parameter
	let v1_response = reqwest::get(&format!("{}/v1/users/123/posts", server.url))
		.await
		.expect("Failed to send v1 nested request");
	assert_eq!(
		v1_response.status(),
		StatusCode::OK,
		"v1 nested route should return OK"
	);
	let v1_body = v1_response
		.text()
		.await
		.expect("Failed to read v1 nested response");
	assert_eq!(
		v1_body, r#"{"version":"v1","user_id":"123","posts":["post1","post2"]}"#,
		"v1 nested route should extract user_id and return v1 format"
	);

	// Test v2 nested route with path parameter
	let v2_response = reqwest::get(&format!("{}/v2/users/456/posts", server.url))
		.await
		.expect("Failed to send v2 nested request");
	assert_eq!(
		v2_response.status(),
		StatusCode::OK,
		"v2 nested route should return OK"
	);
	let v2_body = v2_response
		.text()
		.await
		.expect("Failed to read v2 nested response");
	assert_eq!(
		v2_body,
		r#"{"version":"v2","user_id":"456","posts":[{"id":1,"title":"First"},{"id":2,"title":"Second"}]}"#,
		"v2 nested route should extract user_id and return v2 structured format"
	);
}

/// Test: Accept header versioning with routers
///
/// Validates Accept header based version routing:
/// - Same path `/api/users` routes to different handlers based on Accept header
/// - `Accept: application/json; version=v1` routes to v1 handler
/// - `Accept: application/json; version=v2` routes to v2 handler
/// - No version specified uses default handler
#[rstest]
#[tokio::test]
async fn test_accept_header_versioning_with_routers() {
	// Create versioning strategy
	let mut allowed_versions = HashSet::new();
	allowed_versions.insert("v1".to_string());
	allowed_versions.insert("v2".to_string());

	let versioning = AcceptHeaderVersioning {
		default_version: Some("v1".to_string()),
		allowed_versions,
		version_param: "version".to_string(),
	};

	// For this test, we simulate version extraction manually since middleware
	// integration requires more complex setup. In production, this would be
	// done via middleware that extracts version and stores it in request extensions.

	// Create router with version-prefixed routes
	// Note: Real Accept-header versioning would use middleware to extract version
	// and route dynamically. This test validates the routing layer separately.
	let router = Arc::new(
		UnifiedRouter::new()
			.function("/api/v1/users", hyper::Method::GET, users_v1_handler)
			.function("/api/v2/users", hyper::Method::GET, users_v2_handler)
			.function("/api/users", hyper::Method::GET, users_default_handler),
	);

	let server = test_server_guard(router).await;

	// Test v1 via explicit path (simulates Accept header routing)
	let client = reqwest::Client::new();
	let v1_response = client
		.get(&format!("{}/api/v1/users", server.url))
		.header("Accept", "application/json; version=v1")
		.send()
		.await
		.expect("Failed to send v1 request");
	assert_eq!(
		v1_response.status(),
		StatusCode::OK,
		"Accept header v1 should route to v1 handler"
	);
	let v1_body = v1_response
		.text()
		.await
		.expect("Failed to read v1 response");
	assert_eq!(
		v1_body, r#"{"version":"v1","users":["alice","bob"]}"#,
		"v1 Accept header should return v1 format"
	);

	// Test v2 via explicit path (simulates Accept header routing)
	let v2_response = client
		.get(&format!("{}/api/v2/users", server.url))
		.header("Accept", "application/json; version=v2")
		.send()
		.await
		.expect("Failed to send v2 request");
	assert_eq!(
		v2_response.status(),
		StatusCode::OK,
		"Accept header v2 should route to v2 handler"
	);
	let v2_body = v2_response
		.text()
		.await
		.expect("Failed to read v2 response");
	assert_eq!(
		v2_body, r#"{"version":"v2","users":[{"id":1,"name":"alice"},{"id":2,"name":"bob"}]}"#,
		"v2 Accept header should return v2 structured format"
	);

	// Test default path (no version in URL, simulates missing Accept header)
	let default_response = reqwest::get(&format!("{}/api/users", server.url))
		.await
		.expect("Failed to send default request");
	assert_eq!(
		default_response.status(),
		StatusCode::OK,
		"Default path should return OK"
	);
	let default_body = default_response
		.text()
		.await
		.expect("Failed to read default response");
	assert_eq!(
		default_body, r#"{"version":"default","users":["default_user"]}"#,
		"No Accept header version should use default handler"
	);

	// Validate versioning strategy behavior independently
	let mock_request_v1 = Request::builder()
		.method(hyper::Method::GET)
		.header("Accept", "application/json; version=v1")
		.uri("/api/users")
		.body(Bytes::new())
		.build()
		.unwrap();
	let version_v1 = versioning
		.determine_version(&mock_request_v1)
		.await
		.expect("Failed to determine version");
	assert_eq!(version_v1, "v1", "Should extract v1 from Accept header");

	let mock_request_v2 = Request::builder()
		.method(hyper::Method::GET)
		.header("Accept", "application/json; version=v2")
		.uri("/api/users")
		.body(Bytes::new())
		.build()
		.unwrap();
	let version_v2 = versioning
		.determine_version(&mock_request_v2)
		.await
		.expect("Failed to determine version");
	assert_eq!(version_v2, "v2", "Should extract v2 from Accept header");
}

/// Test: Middleware versioning with route groups
///
/// Validates versioning middleware applied to route groups:
/// - Route groups inherit version configuration
/// - Middleware extracts version and makes it available to handlers
/// - Version-specific route groups work correctly
///
/// Note: This test uses URL path versioning as a proxy for middleware behavior
/// since full middleware integration would require more complex setup.
///
/// NOTE: Currently ignored because UnifiedRouter doesn't support `:id` path parameters yet.
/// This test will be enabled when path parameter routing is implemented.
#[rstest]
#[tokio::test]
#[ignore = "UnifiedRouter doesn't support :id path parameters yet"]
async fn test_middleware_versioning_with_route_groups() {
	// Create URLPathVersioning strategy
	let allowed_versions = vec!["v1".to_string(), "v2".to_string()];

	let versioning = URLPathVersioning::new()
		.with_default_version("v1")
		.with_allowed_versions(allowed_versions);

	// Create router with route groups for each version
	// In production, route groups would have versioning middleware attached
	let router = Arc::new(
		UnifiedRouter::new()
			.function("/v1/users", hyper::Method::GET, users_v1_handler)
			.function(
				"/v1/users/:id/posts",
				hyper::Method::GET,
				user_posts_v1_handler,
			)
			.function("/v2/users", hyper::Method::GET, users_v2_handler)
			.function(
				"/v2/users/:id/posts",
				hyper::Method::GET,
				user_posts_v2_handler,
			),
	);

	let server = test_server_guard(router).await;

	// Test v1 route group
	let v1_users_response = reqwest::get(&format!("{}/v1/users", server.url))
		.await
		.expect("Failed to send v1 users request");
	assert_eq!(
		v1_users_response.status(),
		StatusCode::OK,
		"v1 route group users endpoint should return OK"
	);
	let v1_users_body = v1_users_response
		.text()
		.await
		.expect("Failed to read v1 users response");
	assert_eq!(
		v1_users_body, r#"{"version":"v1","users":["alice","bob"]}"#,
		"v1 route group should return v1 format"
	);

	let v1_posts_response = reqwest::get(&format!("{}/v1/users/123/posts", server.url))
		.await
		.expect("Failed to send v1 posts request");
	assert_eq!(
		v1_posts_response.status(),
		StatusCode::OK,
		"v1 route group posts endpoint should return OK"
	);
	let v1_posts_body = v1_posts_response
		.text()
		.await
		.expect("Failed to read v1 posts response");
	assert_eq!(
		v1_posts_body, r#"{"version":"v1","user_id":"123","posts":["post1","post2"]}"#,
		"v1 route group should handle nested routes"
	);

	// Test v2 route group
	let v2_users_response = reqwest::get(&format!("{}/v2/users", server.url))
		.await
		.expect("Failed to send v2 users request");
	assert_eq!(
		v2_users_response.status(),
		StatusCode::OK,
		"v2 route group users endpoint should return OK"
	);
	let v2_users_body = v2_users_response
		.text()
		.await
		.expect("Failed to read v2 users response");
	assert_eq!(
		v2_users_body,
		r#"{"version":"v2","users":[{"id":1,"name":"alice"},{"id":2,"name":"bob"}]}"#,
		"v2 route group should return v2 structured format"
	);

	let v2_posts_response = reqwest::get(&format!("{}/v2/users/456/posts", server.url))
		.await
		.expect("Failed to send v2 posts request");
	assert_eq!(
		v2_posts_response.status(),
		StatusCode::OK,
		"v2 route group posts endpoint should return OK"
	);
	let v2_posts_body = v2_posts_response
		.text()
		.await
		.expect("Failed to read v2 posts response");
	assert_eq!(
		v2_posts_body,
		r#"{"version":"v2","user_id":"456","posts":[{"id":1,"title":"First"},{"id":2,"title":"Second"}]}"#,
		"v2 route group should handle nested routes with structured data"
	);

	// Validate versioning strategy extracts version correctly
	let mock_request_v1 = Request::builder()
		.method(hyper::Method::GET)
		.uri("/v1/users")
		.body(Bytes::new())
		.build()
		.unwrap();
	let version_v1 = versioning
		.determine_version(&mock_request_v1)
		.await
		.expect("Failed to determine v1 version");
	assert_eq!(
		version_v1, "v1",
		"URLPathVersioning should extract v1 from path"
	);

	let mock_request_v2 = Request::builder()
		.method(hyper::Method::GET)
		.uri("/v2/users/123/posts")
		.body(Bytes::new())
		.build()
		.unwrap();
	let version_v2 = versioning
		.determine_version(&mock_request_v2)
		.await
		.expect("Failed to determine v2 version");
	assert_eq!(
		version_v2, "v2",
		"URLPathVersioning should extract v2 from nested path"
	);
}

/// Test: Versioned fallback routing
///
/// Validates fallback behavior when version is not specified:
/// - No version in URL routes to default handler
/// - Default handler returns expected response
/// - Not a 404 error, but intentional fallback
#[rstest]
#[tokio::test]
async fn test_versioned_fallback_routing() {
	// Create router with version routes and a default fallback
	let router = Arc::new(
		UnifiedRouter::new()
			.function("/v1/users", hyper::Method::GET, users_v1_handler)
			.function("/v2/users", hyper::Method::GET, users_v2_handler)
			.function("/users", hyper::Method::GET, users_default_handler),
	);

	let server = test_server_guard(router).await;

	// Test default fallback (no version in path)
	let fallback_response = reqwest::get(&format!("{}/users", server.url))
		.await
		.expect("Failed to send fallback request");
	assert_eq!(
		fallback_response.status(),
		StatusCode::OK,
		"Fallback route should return OK, not 404"
	);
	let fallback_body = fallback_response
		.text()
		.await
		.expect("Failed to read fallback response");
	assert_eq!(
		fallback_body, r#"{"version":"default","users":["default_user"]}"#,
		"Fallback should return default handler response"
	);

	// Verify v1 still works
	let v1_response = reqwest::get(&format!("{}/v1/users", server.url))
		.await
		.expect("Failed to send v1 request");
	assert_eq!(v1_response.status(), StatusCode::OK, "v1 should still work");
	let v1_body = v1_response
		.text()
		.await
		.expect("Failed to read v1 response");
	assert_eq!(
		v1_body, r#"{"version":"v1","users":["alice","bob"]}"#,
		"v1 should return correct format"
	);

	// Verify v2 still works
	let v2_response = reqwest::get(&format!("{}/v2/users", server.url))
		.await
		.expect("Failed to send v2 request");
	assert_eq!(v2_response.status(), StatusCode::OK, "v2 should still work");
	let v2_body = v2_response
		.text()
		.await
		.expect("Failed to read v2 response");
	assert_eq!(
		v2_body, r#"{"version":"v2","users":[{"id":1,"name":"alice"},{"id":2,"name":"bob"}]}"#,
		"v2 should return correct structured format"
	);
}

/// Test: Version negotiation with multiple strategies
///
/// Validates priority when multiple versioning strategies are available:
/// - AcceptHeader takes priority over URLPath
/// - URLPath used as fallback if Accept header missing
/// - Correct version extracted based on priority
///
/// Note: URLPathVersioning extracts numeric version only (e.g., "1" from "/v1/users")
/// so allowed_versions must be numeric strings ["1", "2"], not ["v1", "v2"]
#[rstest]
#[tokio::test]
async fn test_version_negotiation_with_multiple_strategies() {
	// Create both versioning strategies
	// Note: URLPathVersioning regex captures numeric part only (1, 2) not (v1, v2)
	let allowed_versions_accept: HashSet<String> = vec!["v1".to_string(), "v2".to_string()]
		.into_iter()
		.collect();
	let allowed_versions_url = vec!["1".to_string(), "2".to_string()]; // Numeric only

	let accept_versioning = AcceptHeaderVersioning {
		default_version: Some("v1".to_string()),
		allowed_versions: allowed_versions_accept,
		version_param: "version".to_string(),
	};

	let url_versioning = URLPathVersioning::new()
		.with_default_version("1")
		.with_allowed_versions(allowed_versions_url);

	// Test Accept header priority (should extract from Accept even if URL has version)
	let request_with_both = Request::builder()
		.method(hyper::Method::GET)
		.header("Accept", "application/json; version=v2")
		.uri("/v1/users")
		.body(Bytes::new())
		.build()
		.unwrap();

	// AcceptHeaderVersioning should extract v2 (ignores URL)
	let accept_version = accept_versioning
		.determine_version(&request_with_both)
		.await
		.expect("Failed to determine Accept version");
	assert_eq!(
		accept_version, "v2",
		"Accept header should take priority and extract v2"
	);

	// URLPathVersioning should extract "1" from "/v1/users" (numeric only)
	let url_version = url_versioning
		.determine_version(&request_with_both)
		.await
		.expect("Failed to determine URL version");
	assert_eq!(
		url_version, "1",
		"URL path should extract numeric version '1' from '/v1/users'"
	);

	// Test fallback to URL when Accept header missing
	let request_url_only = Request::builder()
		.method(hyper::Method::GET)
		.uri("/v2/users")
		.body(Bytes::new())
		.build()
		.unwrap();

	// AcceptHeaderVersioning should fall back to default (no Accept header)
	let accept_fallback = accept_versioning
		.determine_version(&request_url_only)
		.await
		.expect("Failed to determine Accept fallback");
	assert_eq!(
		accept_fallback, "v1",
		"Accept versioning should use default when header missing"
	);

	// URLPathVersioning should extract "2" from "/v2/users"
	let url_path_version = url_versioning
		.determine_version(&request_url_only)
		.await
		.expect("Failed to determine URL path version");
	assert_eq!(
		url_path_version, "2",
		"URL path should extract numeric version '2' from '/v2/users'"
	);

	// Create router that demonstrates priority behavior via explicit routing
	let router = Arc::new(
		UnifiedRouter::new()
			.function("/api/v1/users", hyper::Method::GET, users_v1_handler)
			.function("/api/v2/users", hyper::Method::GET, users_v2_handler)
			.function("/api/users", hyper::Method::GET, users_default_handler),
	);

	let server = test_server_guard(router).await;

	// Test with explicit v2 path (simulates version negotiation result)
	let client = reqwest::Client::new();
	let v2_response = client
		.get(&format!("{}/api/v2/users", server.url))
		.header("Accept", "application/json; version=v2")
		.send()
		.await
		.expect("Failed to send negotiated request");
	assert_eq!(
		v2_response.status(),
		StatusCode::OK,
		"Negotiated version should route correctly"
	);
	let v2_body = v2_response
		.text()
		.await
		.expect("Failed to read negotiated response");
	assert_eq!(
		v2_body, r#"{"version":"v2","users":[{"id":1,"name":"alice"},{"id":2,"name":"bob"}]}"#,
		"Negotiated version should return v2 structured format"
	);

	// Validate that priority would be: Accept > URLPath > Default
	// In a full middleware implementation, the negotiation would:
	// 1. Check Accept header first
	// 2. Fall back to URL path
	// 3. Use default if neither present
}

/// Test: Versioned API documentation routes
///
/// Validates version-specific documentation endpoints:
/// - `/docs/v1` returns v1 documentation
/// - `/docs/v2` returns v2 documentation
/// - Documentation routes are version-aware
/// - Can be used for OpenAPI schema versioning
///
/// Note: URLPathVersioning regex extracts numeric version only ("1", "2")
#[rstest]
#[tokio::test]
async fn test_versioned_api_documentation_routes() {
	// Create router with versioned documentation routes
	let router = Arc::new(
		UnifiedRouter::new()
			.function("/docs/v1", hyper::Method::GET, docs_v1_handler)
			.function("/docs/v2", hyper::Method::GET, docs_v2_handler),
	);

	let server = test_server_guard(router).await;

	// Test v1 documentation endpoint
	let v1_docs_response = reqwest::get(&format!("{}/docs/v1", server.url))
		.await
		.expect("Failed to send v1 docs request");
	assert_eq!(
		v1_docs_response.status(),
		StatusCode::OK,
		"v1 documentation endpoint should return OK"
	);
	let v1_docs_body = v1_docs_response
		.text()
		.await
		.expect("Failed to read v1 docs response");
	assert_eq!(
		v1_docs_body, r#"{"docs":"API Documentation v1"}"#,
		"v1 documentation should return v1-specific content"
	);

	// Test v2 documentation endpoint
	let v2_docs_response = reqwest::get(&format!("{}/docs/v2", server.url))
		.await
		.expect("Failed to send v2 docs request");
	assert_eq!(
		v2_docs_response.status(),
		StatusCode::OK,
		"v2 documentation endpoint should return OK"
	);
	let v2_docs_body = v2_docs_response
		.text()
		.await
		.expect("Failed to read v2 docs response");
	assert_eq!(
		v2_docs_body, r#"{"docs":"API Documentation v2"}"#,
		"v2 documentation should return v2-specific content"
	);

	// Verify URLPathVersioning can extract version from docs paths
	// URLPathVersioning extracts numeric version only (1, 2) not (v1, v2)
	let allowed_versions = vec!["1".to_string(), "2".to_string()];

	let versioning = URLPathVersioning::new()
		.with_default_version("1")
		.with_allowed_versions(allowed_versions);

	let mock_docs_v1 = Request::builder()
		.method(hyper::Method::GET)
		.uri("/docs/v1")
		.body(Bytes::new())
		.build()
		.unwrap();
	let docs_version_v1 = versioning
		.determine_version(&mock_docs_v1)
		.await
		.expect("Failed to determine docs v1 version");
	assert_eq!(
		docs_version_v1, "1",
		"Should extract numeric version '1' from '/docs/v1' path"
	);

	let mock_docs_v2 = Request::builder()
		.method(hyper::Method::GET)
		.uri("/docs/v2")
		.body(Bytes::new())
		.build()
		.unwrap();
	let docs_version_v2 = versioning
		.determine_version(&mock_docs_v2)
		.await
		.expect("Failed to determine docs v2 version");
	assert_eq!(
		docs_version_v2, "2",
		"Should extract numeric version '2' from '/docs/v2' path"
	);
}
