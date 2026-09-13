//! Base HTTP request handler
//!
//! This module provides the base handler for processing HTTP requests,
//! similar to Django's `django.core.handlers.base.BaseHandler`.

use hyper::StatusCode;
use reinhardt_core::signals::{
	RequestFinishedEvent, RequestStartedEvent, request_finished, request_started,
};
use reinhardt_http::Handler;
use reinhardt_http::{Request, Response};
use reinhardt_urls::routers::DefaultRouter;
use std::sync::Arc;
use tracing::{debug, error, trace, warn};

use crate::{DispatchError, exception::exception_to_dispatch_error};

/// Base HTTP request handler
///
/// Handles the complete request lifecycle including URL resolution,
/// view execution, and signal emission.
pub struct BaseHandler {
	/// Whether the handler operates in async mode.
	///
	/// This flag mirrors Django's `BaseHandler._is_async` and is read by
	/// `Dispatcher` to choose between sync and async code paths. When
	/// `false`, async dispatch still works but callers may opt for a
	/// blocking wrapper.
	// Allow dead_code: read via is_async() accessor; behavioral branching planned
	#[allow(dead_code)]
	is_async: bool,
	router: Option<Arc<DefaultRouter>>,
}

impl BaseHandler {
	/// Create a new base handler
	pub fn new() -> Self {
		Self {
			is_async: true,
			router: None,
		}
	}

	/// Create a handler with a router
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_dispatch::BaseHandler;
	/// use reinhardt_urls::routers::DefaultRouter;
	/// use std::sync::Arc;
	///
	/// let router = DefaultRouter::new();
	/// let handler = BaseHandler::with_router(Arc::new(router));
	/// assert!(handler.is_async());
	/// ```
	pub fn with_router(router: Arc<DefaultRouter>) -> Self {
		Self {
			is_async: true,
			router: Some(router),
		}
	}

	/// Handle an HTTP request
	///
	/// This is the main entry point for request processing. It:
	/// 1. Emits `request_started` signal
	/// 2. Resolves URL and dispatches to view
	/// 3. Emits `request_finished` signal
	/// 4. Converts an unmatched route into a 404 response
	pub async fn handle_request(
		&self,
		request: Request,
	) -> std::result::Result<Response, DispatchError> {
		match self.handle_request_with_errors(request).await {
			Err(DispatchError::UrlResolution(_)) => Ok(Response::new(StatusCode::NOT_FOUND)),
			response => response,
		}
	}

	/// Handle a request while preserving routing and view failures for an outer
	/// exception handler.
	async fn handle_request_with_errors(
		&self,
		request: Request,
	) -> std::result::Result<Response, DispatchError> {
		self.handle_request_with_framework_errors(request)
			.await
			.map_err(exception_to_dispatch_error)
	}

	/// Handle a request while preserving the original framework error variants.
	///
	/// The HTTP exception-handler adapter uses this path so an endpoint's
	/// authentication, authorization, validation, or conflict error reaches the
	/// application handler without being reclassified as a generic view error.
	async fn handle_request_with_framework_errors(
		&self,
		request: Request,
	) -> reinhardt_core::exception::Result<Response> {
		trace!("Handling request: {:?}", request.uri);

		// Emit request_started signal
		let event = RequestStartedEvent::new();
		if let Err(e) = request_started().send(event).await {
			warn!("Failed to send request_started signal: {}", e);
		}

		// Get response with router
		let response = Self::get_response_async(request, self.router.as_ref()).await;

		// Emit request_finished signal
		let event = RequestFinishedEvent::new();
		if let Err(e) = request_finished().send(event).await {
			warn!("Failed to send request_finished signal: {}", e);
		}

		response
	}

