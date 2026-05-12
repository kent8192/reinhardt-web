//! Unit tests for ServerRouter, ServerRouter splitting, and helpers.
#![allow(deprecated)]

use super::*;
use hyper::Method;
use reinhardt_http::{Handler, Request, Response, Result};
use rstest::rstest;
use std::sync::Arc;

#[rstest]
fn test_new_router() {
	// Arrange & Act
	let router = ServerRouter::new();

	// Assert
	assert_eq!(router.prefix(), "");
	assert_eq!(router.namespace(), None);
	assert_eq!(router.children_count(), 0);
}

#[rstest]
fn test_with_prefix() {
	// Arrange & Act
	let router = ServerRouter::new().with_prefix("/api/v1");

	// Assert
	assert_eq!(router.prefix(), "/api/v1");
}

#[rstest]
fn test_with_namespace() {
	// Arrange & Act
	let router = ServerRouter::new().with_namespace("v1");

	// Assert
	assert_eq!(router.namespace(), Some("v1"));
}

#[rstest]
fn test_mount() {
	// Arrange
	let child = ServerRouter::new();

	// Act
	let router = ServerRouter::new().mount("/users/", child);

	// Assert
	assert_eq!(router.children_count(), 1);
}

#[rstest]
#[should_panic(expected = "path parameter placeholder")]
fn test_mount_panics_on_param_prefix() {
	// Arrange
	let child = ServerRouter::new();

	// Act
	// Mounting with a `{param}` placeholder in the prefix is not supported
	// and must panic at construction time.
	let _ = ServerRouter::new().mount("/orgs/{org}/clusters/", child);

	// Assert: handled by `#[should_panic]`.
}

#[rstest]
fn test_mount_inherits_di_context() {
	// Arrange
	let di_ctx =
		Arc::new(InjectionContext::builder(Arc::new(reinhardt_di::SingletonScope::new())).build());
	let child = ServerRouter::new();

	// Act
	let router = ServerRouter::new()
		.with_di_context(di_ctx.clone())
		.mount("/users/", child);

	// Assert
	assert!(router.di_context.is_some());
	assert_eq!(router.children_count(), 1);
}

#[rstest]
fn test_group() {
	// Arrange
	let users = ServerRouter::new().with_prefix("/users");
	let posts = ServerRouter::new().with_prefix("/posts");

	// Act
	let router = ServerRouter::new().group(vec![users, posts]);

	// Assert
	assert_eq!(router.children_count(), 2);
}

#[rstest]
fn test_get_all_routes() {
	// Arrange
	let router = ServerRouter::new()
		.with_prefix("/api")
		.with_namespace("api");

	// Act
	let routes = router.get_all_routes();

	// Assert
	assert_eq!(routes.len(), 0);
}

#[rstest]
fn test_get_full_namespace_no_parent() {
	// Arrange
	let router = ServerRouter::new().with_namespace("users");

	// Act & Assert
	assert_eq!(router.get_full_namespace(None), Some("users".to_string()));
}

#[rstest]
fn test_get_full_namespace_with_parent() {
	// Arrange
	let router = ServerRouter::new().with_namespace("users");

	// Act & Assert
	assert_eq!(
		router.get_full_namespace(Some("v1")),
		Some("v1:users".to_string())
	);
}

#[rstest]
fn test_get_full_namespace_no_namespace() {
	// Arrange
	let router = ServerRouter::new();

	// Act & Assert
	assert_eq!(
		router.get_full_namespace(Some("v1")),
		Some("v1".to_string())
	);
	assert_eq!(router.get_full_namespace(None), None);
}

#[rstest]
fn test_hierarchical_namespace() {
	// Arrange
	let child = ServerRouter::new().with_namespace("users");

	// Act
	let parent = ServerRouter::new()
		.with_namespace("v1")
		.mount("/users/", child);

	// Assert
	assert_eq!(parent.namespace(), Some("v1"));
	assert_eq!(parent.children_count(), 1);
}

#[rstest]
fn test_register_all_routes_with_namespace() {
	use hyper::Method;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let mut router = ServerRouter::new().with_namespace("api").function_named(
		"/health",
		Method::GET,
		"health",
		dummy_handler,
	);

	// Act
	let errors = router.register_all_routes();
	assert!(errors.is_empty());

	// Assert
	let url = router.reverse("api:health", &[]);
	assert!(url.is_some());
	assert_eq!(url.unwrap(), "/health");
}

