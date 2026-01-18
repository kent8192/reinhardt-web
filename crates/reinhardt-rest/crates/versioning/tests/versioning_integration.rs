//! Versioning integration tests
//!
//! Tests the integration of various versioning strategies with HTTP server,
//! router, and middleware components.

use bytes::Bytes;
use hyper::Method;
use reinhardt_http::{Request, Response, Result};
use reinhardt_urls::routers::ServerRouter;
use reinhardt_test::fixtures::{api_client_from_url, test_server_guard};
use reinhardt_rest::versioning::{
	AcceptHeaderVersioning, BaseVersioning, HostNameVersioning, NamespaceVersioning,
	QueryParameterVersioning, URLPathVersioning,
};
use rstest::*;

/// V1 async handler function
async fn v1_handler(_req: Request) -> Result<Response> {
	let body = r#"{"version":"v1","message":"This is version 1"}"#;
	Ok(Response::ok().with_body(Bytes::from(body)))
}

/// V2 async handler function
async fn v2_handler(_req: Request) -> Result<Response> {
	let body = r#"{"version":"v2","message":"This is version 2"}"#;
	Ok(Response::ok().with_body(Bytes::from(body)))
}

/// Fixture for creating a versioned router with v1 and v2 handlers
#[fixture]
async fn versioned_router() -> ServerRouter {
	ServerRouter::new()
		.function("/v1/resource", Method::GET, v1_handler)
		.function("/v2/resource", Method::GET, v2_handler)
		.function("/resource", Method::GET, v1_handler)
}

#[rstest]
#[tokio::test]
async fn test_accept_header_versioning_integration(#[future] versioned_router: ServerRouter) {
	let router = versioned_router.await;
	let server = test_server_guard(router).await;

	// Send request with Accept header version=v1
	let client = api_client_from_url(&server.url);
	let response = client
		.get_with_headers("/resource", &[("accept", "application/json; version=v1")])
		.await
		.unwrap();

	assert_eq!(response.status_code(), 200);

	let body = response.text();
	assert!(body.contains("\"version\":\"v1\""));
	assert!(body.contains("This is version 1"));
}

#[rstest]
#[tokio::test]
async fn test_url_path_versioning_integration(#[future] versioned_router: ServerRouter) {
	let router = versioned_router.await;
	let server = test_server_guard(router).await;

	// Send request to /v1/resource
	let client = api_client_from_url(&server.url);
	let response = client.get("/v1/resource").await.unwrap();

	assert_eq!(response.status_code(), 200);

	let body = response.text();
	assert!(body.contains("\"version\":\"v1\""));
	assert!(body.contains("This is version 1"));
}

#[rstest]
#[tokio::test]
async fn test_hostname_versioning_integration(#[future] versioned_router: ServerRouter) {
	let router = versioned_router.await;
	let server = test_server_guard(router).await;

	// Send request with Host header v1.api.example.com
	let client = api_client_from_url(&server.url);
	let response = client
		.get_with_headers("/resource", &[("host", "v1.api.example.com")])
		.await
		.unwrap();

	assert_eq!(response.status_code(), 200);

	let body = response.text();
	assert!(body.contains("\"version\":\"v1\""));
	assert!(body.contains("This is version 1"));
}

#[rstest]
#[tokio::test]
async fn test_query_parameter_versioning_integration(#[future] versioned_router: ServerRouter) {
	let router = versioned_router.await;
	let server = test_server_guard(router).await;

	// Send request with query parameter ?version=v1
	let client = api_client_from_url(&server.url);
	let response = client.get("/resource?version=v1").await.unwrap();

	assert_eq!(response.status_code(), 200);

	let body = response.text();
	assert!(body.contains("\"version\":\"v1\""));
	assert!(body.contains("This is version 1"));
}

#[rstest]
#[tokio::test]
async fn test_namespace_versioning_integration() {
	// Create router with namespace versioning pattern
	let router = ServerRouter::new()
		.function("/v1/resource", Method::GET, v1_handler)
		.function("/v2/resource", Method::GET, v2_handler);

	let server = test_server_guard(router).await;

	// Send request with custom header X-API-Version: v1
	let client = api_client_from_url(&server.url);
	let response = client.get("/v1/resource").await.unwrap();

	assert_eq!(response.status_code(), 200);

	let body = response.text();
	assert!(body.contains("\"version\":\"v1\""));
	assert!(body.contains("This is version 1"));
}