	/// Get response for a request (async version) with URL resolution
	///
	/// This is the core request processing logic that:
	/// - Resolves the URL using the router
	/// - Dispatches to the matched handler
	/// - Returns a routing error if no route matches
	/// - Returns an error for handler failures
	async fn get_response_async(
		request: Request,
		router: Option<&Arc<DefaultRouter>>,
	) -> reinhardt_core::exception::Result<Response> {
		debug!("Getting response for: {}", request.uri.path());

		// URL resolution with router
		if let Some(router) = router {
			trace!("Attempting to route request through router");

			// Use the router to handle the request
			match router.handle(request).await {
				Ok(response) => {
					trace!("Route handled successfully");
					return Ok(response);
				}
				Err(reinhardt_core::exception::Error::NotFound(msg)) => {
					debug!("No route matched: {}", msg);
					return Err(reinhardt_core::exception::Error::NotFound(msg));
				}
				Err(e) => {
					error!("Handler error: {}", e);
					// Return the original error so an installed exception handler can
					// preserve its status and variant.
					return Err(e);
				}
			}
		}

		// Fallback: router not configured, so no routes can match.
		debug!("No router configured, returning a URL resolution error");
		Err(reinhardt_core::exception::Error::NotFound(
			"No router configured".to_owned(),
		))
	}

	/// Process an exception and convert it to a response.
	///
	/// Error details are logged server-side but not included in the response
	/// body to prevent information disclosure.
	pub async fn handle_exception(&self, _request: &Request, error: DispatchError) -> Response {
		error!("Handling exception: {}", error);

		crate::build_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
	}

	/// Check if handler is configured for async mode
	pub fn is_async(&self) -> bool {
		self.is_async
	}

	/// Set async mode for the handler
	pub fn set_async(&mut self, is_async: bool) {
		self.is_async = is_async;
	}
}

impl Default for BaseHandler {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl Handler for BaseHandler {
	async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
		let has_exception_handler = request
			.extensions
			.contains::<Arc<dyn reinhardt_http::ExceptionHandler>>();
		if has_exception_handler {
			return self.handle_request_with_framework_errors(request).await;
		}
		match self.handle_request_with_errors(request).await {
			Ok(response) => Ok(response),
			Err(DispatchError::UrlResolution(_)) => Ok(Response::new(StatusCode::NOT_FOUND)),
			Err(e) => {
				// Log the detailed error server-side; return generic message to client
				error!("Handler error in BaseHandler::handle: {}", e);
				Ok(crate::build_error_response(
					StatusCode::INTERNAL_SERVER_ERROR,
					"Internal Server Error",
				))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use async_trait::async_trait;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use reinhardt_http::{ExceptionHandler, ExceptionHandlingHandler};
	use reinhardt_urls::routers::{DefaultRouter, Router, path};
	use rstest::rstest;

	// Test handler for routing tests
	struct TestHandler {
		response_body: String,
	}

	#[async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
			Ok(Response::ok().with_body(self.response_body.clone()))
		}
	}

	#[tokio::test]
	async fn test_base_handler_new() {
		let handler = BaseHandler::new();
		assert!(handler.is_async());
	}

	#[tokio::test]
	async fn test_base_handler_handle_request() {
		let handler = BaseHandler::new();
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle_request(request).await;
		let resp = response.unwrap();
		// Handler without router should return 404 Not Found
		assert_eq!(resp.status, StatusCode::NOT_FOUND);
	}

	#[tokio::test]
	async fn test_base_handler_handle_exception() {
		let handler = BaseHandler::new();
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let error = DispatchError::View("Test error".to_string());

		let response = handler.handle_exception(&request, error).await;
		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
	}

	// ==========================================================================
	// Information Disclosure Prevention Tests (#439)
	// ==========================================================================

	#[tokio::test]
	async fn test_handle_exception_does_not_expose_internal_details() {
		// Arrange
		let handler = BaseHandler::new();
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let sensitive_detail = "database connection refused at postgres://admin:secret@db:5432";
		let error = DispatchError::Internal(sensitive_detail.to_string());

		// Act
		let response = handler.handle_exception(&request, error).await;

		// Assert: response must not contain the sensitive detail
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
		assert!(!body.contains("database"));
		assert!(!body.contains("postgres"));
		assert!(!body.contains("secret"));
		assert_eq!(body, "Internal Server Error");
	}