#[rstest]
fn test_nested_namespace_registration() {
	use hyper::Method;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let users = ServerRouter::new().with_namespace("users").function_named(
		"/list",
		Method::GET,
		"list",
		dummy_handler,
	);

	let mut api = ServerRouter::new()
		.with_namespace("v1")
		.with_prefix("/api/v1")
		.mount("/users/", users);

	// Act
	let errors = api.register_all_routes();
	assert!(errors.is_empty());

	// Assert
	let url = api.reverse("v1:users:list", &[]);
	assert!(url.is_some());
	assert_eq!(url.unwrap(), "/api/v1/users/list");
}

#[rstest]
fn test_mount_prefix_inheritance() {
	// Arrange
	let child = ServerRouter::new();

	// Act
	let parent = ServerRouter::new().with_prefix("/api").mount("/v1/", child);

	// Assert
	assert_eq!(parent.children_count(), 1);
}

#[rstest]
fn test_multiple_child_routers() {
	// Arrange
	let users = ServerRouter::new().with_namespace("users");
	let posts = ServerRouter::new().with_namespace("posts");
	let comments = ServerRouter::new().with_namespace("comments");

	// Act
	let router = ServerRouter::new()
		.mount("/users/", users)
		.mount("/posts/", posts)
		.mount("/comments/", comments);

	// Assert
	assert_eq!(router.children_count(), 3);
}

#[rstest]
fn test_deep_nesting() {
	// Arrange
	let resource = ServerRouter::new().with_namespace("resource");
	let v2 = ServerRouter::new()
		.with_namespace("v2")
		.mount("/resource/", resource);
	let v1 = ServerRouter::new().with_namespace("v1").mount("/v2/", v2);

	// Act
	let api = ServerRouter::new().with_namespace("api").mount("/v1/", v1);

	// Assert
	assert_eq!(api.children_count(), 1);
}

#[tokio::test]
async fn test_route_matching_performance_many_routes() {
	use hyper::Method;
	use std::time::Instant;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let mut router = ServerRouter::new();
	for i in 0..1000 {
		router = router.function(
			&format!("/api/resource{}/action", i),
			Method::GET,
			dummy_handler,
		);
	}

	// Act
	router.compile_routes();
	let start = Instant::now();
	for _ in 0..10000 {
		let result = router.match_own_routes("/api/resource500/action", &Method::GET);
		assert!(result.is_some());
	}
	let elapsed = start.elapsed();

	// Assert
	assert!(
		elapsed.as_millis() < 100,
		"Route matching too slow: {:?}",
		elapsed
	);
}

#[tokio::test]
async fn test_route_matching_correctness() {
	use hyper::Method;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let router = ServerRouter::new()
		.function("/users/{id}", Method::GET, dummy_handler)
		.function("/users/{id}/posts", Method::GET, dummy_handler)
		.function(
			"/posts/{post_id}/comments/{comment_id}",
			Method::GET,
			dummy_handler,
		);
	router.compile_routes();

	// Act & Assert - exact path matching
	let result = router.match_own_routes("/users/123", &Method::GET);
	assert!(result.is_some());
	assert_eq!(result.unwrap().param("id"), Some("123"));

	// Act & Assert - nested path matching
	let result = router.match_own_routes("/users/456/posts", &Method::GET);
	assert!(result.is_some());
	assert_eq!(result.unwrap().param("id"), Some("456"));

	// Act & Assert - multiple parameters; verify both values AND
	// declaration order (post_id appears before comment_id in the URL).
	let route_match = router.match_own_routes("/posts/789/comments/101", &Method::GET);
	let route_match = route_match.unwrap();
	assert_eq!(route_match.param("post_id"), Some("789"));
	assert_eq!(route_match.param("comment_id"), Some("101"));
	assert_eq!(
		route_match.params,
		vec![
			("post_id".to_string(), "789".to_string()),
			("comment_id".to_string(), "101".to_string()),
		],
		"path params must be stored in URL pattern declaration order (issue #4013)"
	);

	// Act & Assert - non-matching route
	let result = router.match_own_routes("/nonexistent", &Method::GET);
	assert!(result.is_none());
}

#[tokio::test]
async fn test_route_matching_preserves_url_pattern_order_issue_4013() {
	// Regression test for issue #4013: path parameters must be exposed in
	// URL pattern declaration order (not alphabetical), so that tuple
	// extractors `Path<(T1, T2)>` populate fields by position.
	use hyper::Method;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange: alphabetical order would put `cluster_id` before `org`,
	// but URL declaration order is `org` first, `cluster_id` second.
	let router = ServerRouter::new().function(
		"/orgs/{org}/clusters/{cluster_id}/",
		Method::GET,
		dummy_handler,
	);
	router.compile_routes();

	// Act
	let route_match = router
		.match_own_routes("/orgs/myslug/clusters/5/", &Method::GET)
		.expect("route should match");

	// Assert
	assert_eq!(
		route_match.params,
		vec![
			("org".to_string(), "myslug".to_string()),
			("cluster_id".to_string(), "5".to_string()),
		],
		"matched params must follow URL declaration order (issue #4013)"
	);
}

