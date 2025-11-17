//! Server Middleware Integration Tests
//!
//! Integration tests for reinhardt-server working with reinhardt-middleware.
//! These tests verify that the server properly integrates with middleware
//! chain functionality.

use reinhardt_middleware::{LoggingMiddleware, MiddlewareChain};
use reinhardt_types::Middleware;
use std::sync::Arc;

// Test helper modules - these need to be accessible from the server crate
// For now, we'll inline a simple test handler

use async_trait::async_trait;
use reinhardt_http::{Request, Response};
use reinhardt_types::Handler;

/// Simple test handler for middleware chain tests
struct TestHandler {
	response_body: String,
}

impl TestHandler {
	fn new(body: &str) -> Self {
		Self {
			response_body: body.to_string(),
		}
	}
}

#[async_trait]
impl Handler for TestHandler {
	async fn handle(&self, _request: Request) -> reinhardt_exception::Result<Response> {
		use hyper::StatusCode;
		let mut response = Response::new(StatusCode::OK);
		response.body = self.response_body.clone().into();
		Ok(response)
	}
}

#[tokio::test]
async fn test_middleware_integration_chain() {
	// Create a simple handler
	let base_handler = Arc::new(TestHandler::new("Response through middleware"));

	// Build middleware chain with logging middleware
	let chain = MiddlewareChain::new(base_handler)
		.with_middleware(Arc::new(LoggingMiddleware::new()) as Arc<dyn Middleware>);

	// In a full integration test, we would spawn a server here
	// For now, we can test that the chain is constructed correctly

	// Create a test request
	use hyper::{HeaderMap, Method, Uri, Version};
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(bytes::Bytes::new())
		.build()
		.unwrap();

	// Execute through the chain
	let response = chain.handle(request).await;

	let response = response.unwrap();
	assert_eq!(response.status, hyper::StatusCode::OK);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	assert_eq!(body, "Response through middleware");
}

#[tokio::test]
async fn test_server_middleware_integration_multiple() {
	// Test that multiple middlewares can be chained
	let base_handler = Arc::new(TestHandler::new("Multi-middleware response"));

	let chain = MiddlewareChain::new(base_handler)
		.with_middleware(Arc::new(LoggingMiddleware::new()) as Arc<dyn Middleware>)
		.with_middleware(Arc::new(LoggingMiddleware::new()) as Arc<dyn Middleware>);

	use hyper::{HeaderMap, Method, Uri, Version};
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(bytes::Bytes::new())
		.build()
		.unwrap();

	let response = chain.handle(request).await;

	let response = response.unwrap();
	assert_eq!(response.status, hyper::StatusCode::OK);
}
