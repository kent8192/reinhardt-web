use async_trait::async_trait;
use chrono::Utc;
use reinhardt_core::apps::{Handler, Middleware, Request, Response, Result};
use std::sync::Arc;

/// Logging middleware
/// Logs request/response information
pub struct LoggingMiddleware;

impl LoggingMiddleware {
	/// Create a new logging middleware
	///
	/// This middleware logs each request with its method, path, status code, and duration.
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_middleware::LoggingMiddleware;
	/// use reinhardt_core::apps::{Handler, Middleware, Request, Response};
	/// use hyper::{Method, Uri, Version, HeaderMap, StatusCode};
	/// use bytes::Bytes;
	///
	/// struct TestHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for TestHandler {
	///     async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::new(StatusCode::OK).with_body(Bytes::from("OK")))
	///     }
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let middleware = LoggingMiddleware::new();
	/// let handler = Arc::new(TestHandler);
	/// let request = Request::new(
	///     Method::GET,
	///     Uri::from_static("/api/users"),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new(),
	/// );
	///
	/// let response = middleware.process(request, handler).await.unwrap();
	/// assert_eq!(response.status, StatusCode::OK);
	// Logs: [2024-01-01 12:00:00] GET /api/users - 200 (5 ms)
	/// # });
	/// ```
	pub fn new() -> Self {
		Self
	}
}

impl Default for LoggingMiddleware {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Middleware for LoggingMiddleware {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		let start = Utc::now();
		let method = request.method.to_string();
		let path = request.path().to_string();

		// Process request
		let result = next.handle(request).await;

		let duration = Utc::now().signed_duration_since(start);

		match &result {
			Ok(response) => {
				println!(
					"[{}] {} {} - {} ({} ms)",
					start.format("%Y-%m-%d %H:%M:%S"),
					method,
					path,
					response.status.as_u16(),
					duration.num_milliseconds()
				);
			}
			Err(err) => {
				eprintln!(
					"[{}] {} {} - ERROR: {} ({} ms)",
					start.format("%Y-%m-%d %H:%M:%S"),
					method,
					path,
					err,
					duration.num_milliseconds()
				);
			}
		}

		result
	}
}