#[tokio::test]
async fn test_route_matching_different_methods() {
	use hyper::Method;

	async fn get_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	async fn post_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let router = ServerRouter::new()
		.function("/users", Method::GET, get_handler)
		.function("/users", Method::POST, post_handler);
	router.compile_routes();

	// Act & Assert - GET method
	let result = router.match_own_routes("/users", &Method::GET);
	assert!(result.is_some());

	// Act & Assert - POST method
	let result = router.match_own_routes("/users", &Method::POST);
	assert!(result.is_some());

	// Act & Assert - unsupported method
	let result = router.match_own_routes("/users", &Method::DELETE);
	assert!(result.is_none());
}

#[rstest]
fn test_validate_routes_success() {
	use hyper::Method;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let router = ServerRouter::new()
		.function("/users/{id}", Method::GET, dummy_handler)
		.function("/posts", Method::POST, dummy_handler);

	// Act
	let result = router.validate_routes();

	// Assert
	assert!(result.is_ok());
}

#[rstest]
fn test_compile_routes_returns_errors_for_duplicate_routes() {
	use hyper::Method;

	async fn handler_a(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}
	async fn handler_b(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange - register duplicate paths for the same method
	let router = ServerRouter::new()
		.function("/users", Method::GET, handler_a)
		.function("/users", Method::GET, handler_b);

	// Act
	let errors = router.compile_routes();

	// Assert - matchit should report a conflict for duplicate routes
	assert!(!errors.is_empty());
	assert!(errors[0].contains("Failed to compile route"));
}

#[rstest]
fn test_validate_routes_returns_errors_for_invalid_patterns() {
	use hyper::Method;

	async fn handler_a(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}
	async fn handler_b(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange - duplicate routes cause matchit compilation errors
	let router = ServerRouter::new()
		.function("/items", Method::GET, handler_a)
		.function("/items", Method::GET, handler_b);

	// Act
	let result = router.validate_routes();

	// Assert
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(!errors.is_empty());
}

#[rstest]
fn test_router_recovers_from_poisoned_rwlock() {
	use hyper::Method;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let router = ServerRouter::new().function("/health", Method::GET, dummy_handler);

	// Poison the routes_compiled RwLock by panicking while holding write guard
	let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		let _guard = router.routes_compiled.write().unwrap();
		panic!("intentional panic to poison lock");
	}));

	// Act - compile_routes should recover from poisoned lock
	let errors = router.compile_routes();

	// Assert
	assert!(errors.is_empty());
	let result = router.match_own_routes("/health", &Method::GET);
	assert!(result.is_some());
}

#[rstest]
fn test_route_matching_recovers_from_poisoned_method_router() {
	use hyper::Method;

	async fn dummy_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let router = ServerRouter::new().function("/health", Method::GET, dummy_handler);
	router.compile_routes();

	// Poison the get_router RwLock
	let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		let _guard = router.get_router.write().unwrap();
		panic!("intentional panic to poison lock");
	}));

	// Act - match_own_routes should recover from poisoned lock
	let result = router.match_own_routes("/health", &Method::GET);

	// Assert - route matching should still work
	assert!(result.is_some());
}

// --- ServerRouter::exclude() tests ---

// Simple no-op middleware for testing exclude()
struct NoopMiddleware;

#[async_trait::async_trait]
impl Middleware for NoopMiddleware {
	async fn process(
		&self,
		request: reinhardt_http::Request,
		next: std::sync::Arc<dyn reinhardt_http::Handler>,
	) -> reinhardt_http::Result<reinhardt_http::Response> {
		next.handle(request).await
	}
}

fn create_test_request(path: &str) -> reinhardt_http::Request {
	reinhardt_http::Request::builder()
		.method(Method::GET)
		.uri(path)
		.version(hyper::Version::HTTP_11)
		.headers(hyper::HeaderMap::new())
		.body(bytes::Bytes::new())
		.build()
		.unwrap()
}

#[rstest]
fn test_server_router_exclude_stores_exclusion() {
	// Arrange & Act
	let router = ServerRouter::new()
		.with_middleware(NoopMiddleware)
		.exclude("/api/auth/")
		.exclude("/health");

	// Assert
	assert_eq!(router.middleware_exclusions.len(), 1);
	assert_eq!(router.middleware_exclusions[0].len(), 2);
	assert_eq!(router.middleware_exclusions[0][0], "/api/auth/");
	assert_eq!(router.middleware_exclusions[0][1], "/health");
}