	#[tokio::test]
	async fn test_handler_impl_does_not_expose_error_in_body() {
		// Arrange: create a handler that returns a view error with internal paths
		struct FailingHandler;

		#[async_trait]
		impl Handler for FailingHandler {
			async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
				Err(reinhardt_core::exception::Error::Internal(
					"module::secret_handler panicked at /src/app/handlers.rs:42".to_string(),
				))
			}
		}

		let mut router = DefaultRouter::new();
		let failing = Arc::new(FailingHandler);
		let mut route = path("/fail", failing);
		route.name = Some("fail".to_string());
		router.add_route(route);
		let handler = BaseHandler::with_router(Arc::new(router));

		let request = Request::builder()
			.method(Method::GET)
			.uri("/fail")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = handler.handle(request).await.unwrap();

		// Assert: internal details must not leak
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
		assert!(!body.contains("panicked"));
		assert!(!body.contains("handlers.rs"));
		assert!(!body.contains("secret_handler"));
		assert_eq!(body, "Internal Server Error");
	}

	struct TeapotExceptionHandler;

	#[async_trait]
	impl ExceptionHandler for TeapotExceptionHandler {
		async fn handle_exception(
			&self,
			_request: &Request,
			_error: reinhardt_core::exception::Error,
		) -> Response {
			Response::new(StatusCode::IM_A_TEAPOT).with_body("teapot")
		}
	}

	struct StatusExceptionHandler;

	#[async_trait]
	impl ExceptionHandler for StatusExceptionHandler {
		async fn handle_exception(
			&self,
			_request: &Request,
			error: reinhardt_core::exception::Error,
		) -> Response {
			let status = StatusCode::from_u16(error.status_code())
				.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
			Response::new(status)
		}
	}

	#[rstest]
	#[tokio::test]
	async fn base_handler_routing_errors_reach_http_exception_handler() {
		// Arrange
		let base = Arc::new(BaseHandler::with_router(Arc::new(DefaultRouter::new())));
		let handler = ExceptionHandlingHandler::new(base, Arc::new(TeapotExceptionHandler));
		let request = Request::builder()
			.method(Method::GET)
			.uri("/missing")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = handler.handle(request).await.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
		assert_eq!(response.body, Bytes::from_static(b"teapot"));
	}

	#[rstest]
	#[tokio::test]
	async fn base_handler_view_errors_reach_http_exception_handler() {
		struct FailingHandler;

		#[async_trait]
		impl Handler for FailingHandler {
			async fn handle(
				&self,
				_request: Request,
			) -> reinhardt_core::exception::Result<Response> {
				Err(reinhardt_core::exception::Error::Internal(
					"view failed".to_owned(),
				))
			}
		}

		// Arrange
		let mut router = DefaultRouter::new();
		router.add_route(path("/fail", Arc::new(FailingHandler)));
		let base = Arc::new(BaseHandler::with_router(Arc::new(router)));
		let handler = ExceptionHandlingHandler::new(base, Arc::new(TeapotExceptionHandler));
		let request = Request::builder()
			.method(Method::GET)
			.uri("/fail")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = handler.handle(request).await.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
		assert_eq!(response.body, Bytes::from_static(b"teapot"));
	}

	#[rstest]
	#[tokio::test]
	async fn base_handler_preserves_endpoint_error_status_for_http_exception_handler() {
		struct AuthenticationHandler;

		#[async_trait]
		impl Handler for AuthenticationHandler {
			async fn handle(
				&self,
				_request: Request,
			) -> reinhardt_core::exception::Result<Response> {
				Err(reinhardt_core::exception::Error::Authentication(
					"credentials rejected".to_owned(),
				))
			}
		}

		// Arrange
		let mut router = DefaultRouter::new();
		router.add_route(path("/private", Arc::new(AuthenticationHandler)));
		let base = Arc::new(BaseHandler::with_router(Arc::new(router)));
		let handler = ExceptionHandlingHandler::new(base, Arc::new(StatusExceptionHandler));
		let request = Request::builder()
			.method(Method::GET)
			.uri("/private")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = handler.handle(request).await.unwrap();

		// Assert: the endpoint's authentication error is not reclassified as 500.
		assert_eq!(response.status, StatusCode::UNAUTHORIZED);
	}

