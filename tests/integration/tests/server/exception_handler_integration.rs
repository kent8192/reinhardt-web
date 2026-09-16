//! Exception handler installation end-to-end tests (Issue #6294).
//!
//! Exercises the reporter's scenario over a real HTTP connection: a router with
//! an installed [`ExceptionHandler`] must answer errors with the application's
//! fixed error shape rather than the framework default.

use async_trait::async_trait;
// Imported through the facade prelude, mirroring what a generated project's
// `src/config/urls.rs` sees with `use reinhardt::prelude::*`.
use reinhardt::prelude::ExceptionHandler;
use reinhardt_core::exception::Error;
use reinhardt_http::{Handler, Middleware, Request, Response};
use reinhardt_urls::routers::ServerRouter;
use rstest::rstest;
use std::sync::Arc;
use tokio::task::JoinHandle;

use super::server_test_helpers::{shutdown_test_server, spawn_test_server};

struct ServerGuard(Option<JoinHandle<()>>);

impl ServerGuard {
	fn new(handle: JoinHandle<()>) -> Self {
		Self(Some(handle))
	}

	async fn shutdown(mut self) {
		if let Some(handle) = self.0.take() {
			shutdown_test_server(handle).await;
		}
	}
}

impl Drop for ServerGuard {
	fn drop(&mut self) {
		// Abort the accept loop if an assertion or request operation unwinds
		// before the test reaches its explicit asynchronous shutdown.
		if let Some(handle) = self.0.take() {
			handle.abort();
		}
	}
}

/// Reproduces an application whose clients parse a fixed error body.
struct ApiErrors;

#[async_trait]
impl ExceptionHandler for ApiErrors {
	async fn handle_exception(&self, _request: &Request, error: Error) -> Response {
		let status = hyper::StatusCode::from_u16(error.status_code())
			.unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);
		Response::new(status).with_body(
			serde_json::json!({
				"errNo": status.as_u16(),
				"errMsg": "The requested resource is not available.",
			})
			.to_string(),
		)
	}
}

/// Middleware that fails before reaching the next handler.
struct RejectingMiddleware;

#[async_trait]
impl Middleware for RejectingMiddleware {
	async fn process(
		&self,
		_request: Request,
		_next: Arc<dyn Handler>,
	) -> reinhardt_core::exception::Result<Response> {
		// `Error::Authorization` maps to 403 and stands in for the kind of
		// middleware rejection (CSRF, permission check) that must also reach the
		// application's handler.
		Err(Error::Authorization(
			"token rejected by middleware".to_string(),
		))
	}
}

#[rstest]
#[tokio::test]
async fn test_router_404_uses_installed_handler_over_http() {
	// Arrange: the reporter's installation, on a real listener
	let router = ServerRouter::new().with_exception_handler(Arc::new(ApiErrors));
	let (url, handle) = spawn_test_server(Arc::new(router)).await;
	let server = ServerGuard::new(handle);

	// Act
	let response = reqwest::get(format!("{}/missing", url)).await.unwrap();

	// Assert: the framework default body is replaced by the application shape
	assert_eq!(response.status().as_u16(), 404);
	let body: serde_json::Value = response.json().await.unwrap();
	assert_eq!(body["errNo"], 404);
	assert_eq!(body["errMsg"], "The requested resource is not available.");

	// Cleanup
	server.shutdown().await;
}

#[rstest]
#[tokio::test]
async fn test_middleware_error_uses_installed_handler_over_http() {
	// Arrange: a failing middleware on the router, so the error never reaches a
	// view and can only be answered by the router's installed handler
	let router = ServerRouter::new()
		.with_exception_handler(Arc::new(ApiErrors))
		.with_middleware(RejectingMiddleware);
	let (url, handle) = spawn_test_server(Arc::new(router)).await;
	let server = ServerGuard::new(handle);

	// Act
	let response = reqwest::get(format!("{}/anything", url)).await.unwrap();

	// Assert
	assert_eq!(response.status().as_u16(), 403);
	let body: serde_json::Value = response.json().await.unwrap();
	assert_eq!(body["errNo"], 403);
	assert_eq!(body["errMsg"], "The requested resource is not available.");

	// Cleanup
	server.shutdown().await;
}