#[rstest]
fn test_server_router_exclude_only_affects_last_middleware() {
	// Arrange & Act
	let router = ServerRouter::new()
		.with_middleware(NoopMiddleware)
		.exclude("/admin/")
		.with_middleware(NoopMiddleware)
		.exclude("/api/auth/");

	// Assert
	assert_eq!(router.middleware_exclusions.len(), 2);
	assert_eq!(router.middleware_exclusions[0], vec!["/admin/"]);
	assert_eq!(router.middleware_exclusions[1], vec!["/api/auth/"]);
}

#[rstest]
#[should_panic(expected = "exclude() called with no middleware")]
fn test_server_router_exclude_panics_without_middleware() {
	// Arrange & Act & Assert
	let _router = ServerRouter::new().exclude("/api/auth/");
}

#[rstest]
fn test_server_router_build_middleware_with_exclusions() {
	// Arrange
	let router = ServerRouter::new()
		.with_middleware(NoopMiddleware)
		.exclude("/admin/")
		.with_middleware(NoopMiddleware);

	// Act
	let built = router.build_middleware_with_exclusions();

	// Assert
	assert_eq!(built.len(), 2);

	let request_admin = create_test_request("/admin/dashboard");
	let request_public = create_test_request("/public");

	// First middleware (with exclusion) skips /admin/
	assert!(!built[0].should_continue(&request_admin));
	assert!(built[0].should_continue(&request_public));
	// Second middleware (no exclusion) runs for all
	assert!(built[1].should_continue(&request_admin));
	assert!(built[1].should_continue(&request_public));
}

// --- Framework-level 404/405 middleware tests (#3234) ---

// Middleware that adds a security header to responses
struct SecurityHeaderTestMiddleware;

#[async_trait::async_trait]
impl Middleware for SecurityHeaderTestMiddleware {
	async fn process(
		&self,
		request: reinhardt_http::Request,
		next: std::sync::Arc<dyn reinhardt_http::Handler>,
	) -> reinhardt_http::Result<reinhardt_http::Response> {
		let mut response = next.handle(request).await?;
		response.headers.insert(
			hyper::header::HeaderName::from_static("x-security-test"),
			hyper::header::HeaderValue::from_static("applied"),
		);
		Ok(response)
	}
}

async fn dummy_handler(_req: reinhardt_http::Request) -> reinhardt_http::Result<Response> {
	Ok(Response::ok())
}

#[rstest]
#[tokio::test]
async fn test_404_response_gets_middleware_headers() {
	// Arrange: router with middleware and a registered route
	let router = ServerRouter::new()
		.with_middleware(SecurityHeaderTestMiddleware)
		.route("/api/users/", Method::GET, dummy_handler);

	// Act: request a non-existent path
	let request = create_test_request("/nonexistent");
	let response = Handler::handle(&router, request).await.unwrap();

	// Assert: 404 response has security header from middleware
	assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
	assert_eq!(
		response
			.headers
			.get("x-security-test")
			.map(|v| v.to_str().unwrap()),
		Some("applied"),
		"Framework-level 404 response should have middleware security header"
	);
}

#[rstest]
#[tokio::test]
async fn test_405_response_gets_middleware_headers() {
	// Arrange: router with middleware and a GET-only route
	let router = ServerRouter::new()
		.with_middleware(SecurityHeaderTestMiddleware)
		.route("/api/users/", Method::GET, dummy_handler);

	// Act: send POST to a GET-only route
	let request = reinhardt_http::Request::builder()
		.method(Method::POST)
		.uri("/api/users/")
		.version(hyper::Version::HTTP_11)
		.headers(hyper::HeaderMap::new())
		.body(bytes::Bytes::new())
		.build()
		.unwrap();
	let response = Handler::handle(&router, request).await.unwrap();

	// Assert: 405 response has security header from middleware
	assert_eq!(response.status, hyper::StatusCode::METHOD_NOT_ALLOWED);
	assert_eq!(
		response
			.headers
			.get("x-security-test")
			.map(|v| v.to_str().unwrap()),
		Some("applied"),
		"Framework-level 405 response should have middleware security header"
	);
}