	#[test]
	fn test_base_handler_async_mode() {
		let mut handler = BaseHandler::new();
		assert!(handler.is_async());

		handler.set_async(false);
		assert!(!handler.is_async());
	}

	#[tokio::test]
	async fn test_base_handler_different_methods() {
		let handler = BaseHandler::new();

		for method in [Method::GET, Method::POST, Method::PUT, Method::DELETE] {
			let request = Request::builder()
				.method(method)
				.uri("/")
				.version(Version::HTTP_11)
				.headers(HeaderMap::new())
				.body(Bytes::new())
				.build()
				.unwrap();

			let response = handler.handle_request(request).await;
			assert!(response.is_ok());
		}
	}

	#[tokio::test]
	async fn test_base_handler_different_uris() {
		let handler = BaseHandler::new();

		for path in ["/", "/test", "/api/v1/users", "/admin/login"] {
			let request = Request::builder()
				.method(Method::GET)
				.uri(path)
				.version(Version::HTTP_11)
				.headers(HeaderMap::new())
				.body(Bytes::new())
				.build()
				.unwrap();

			let response = handler.handle_request(request).await;
			assert!(response.is_ok());
		}
	}

	#[tokio::test]
	async fn test_handler_with_router() {
		// Create a router with a test route
		let mut router = DefaultRouter::new();
		let test_handler = Arc::new(TestHandler {
			response_body: "Test response".to_string(),
		});
		let mut route = path("/test", test_handler);
		route.name = Some("test".to_string());
		router.add_route(route);

		// Create BaseHandler with router
		let handler = BaseHandler::with_router(Arc::new(router));

		// Test matching route
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle_request(request).await;
		let resp = response.unwrap();
		assert_eq!(resp.status, StatusCode::OK);

		let body = String::from_utf8(resp.body.to_vec()).unwrap();
		assert_eq!(body, "Test response");
	}

	#[tokio::test]
	async fn test_handler_404_not_found() {
		// Create empty router
		let router = DefaultRouter::new();

		// Create BaseHandler with router
		let handler = BaseHandler::with_router(Arc::new(router));

		// Test non-existent route
		let request = Request::builder()
			.method(Method::GET)
			.uri("/nonexistent")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle_request(request).await;
		let resp = response.unwrap();
		assert_eq!(resp.status, StatusCode::NOT_FOUND);
	}

	#[tokio::test]
	async fn test_handler_multiple_routes() {
		// Create router with multiple routes
		let mut router = DefaultRouter::new();

		let hello_handler = Arc::new(TestHandler {
			response_body: "Hello".to_string(),
		});
		let mut hello_route = path("/hello", hello_handler);
		hello_route.name = Some("hello".to_string());
		router.add_route(hello_route);

		let world_handler = Arc::new(TestHandler {
			response_body: "World".to_string(),
		});
		let mut world_route = path("/world", world_handler);
		world_route.name = Some("world".to_string());
		router.add_route(world_route);

		let handler = BaseHandler::with_router(Arc::new(router));

		// Test first route
		let request = Request::builder()
			.method(Method::GET)
			.uri("/hello")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = handler.handle_request(request).await.unwrap();
		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(String::from_utf8(response.body.to_vec()).unwrap(), "Hello");

		// Test second route
		let request = Request::builder()
			.method(Method::GET)
			.uri("/world")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = handler.handle_request(request).await.unwrap();
		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(String::from_utf8(response.body.to_vec()).unwrap(), "World");
	}
}
