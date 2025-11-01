use reinhardt_http::{Request, Response};
use reinhardt_types::Handler;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Middleware that adds timeout to request handling
///
/// Wraps a handler and enforces a maximum execution time for requests.
/// If the handler takes longer than the specified duration, a 408 Request Timeout
/// response is returned.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
/// use reinhardt_server_core::TimeoutHandler;
/// use reinhardt_types::Handler;
/// use reinhardt_http::{Request, Response};
///
/// struct MyHandler;
///
/// #[async_trait::async_trait]
/// impl Handler for MyHandler {
///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
///         Ok(Response::ok().with_body("Hello"))
///     }
/// }
///
/// let handler = Arc::new(MyHandler);
/// let timeout_handler = TimeoutHandler::new(handler, Duration::from_secs(30));
/// ```
pub struct TimeoutHandler {
	inner: Arc<dyn Handler>,
	timeout_duration: Duration,
}

impl TimeoutHandler {
	/// Create a new timeout handler
	///
	/// # Arguments
	///
	/// * `inner` - The inner handler to wrap
	/// * `timeout_duration` - Maximum time to wait for the handler to complete
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use std::time::Duration;
	/// use reinhardt_server_core::TimeoutHandler;
	/// use reinhardt_types::Handler;
	/// use reinhardt_http::{Request, Response};
	///
	/// struct MyHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for MyHandler {
	///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// let handler = Arc::new(MyHandler);
	/// let timeout_handler = TimeoutHandler::new(handler, Duration::from_secs(30));
	/// ```
	pub fn new(inner: Arc<dyn Handler>, timeout_duration: Duration) -> Self {
		Self {
			inner,
			timeout_duration,
		}
	}

	/// Get the timeout duration
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use std::time::Duration;
	/// use reinhardt_server_core::TimeoutHandler;
	/// use reinhardt_types::Handler;
	/// use reinhardt_http::{Request, Response};
	///
	/// struct MyHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for MyHandler {
	///     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// let handler = Arc::new(MyHandler);
	/// let timeout_handler = TimeoutHandler::new(handler, Duration::from_secs(30));
	/// assert_eq!(timeout_handler.timeout_duration(), Duration::from_secs(30));
	/// ```
	pub fn timeout_duration(&self) -> Duration {
		self.timeout_duration
	}
}

#[async_trait::async_trait]
impl Handler for TimeoutHandler {
	async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
		match timeout(self.timeout_duration, self.inner.handle(request)).await {
			Ok(result) => result,
			Err(_) => {
				// Timeout occurred
				Ok(Response::new(http::StatusCode::REQUEST_TIMEOUT).with_body("Request Timeout"))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	struct FastHandler;

	#[async_trait::async_trait]
	impl Handler for FastHandler {
		async fn handle(&self, _request: Request) -> reinhardt_exception::Result<Response> {
			Ok(Response::ok().with_body("Fast response"))
		}
	}

	struct SlowHandler {
		delay: Duration,
	}

	#[async_trait::async_trait]
	impl Handler for SlowHandler {
		async fn handle(&self, _request: Request) -> reinhardt_exception::Result<Response> {
			tokio::time::sleep(self.delay).await;
			Ok(Response::ok().with_body("Slow response"))
		}
	}

	#[tokio::test]
	async fn test_timeout_handler_creation() {
		let handler = Arc::new(FastHandler);
		let timeout_handler = TimeoutHandler::new(handler, Duration::from_secs(5));
		assert_eq!(timeout_handler.timeout_duration(), Duration::from_secs(5));
	}

	#[tokio::test]
	async fn test_fast_request_completes() {
		let handler = Arc::new(FastHandler);
		let timeout_handler = TimeoutHandler::new(handler, Duration::from_secs(1));

		let request = Request::new(
			http::Method::GET,
			"/".parse().unwrap(),
			http::Version::HTTP_11,
			http::HeaderMap::new(),
			bytes::Bytes::new(),
		);

		let response = timeout_handler.handle(request).await.unwrap();
		assert_eq!(response.status, http::StatusCode::OK);
	}

	#[tokio::test]
	async fn test_slow_request_times_out() {
		let handler = Arc::new(SlowHandler {
			delay: Duration::from_secs(2),
		});
		let timeout_handler = TimeoutHandler::new(handler, Duration::from_millis(100));

		let request = Request::new(
			http::Method::GET,
			"/".parse().unwrap(),
			http::Version::HTTP_11,
			http::HeaderMap::new(),
			bytes::Bytes::new(),
		);

		let response = timeout_handler.handle(request).await.unwrap();
		assert_eq!(response.status, http::StatusCode::REQUEST_TIMEOUT);
	}

	#[tokio::test]
	async fn test_request_just_within_timeout() {
		let handler = Arc::new(SlowHandler {
			delay: Duration::from_millis(50),
		});
		let timeout_handler = TimeoutHandler::new(handler, Duration::from_millis(100));

		let request = Request::new(
			http::Method::GET,
			"/".parse().unwrap(),
			http::Version::HTTP_11,
			http::HeaderMap::new(),
			bytes::Bytes::new(),
		);

		let response = timeout_handler.handle(request).await.unwrap();
		assert_eq!(response.status, http::StatusCode::OK);
	}
}