#[rstest]
#[tokio::test]
async fn test_404_without_middleware_returns_error() {
	// Arrange: router with no middleware
	let router = ServerRouter::new().route("/api/users/", Method::GET, dummy_handler);

	// Act: request a non-existent path
	let request = create_test_request("/nonexistent");
	let result = Handler::handle(&router, request).await;

	// Assert: returns Err (not wrapped in middleware chain)
	assert!(result.is_err(), "404 without middleware should return Err");
}

#[rstest]
#[tokio::test]
async fn test_404_respects_middleware_exclusions() {
	// Arrange: router with middleware excluded for /admin/
	let router = ServerRouter::new()
		.with_middleware(SecurityHeaderTestMiddleware)
		.exclude("/admin/")
		.route("/api/users/", Method::GET, dummy_handler);

	// Act: request non-existent path under excluded prefix
	let request = create_test_request("/admin/nonexistent");
	let response = Handler::handle(&router, request).await.unwrap();

	// Assert: 404 response but security header absent (middleware excluded)
	assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
	assert!(
		response.headers.get("x-security-test").is_none(),
		"404 under excluded path should NOT have middleware security header"
	);
}

// --- Prefix double-application fix tests (#3407, #3408) ---

#[rstest]
#[tokio::test]
async fn test_function_route_with_prefix_strips_prefix_during_compilation() {
	// Arrange: register a route whose path already contains the prefix,
	// simulating server function registration (e.g., ServerFnRegistration::PATH)
	let router = ServerRouter::new().with_prefix("/api").function(
		"/api/server_fn/test",
		Method::POST,
		dummy_handler,
	);

	// Act: resolve the full path (resolve() strips "/api" before matchit lookup)
	let result = router.resolve("/api/server_fn/test", &Method::POST);

	// Assert: route matches without double-prefix issue
	assert!(
		result.is_some(),
		"POST /api/server_fn/test should match when router has prefix /api"
	);
}

#[rstest]
#[tokio::test]
async fn test_function_route_post_with_prefix_no_405() {
	// Arrange: register a POST route with a path that includes the prefix
	let router =
		ServerRouter::new()
			.with_prefix("/api")
			.function("/api/users", Method::POST, dummy_handler);

	// Act: resolve POST request (verifies no 405 Method Not Allowed)
	let result = router.resolve("/api/users", &Method::POST);

	// Assert: POST route is reachable
	assert!(
		result.is_some(),
		"POST /api/users should match when router has prefix /api (no 405)"
	);

	// Also verify GET returns None (route is POST-only)
	let get_result = router.resolve("/api/users", &Method::GET);
	assert!(
		get_result.is_none(),
		"GET /api/users should not match a POST-only route"
	);
}

#[rstest]
#[tokio::test]
async fn test_function_route_without_prefix_overlap_still_works() {
	// Arrange: route path does not start with the prefix
	let router =
		ServerRouter::new()
			.with_prefix("/api")
			.function("/health", Method::GET, dummy_handler);

	// Act: resolve a path under the prefix
	let result = router.resolve("/api/health", &Method::GET);

	// Assert: route matches (path kept as-is since it does not start with prefix)
	assert!(
		result.is_some(),
		"/api/health should match /health route under /api prefix"
	);
}

// --- Leading slash normalization fix tests (#3419) ---
//
// strip_prefix_normalized: unit tests (normal / edge / error)

#[rstest]
// Normal: trailing-slash prefix strips correctly
#[case("/api/", "/api/auth/register/", "/auth/register/")]
// Normal: non-trailing-slash prefix strips correctly
#[case("/api", "/api/auth/register/", "/auth/register/")]
// Normal: prefix equals full path → root "/"
#[case("/api/", "/api/", "/")]
#[case("/api", "/api", "/")]
// Normal: single-segment after strip
#[case("/api/", "/api/health", "/health")]
#[case("/v1/", "/v1/users/", "/users/")]
// Edge: empty prefix returns path as-is
#[case("", "/anything", "/anything")]
#[case("", "/", "/")]
#[case("", "/a/b/c", "/a/b/c")]
// Edge: prefix is "/" — remainder loses leading slash, must be restored
#[case("/", "/health", "/health")]
#[case("/", "/a/b/c", "/a/b/c")]
// Edge: long multi-segment prefix
#[case("/api/v2/internal/", "/api/v2/internal/metrics", "/metrics")]
// Edge: path with URL-encoded segments
#[case("/api/", "/api/users%2F123/", "/users%2F123/")]
// Edge: path with hyphens and underscores
#[case("/api/", "/api/my-resource/sub_path/", "/my-resource/sub_path/")]
fn test_strip_prefix_normalized(#[case] prefix: &str, #[case] path: &str, #[case] expected: &str) {
	// Act
	let result = ServerRouter::strip_prefix_normalized(prefix, path);

	// Assert
	assert!(
		result.is_some(),
		"strip_prefix_normalized({prefix:?}, {path:?}) should return Some"
	);
	let normalized = result.unwrap();
	assert_eq!(
		normalized.as_ref(),
		expected,
		"strip_prefix_normalized({prefix:?}, {path:?})"
	);
}

