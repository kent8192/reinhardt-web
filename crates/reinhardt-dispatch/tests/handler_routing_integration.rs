//! Integration tests for BaseHandler + URL Resolver
//!
//! Tests the integration between BaseHandler and DefaultRouter:
//! - Route resolution and view execution
//! - Error handling and HTTP status codes
//! - Request/response flow

use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, StatusCode};
use reinhardt_core::Handler;
use reinhardt_core::http::{Request, Response};
use reinhardt_dispatch::handler::BaseHandler;
use reinhardt_urls::prelude::Router;
use reinhardt_urls::routers::{DefaultRouter, Route};
use std::sync::Arc;

/// Simple test handler that returns OK
struct TestHandlerOk;

#[async_trait]
impl Handler for TestHandlerOk {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		Ok(Response::new(StatusCode::OK).with_body(Bytes::from("OK")))
	}
}

/// Handler that simulates an error
struct TestHandlerError;

#[async_trait]
impl Handler for TestHandlerError {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		Err(reinhardt_core::exception::Error::Internal(
			"Simulated error".to_string(),
		))
	}
}

/// Handler that returns JSON response
struct TestHandlerJson;

#[async_trait]
impl Handler for TestHandlerJson {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		let json = serde_json::json!({
			"status": "success",
			"data": {"id": 1, "name": "test"}
		});
		Ok(Response::new(StatusCode::OK)
			.with_header("Content-Type", "application/json")
			.with_body(Bytes::from(json.to_string())))
	}
}

#[tokio::test]
async fn test_handler_routes_to_correct_view() {
	// Setup router with test routes
	let mut router = DefaultRouter::new();
	let route = Route::from_handler("/test", Arc::new(TestHandlerOk));
	router.add_route(route);

	// Create handler with router
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	// Execute handler
	let response = handler.handle(request).await;

	// Verify response
	assert!(response.is_ok());
	let response = response.unwrap();
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(&response.body, &Bytes::from("OK"));
}

#[tokio::test]
async fn test_handler_returns_404_for_unknown_route() {
	// Setup router with no routes
	let router = DefaultRouter::new();

	// Create handler with empty router
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create request for non-existent route
	let request = Request::builder()
		.method(Method::GET)
		.uri("/nonexistent")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	// Execute handler
	let response = handler.handle(request).await;

	// Verify 404 response
	assert!(response.is_ok());
	let response = response.unwrap();
	assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_handler_returns_500_for_view_error() {
	// Setup router with error-producing handler
	let mut router = DefaultRouter::new();
	let route = Route::from_handler("/error", Arc::new(TestHandlerError));
	router.add_route(route);

	// Create handler with router
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/error")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	// Execute handler
	let response = handler.handle(request).await;

	// Verify 500 response (error handled by BaseHandler)
	// BaseHandler catches errors and returns 500
	assert!(response.is_ok());
	let response = response.unwrap();
	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_handler_routes_multiple_paths() {
	// Setup router with multiple routes
	let mut router = DefaultRouter::new();
	let route_ok = Route::from_handler("/test", Arc::new(TestHandlerOk));
	let route_json = Route::from_handler("/api/data", Arc::new(TestHandlerJson));
	router.add_route(route_ok);
	router.add_route(route_json);

	// Create handler with router
	let handler = BaseHandler::with_router(Arc::new(router));

	// Test first route
	let request1 = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");
	let response1 = handler.handle(request1).await.unwrap();
	assert_eq!(response1.status, StatusCode::OK);
	assert_eq!(&response1.body, &Bytes::from("OK"));

	// Test second route
	let request2 = Request::builder()
		.method(Method::GET)
		.uri("/api/data")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");
	let response2 = handler.handle(request2).await.unwrap();
	assert_eq!(response2.status, StatusCode::OK);
	assert!(
		response2
			.headers
			.get("Content-Type")
			.unwrap()
			.to_str()
			.unwrap()
			.contains("application/json")
	);
}