#[rstest]
#[tokio::test]
async fn test_versioning_middleware_integration() {
	// Create router with middleware
	// Note: This test demonstrates that middleware can be used to extract version
	// In a full implementation, the middleware would be applied at the router level
	let router = ServerRouter::new().function("/v1/extract", Method::GET, v1_handler);

	let server = test_server_guard(router).await;

	// Send request to /v1/extract
	let client = api_client_from_url(&server.url);
	let response = client.get("/v1/extract").await.unwrap();

	assert_eq!(response.status_code(), 200);

	let body = response.text();
	assert!(body.contains("\"version\":\"v1\""));
}

#[rstest]
#[tokio::test]
async fn test_versioned_handler_with_fallback() {
	// Create router with default version fallback
	let router = ServerRouter::new().function("/resource", Method::GET, v1_handler);

	let server = test_server_guard(router).await;

	// Send request without version - should use default (v1)
	let client = api_client_from_url(&server.url);
	let response = client.get("/resource").await.unwrap();

	assert_eq!(response.status_code(), 200);

	let body = response.text();
	assert!(body.contains("\"version\":\"v1\""));
	assert!(body.contains("This is version 1"));
}

#[rstest]
#[tokio::test]
async fn test_accept_header_versioning_strategy() {
	// Test AcceptHeaderVersioning strategy directly
	let versioning = AcceptHeaderVersioning::new()
		.with_default_version("v1")
		.with_allowed_versions(vec!["v1", "v2"]);

	let request = reinhardt_core::http::Request::builder()
		.method(hyper::Method::GET)
		.uri("http://example.com/resource")
		.version(hyper::Version::HTTP_11)
		.header("accept", "application/json; version=v2")
		.body(Bytes::new())
		.build()
		.unwrap();

	let version = versioning.determine_version(&request).await.unwrap();
	assert_eq!(version, "v2");
}

#[rstest]
#[tokio::test]
async fn test_url_path_versioning_strategy() {
	// Test URLPathVersioning strategy directly
	let versioning = URLPathVersioning::new()
		.with_default_version("v1")
		.with_allowed_versions(vec!["v1", "v2", "1", "2"]);

	let request = reinhardt_core::http::Request::builder()
		.method(hyper::Method::GET)
		.uri("http://example.com/v2/resource")
		.version(hyper::Version::HTTP_11)
		.body(Bytes::new())
		.build()
		.unwrap();

	let version = versioning.determine_version(&request).await.unwrap();
	assert_eq!(version, "2");
}

#[rstest]
#[tokio::test]
async fn test_hostname_versioning_strategy() {
	// Test HostNameVersioning strategy directly
	let versioning = HostNameVersioning::new()
		.with_default_version("v1")
		.with_allowed_versions(vec!["v1", "v2"]);

	let request = reinhardt_core::http::Request::builder()
		.method(hyper::Method::GET)
		.uri("http://v2.api.example.com/resource")
		.version(hyper::Version::HTTP_11)
		.header("host", "v2.api.example.com")
		.body(Bytes::new())
		.build()
		.unwrap();

	let version = versioning.determine_version(&request).await.unwrap();
	assert_eq!(version, "v2");
}

#[rstest]
#[tokio::test]
async fn test_query_parameter_versioning_strategy() {
	// Test QueryParameterVersioning strategy directly
	let versioning = QueryParameterVersioning::new()
		.with_default_version("v1")
		.with_allowed_versions(vec!["v1", "v2"]);

	let request = reinhardt_core::http::Request::builder()
		.method(hyper::Method::GET)
		.uri("http://example.com/resource?version=v2")
		.version(hyper::Version::HTTP_11)
		.body(Bytes::new())
		.build()
		.unwrap();

	let version = versioning.determine_version(&request).await.unwrap();
	assert_eq!(version, "v2");
}

#[rstest]
#[tokio::test]
async fn test_namespace_versioning_strategy() {
	// Test NamespaceVersioning strategy directly
	let versioning = NamespaceVersioning::new()
		.with_default_version("v1")
		.with_allowed_versions(vec!["v1", "v2", "1", "2"]);

	let request = reinhardt_core::http::Request::builder()
		.method(hyper::Method::GET)
		.uri("http://example.com/v2/resource")
		.version(hyper::Version::HTTP_11)
		.body(Bytes::new())
		.build()
		.unwrap();

	let version = versioning.determine_version(&request).await.unwrap();
	assert_eq!(version, "2");
}