#[rstest]
// Error: path doesn't start with prefix at all
#[case("/api/", "/web/page")]
#[case("/api", "/web/page")]
// Error: partial prefix match (not a real prefix)
#[case("/api/", "/ap")]
#[case("/api", "/ap")]
// Error: path is empty
#[case("/api/", "")]
#[case("/", "")]
// Error: prefix longer than path
#[case("/api/v2/", "/api/")]
fn test_strip_prefix_normalized_returns_none(#[case] prefix: &str, #[case] path: &str) {
	// Act
	let result = ServerRouter::strip_prefix_normalized(prefix, path);

	// Assert
	assert!(
		result.is_none(),
		"strip_prefix_normalized({prefix:?}, {path:?}) should return None"
	);
}

#[rstest]
fn test_strip_prefix_normalized_result_always_starts_with_slash() {
	// Arrange: various prefix/path combos that should succeed
	let cases = [
		("/api/", "/api/x"),
		("/a/b/c/", "/a/b/c/d"),
		("/", "/x"),
		("", "/x"),
		("/prefix/", "/prefix/rest/of/path"),
	];

	for (prefix, path) in cases {
		// Act
		let result = ServerRouter::strip_prefix_normalized(prefix, path);

		// Assert
		let normalized = result.unwrap();
		assert!(
			normalized.starts_with('/'),
			"result for ({prefix:?}, {path:?}) should start with '/' but got {normalized:?}"
		);
	}
}

// resolve(): normal cases with child routers

#[rstest]
#[tokio::test]
async fn test_resolve_trailing_slash_prefix_child_router_matches() {
	// Arrange: parent with trailing-slash prefix, child with its own prefix
	let child = ServerRouter::new().with_prefix("/auth/").function(
		"/auth/register/",
		Method::POST,
		dummy_handler,
	);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/auth/", child);

	// Act
	let result = parent.resolve("/api/auth/register/", &Method::POST);

	// Assert
	assert!(
		result.is_some(),
		"POST /api/auth/register/ should match child route through trailing-slash prefix"
	);
}

#[rstest]
#[tokio::test]
async fn test_resolve_no_trailing_slash_with_prefix_child_router_matches() {
	// Arrange: parent with_prefix (no trailing slash) + child mounted with trailing slash
	// Note: mount() requires trailing-slash prefix (Django convention),
	// but with_prefix() allows non-trailing-slash prefix
	let child = ServerRouter::new().with_prefix("/auth/").function(
		"/auth/login/",
		Method::POST,
		dummy_handler,
	);
	let parent = ServerRouter::new()
		.with_prefix("/api")
		.mount("/auth/", child);

	// Act
	let result = parent.resolve("/api/auth/login/", &Method::POST);

	// Assert
	assert!(
		result.is_some(),
		"POST /api/auth/login/ should match child route with non-trailing-slash parent prefix"
	);
}

#[rstest]
#[tokio::test]
async fn test_resolve_multiple_children_with_trailing_slash_prefix() {
	// Arrange: parent with trailing-slash prefix, multiple children
	let auth = ServerRouter::new().with_prefix("/auth/").function(
		"/auth/login/",
		Method::POST,
		dummy_handler,
	);
	let users =
		ServerRouter::new()
			.with_prefix("/users/")
			.function("/users/", Method::GET, dummy_handler);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/auth/", auth)
		.mount("/users/", users);

	// Act & Assert: both children should be reachable
	assert!(
		parent.resolve("/api/auth/login/", &Method::POST).is_some(),
		"POST /api/auth/login/ should match auth child"
	);
	assert!(
		parent.resolve("/api/users/", &Method::GET).is_some(),
		"GET /api/users/ should match users child"
	);
}

#[rstest]
#[tokio::test]
async fn test_resolve_child_root_route_with_trailing_slash_prefix() {
	// Arrange: child's own root route (prefix stripped → "/")
	let child = ServerRouter::new().with_prefix("/dashboard/").function(
		"/dashboard/",
		Method::GET,
		dummy_handler,
	);
	let parent = ServerRouter::new()
		.with_prefix("/app/")
		.mount("/dashboard/", child);

	// Act
	let result = parent.resolve("/app/dashboard/", &Method::GET);

	// Assert
	assert!(
		result.is_some(),
		"GET /app/dashboard/ should match child root route"
	);
}

#[rstest]
#[tokio::test]
async fn test_resolve_parent_own_route_still_works_with_trailing_slash_prefix() {
	// Arrange: parent has both own routes and children
	let child = ServerRouter::new().with_prefix("/sub/").function(
		"/sub/action/",
		Method::POST,
		dummy_handler,
	);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.function("/api/health", Method::GET, dummy_handler)
		.mount("/sub/", child);

	// Act & Assert
	assert!(
		parent.resolve("/api/health", &Method::GET).is_some(),
		"Parent's own route should still work"
	);
	assert!(
		parent.resolve("/api/sub/action/", &Method::POST).is_some(),
		"Child route should also work"
	);
}

// resolve(): deep nesting

#[rstest]
#[tokio::test]
async fn test_resolve_deeply_nested_trailing_slash_prefixes() {
	// Arrange: 3 levels of trailing-slash prefixes
	let grandchild = ServerRouter::new().with_prefix("/profile/").function(
		"/profile/",
		Method::GET,
		dummy_handler,
	);
	let child = ServerRouter::new()
		.with_prefix("/users/")
		.mount("/profile/", grandchild);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/users/", child);

	// Act
	let result = parent.resolve("/api/users/profile/", &Method::GET);

	// Assert
	assert!(
		result.is_some(),
		"GET /api/users/profile/ should match through 3 levels of trailing-slash prefix stripping"
	);
}

#[rstest]
#[tokio::test]
async fn test_resolve_mixed_trailing_and_non_trailing_slash_nesting() {
	// Arrange: with_prefix uses non-trailing slash, mount uses trailing slash
	let grandchild =
		ServerRouter::new()
			.with_prefix("/detail")
			.function("/detail/", Method::GET, dummy_handler);
	let child = ServerRouter::new()
		.with_prefix("/items/")
		.mount("/detail/", grandchild);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/items/", child);

	// Act
	let result = parent.resolve("/api/items/detail/", &Method::GET);

	// Assert
	assert!(
		result.is_some(),
		"Mixed trailing/non-trailing prefix nesting should resolve correctly"
	);
}

// resolve(): error cases (should return None)

#[rstest]
#[tokio::test]
async fn test_resolve_path_not_matching_parent_prefix() {
	// Arrange
	let child = ServerRouter::new().with_prefix("/auth/").function(
		"/auth/login/",
		Method::POST,
		dummy_handler,
	);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/auth/", child);

	// Act
	let result = parent.resolve("/web/auth/login/", &Method::POST);

	// Assert
	assert!(
		result.is_none(),
		"Path not matching parent prefix should return None"
	);
}

#[rstest]
#[tokio::test]
async fn test_resolve_path_matches_parent_but_not_child() {
	// Arrange
	let child = ServerRouter::new().with_prefix("/auth/").function(
		"/auth/login/",
		Method::POST,
		dummy_handler,
	);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/auth/", child);

	// Act: path under parent prefix but doesn't match any child
	let result = parent.resolve("/api/unknown/path/", &Method::GET);

	// Assert
	assert!(
		result.is_none(),
		"Path matching parent but not child should return None"
	);
}

#[rstest]
#[tokio::test]
async fn test_resolve_wrong_method_through_child_with_trailing_slash_prefix() {
	// Arrange: child only has POST route
	let child = ServerRouter::new().with_prefix("/auth/").function(
		"/auth/login/",
		Method::POST,
		dummy_handler,
	);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/auth/", child);

	// Act: try GET instead of POST
	let result = parent.resolve("/api/auth/login/", &Method::GET);

	// Assert
	assert!(
		result.is_none(),
		"Wrong HTTP method through child router should return None"
	);
}

// path_exists_for_any_method(): normal / error / edge

#[rstest]
#[tokio::test]
async fn test_path_exists_with_trailing_slash_prefix_and_child() {
	// Arrange
	let child =
		ServerRouter::new()
			.with_prefix("/users/")
			.function("/users/", Method::GET, dummy_handler);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/users/", child);

	// Act
	let exists = parent.path_exists_for_any_method("/api/users/");

	// Assert
	assert!(
		exists,
		"path_exists_for_any_method should find path in child router after prefix normalization"
	);
}

#[rstest]
#[tokio::test]
async fn test_path_exists_nonexistent_path_with_trailing_slash_prefix() {
	// Arrange
	let child =
		ServerRouter::new()
			.with_prefix("/users/")
			.function("/users/", Method::GET, dummy_handler);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/users/", child);

	// Act
	let exists = parent.path_exists_for_any_method("/api/nonexistent/");

	// Assert
	assert!(
		!exists,
		"path_exists_for_any_method should return false for nonexistent path"
	);
}

#[rstest]
#[tokio::test]
async fn test_path_exists_wrong_prefix_returns_false() {
	// Arrange
	let child =
		ServerRouter::new()
			.with_prefix("/users/")
			.function("/users/", Method::GET, dummy_handler);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/users/", child);

	// Act
	let exists = parent.path_exists_for_any_method("/web/users/");

	// Assert
	assert!(
		!exists,
		"path_exists_for_any_method with wrong parent prefix should return false"
	);
}

#[rstest]
#[tokio::test]
async fn test_path_exists_deeply_nested_with_trailing_slash_prefix() {
	// Arrange: 3-level nesting
	let grandchild =
		ServerRouter::new()
			.with_prefix("/edit/")
			.function("/edit/", Method::PUT, dummy_handler);
	let child = ServerRouter::new()
		.with_prefix("/items/")
		.mount("/edit/", grandchild);
	let parent = ServerRouter::new()
		.with_prefix("/api/")
		.mount("/items/", child);

	// Act
	let exists = parent.path_exists_for_any_method("/api/items/edit/");

	// Assert
	assert!(
		exists,
		"path_exists_for_any_method should find deeply nested path through trailing-slash prefixes"
	);
}

// Edge cases: compile_routes with trailing-slash prefix

#[rstest]
#[tokio::test]
async fn test_function_route_with_trailing_slash_prefix_compiles_correctly() {
	// Arrange: route path includes prefix with trailing slash
	let router = ServerRouter::new().with_prefix("/api/").function(
		"/api/server_fn/test",
		Method::POST,
		dummy_handler,
	);

	// Act
	let result = router.resolve("/api/server_fn/test", &Method::POST);

	// Assert
	assert!(
		result.is_some(),
		"Route with trailing-slash prefix should compile and resolve correctly"
	);
}

#[rstest]
#[should_panic(expected = "URL route prefix cannot be an empty string")]
fn test_mount_with_empty_prefix_panics() {
	// Arrange & Act: mounting with empty prefix should panic
	let child = ServerRouter::new().function("/catch/", Method::GET, dummy_handler);
	let _parent = ServerRouter::new().with_prefix("/api/").mount("", child);
}

#[rstest]
#[tokio::test]
async fn test_resolve_child_with_slash_prefix_under_trailing_slash_parent() {
	// Arrange: child router with "/" prefix under parent with trailing-slash prefix
	let child =
		ServerRouter::new()
			.with_prefix("/")
			.function("/catch/", Method::GET, dummy_handler);
	let parent = ServerRouter::new().with_prefix("/api/").mount("/", child);

	// Act
	let result = parent.resolve("/api/catch/", &Method::GET);

	// Assert
	assert!(
		result.is_some(),
		"Child with '/' prefix under trailing-slash parent should match"
	);
}

// ===================================================================
// Duplicate route name detection tests (Issue #3462)
// ===================================================================

#[rstest]
fn test_register_all_routes_detects_duplicate_names() {
	async fn handler_a(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}
	async fn handler_b(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange — two routes with the same name in the same router
	let mut router = ServerRouter::new()
		.with_namespace("api")
		.function_named("/users", Method::GET, "list", handler_a)
		.function_named("/items", Method::GET, "list", handler_b);

	// Act
	let errors = router.register_all_routes();

	// Assert
	assert_eq!(errors.len(), 1);
	assert!(errors[0].contains("Duplicate route name 'api:list'"));
}

#[rstest]
fn test_validate_route_names_succeeds_with_unique_names() {
	async fn handler_a(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}
	async fn handler_b(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange
	let router = ServerRouter::new()
		.with_namespace("api")
		.function_named("/users", Method::GET, "users-list", handler_a)
		.function_named("/items", Method::GET, "items-list", handler_b);

	// Act
	let result = router.validate_route_names();

	// Assert
	assert!(result.is_ok());
}

#[rstest]
fn test_validate_routes_includes_name_errors() {
	async fn handler_a(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}
	async fn handler_b(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	// Arrange — duplicate name
	let router = ServerRouter::new()
		.with_namespace("api")
		.function_named("/users", Method::GET, "list", handler_a)
		.function_named("/items", Method::GET, "list", handler_b);

	// Act
	let result = router.validate_routes();

	// Assert
	assert!(result.is_err());
	let errors = result.unwrap_err();
	assert!(errors.iter().any(|e| e.contains("Duplicate route name")));
}
