use reinhardt_core::Handler;
use reinhardt_core::http::{Request, Response};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiting strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitStrategy {
	/// Fixed window rate limiting
	FixedWindow,
	/// Sliding window rate limiting
	SlidingWindow,
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
	/// Maximum requests allowed in the window
	pub max_requests: usize,
	/// Time window duration
	pub window_duration: Duration,
	/// Rate limiting strategy
	pub strategy: RateLimitStrategy,
}

impl RateLimitConfig {
	/// Create a new rate limit configuration
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_server_core::{RateLimitConfig, RateLimitStrategy};
	///
	/// let config = RateLimitConfig::new(
	///     100,
	///     Duration::from_secs(60),
	///     RateLimitStrategy::FixedWindow,
	/// );
	/// ```
	pub fn new(
		max_requests: usize,
		window_duration: Duration,
		strategy: RateLimitStrategy,
	) -> Self {
		Self {
			max_requests,
			window_duration,
			strategy,
		}
	}

	/// Create a per-minute rate limit
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_server_core::RateLimitConfig;
	///
	/// let config = RateLimitConfig::per_minute(60);
	/// ```
	pub fn per_minute(max_requests: usize) -> Self {
		Self::new(
			max_requests,
			Duration::from_secs(60),
			RateLimitStrategy::FixedWindow,
		)
	}

	/// Create a per-hour rate limit
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_server_core::RateLimitConfig;
	///
	/// let config = RateLimitConfig::per_hour(1000);
	/// ```
	pub fn per_hour(max_requests: usize) -> Self {
		Self::new(
			max_requests,
			Duration::from_secs(3600),
			RateLimitStrategy::FixedWindow,
		)
	}
}

/// Rate limit entry for tracking requests
#[derive(Debug, Clone)]
struct RateLimitEntry {
	count: usize,
	window_start: Instant,
}

/// Middleware that implements rate limiting
///
/// Tracks requests by client IP address and enforces rate limits.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
/// use reinhardt_server_core::{RateLimitHandler, RateLimitConfig};
/// use reinhardt_core::Handler;
/// use reinhardt_core::http::{Request, Response};
///
/// struct MyHandler;
///
/// #[async_trait::async_trait]
/// impl Handler for MyHandler {
///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
///         Ok(Response::ok())
///     }
/// }
///
/// let handler = Arc::new(MyHandler);
/// let config = RateLimitConfig::per_minute(60);
/// let rate_limit_handler = RateLimitHandler::new(handler, config);
/// ```
pub struct RateLimitHandler {
	inner: Arc<dyn Handler>,
	config: RateLimitConfig,
	limits: Arc<RwLock<HashMap<IpAddr, RateLimitEntry>>>,
}

impl RateLimitHandler {
	/// Create a new rate limit handler
	///
	/// # Arguments
	///
	/// * `inner` - The inner handler to wrap
	/// * `config` - Rate limit configuration
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_server_core::{RateLimitHandler, RateLimitConfig};
	/// use reinhardt_core::Handler;
	/// use reinhardt_core::http::{Request, Response};
	///
	/// struct MyHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for MyHandler {
	///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// let handler = Arc::new(MyHandler);
	/// let config = RateLimitConfig::per_minute(100);
	/// let rate_limit_handler = RateLimitHandler::new(handler, config);
	/// ```
	pub fn new(inner: Arc<dyn Handler>, config: RateLimitConfig) -> Self {
		Self {
			inner,
			config,
			limits: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Check if a request is allowed for the given IP
	async fn is_allowed(&self, ip: IpAddr) -> bool {
		let now = Instant::now();
		let mut limits = self.limits.write().await;

		let entry = limits.entry(ip).or_insert(RateLimitEntry {
			count: 0,
			window_start: now,
		});

		// Check if window has expired
		if now.duration_since(entry.window_start) >= self.config.window_duration {
			// Reset window
			entry.count = 0;
			entry.window_start = now;
		}

		// Check if under limit
		if entry.count < self.config.max_requests {
			entry.count += 1;
			true
		} else {
			false
		}
	}

	/// Extract client IP from request
	///
	/// Attempts to extract client IP in the following order:
	/// 1. X-Forwarded-For header (first IP in comma-separated list)
	/// 2. X-Real-IP header
	/// 3. Fallback to localhost (127.0.0.1)
	///
	/// # Note
	///
	/// In production, you should validate that proxy headers come from trusted sources
	/// to prevent IP spoofing attacks.
	fn extract_client_ip(&self, request: &Request) -> IpAddr {
		// 1. Check X-Forwarded-For header
		if let Some(xff) = request.headers.get("X-Forwarded-For")
			&& let Ok(xff_str) = xff.to_str()
		{
			// X-Forwarded-For can contain multiple IPs: "client, proxy1, proxy2"
			// Take the first (leftmost) IP as the original client IP
			if let Some(first_ip) = xff_str.split(',').next()
				&& let Ok(ip) = first_ip.trim().parse()
			{
				return ip;
			}
		}

		// 2. Check X-Real-IP header
		if let Some(xri) = request.headers.get("X-Real-IP")
			&& let Ok(ip_str) = xri.to_str()
			&& let Ok(ip) = ip_str.parse()
		{
			return ip;
		}

		// 3. Fallback to localhost
		"127.0.0.1".parse().unwrap()
	}
}

#[async_trait::async_trait]
impl Handler for RateLimitHandler {
	async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
		let client_ip = self.extract_client_ip(&request);

		if self.is_allowed(client_ip).await {
			self.inner.handle(request).await
		} else {
			Ok(Response::new(http::StatusCode::TOO_MANY_REQUESTS).with_body("Rate limit exceeded"))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	struct TestHandler;

	#[async_trait::async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
			Ok(Response::ok().with_body("Success"))
		}
	}

	#[tokio::test]
	async fn test_rate_limit_config_creation() {
		let config = RateLimitConfig::per_minute(60);
		assert_eq!(config.max_requests, 60);
		assert_eq!(config.window_duration, Duration::from_secs(60));

		let config = RateLimitConfig::per_hour(1000);
		assert_eq!(config.max_requests, 1000);
		assert_eq!(config.window_duration, Duration::from_secs(3600));
	}

	#[tokio::test]
	async fn test_rate_limit_handler_creation() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(10);
		let _rate_limit_handler = RateLimitHandler::new(handler, config);
	}

	#[tokio::test]
	async fn test_requests_within_limit() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(5);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		for _ in 0..5 {
			let request = Request::builder()
				.method(http::Method::GET)
				.uri("/")
				.version(http::Version::HTTP_11)
				.headers(http::HeaderMap::new())
				.body(bytes::Bytes::new())
				.build()
				.unwrap();

			let response = rate_limit_handler.handle(request).await.unwrap();
			assert_eq!(response.status, http::StatusCode::OK);
		}
	}

	#[tokio::test]
	async fn test_requests_exceed_limit() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(3);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		// First 3 requests should succeed
		for _ in 0..3 {
			let request = Request::builder()
				.method(http::Method::GET)
				.uri("/")
				.version(http::Version::HTTP_11)
				.headers(http::HeaderMap::new())
				.body(bytes::Bytes::new())
				.build()
				.unwrap();

			let response = rate_limit_handler.handle(request).await.unwrap();
			assert_eq!(response.status, http::StatusCode::OK);
		}

		// 4th request should be rate limited
		let request = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.version(http::Version::HTTP_11)
			.headers(http::HeaderMap::new())
			.body(bytes::Bytes::new())
			.build()
			.unwrap();

		let response = rate_limit_handler.handle(request).await.unwrap();
		assert_eq!(response.status, http::StatusCode::TOO_MANY_REQUESTS);
	}

	#[tokio::test]
	async fn test_rate_limit_window_reset() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::new(
			2,
			Duration::from_millis(100),
			RateLimitStrategy::FixedWindow,
		);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		// Use up the limit
		for _ in 0..2 {
			let request = Request::builder()
				.method(http::Method::GET)
				.uri("/")
				.version(http::Version::HTTP_11)
				.headers(http::HeaderMap::new())
				.body(bytes::Bytes::new())
				.build()
				.unwrap();
			let response = rate_limit_handler.handle(request).await.unwrap();
			assert_eq!(response.status, http::StatusCode::OK);
		}

		// Poll until rate limit window resets (100ms window duration)
		reinhardt_test::poll_until(
			Duration::from_millis(200),
			Duration::from_millis(10),
			|| async {
				let test_request = Request::builder()
					.method(http::Method::GET)
					.uri("/")
					.version(http::Version::HTTP_11)
					.headers(http::HeaderMap::new())
					.body(bytes::Bytes::new())
					.build()
					.unwrap();
				let test_response = rate_limit_handler.handle(test_request).await.unwrap();
				test_response.status == http::StatusCode::OK
			},
		)
		.await
		.expect("Window should reset within 200ms");
	}

	// Client IP extraction tests

	#[test]
	fn test_extract_client_ip_from_x_forwarded_for() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(10);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		let mut headers = http::HeaderMap::new();
		headers.insert(
			"X-Forwarded-For",
			"192.168.1.100, 10.0.0.1, 172.16.0.1".parse().unwrap(),
		);

		let request = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.version(http::Version::HTTP_11)
			.headers(headers)
			.body(bytes::Bytes::new())
			.build()
			.unwrap();

		let ip = rate_limit_handler.extract_client_ip(&request);
		assert_eq!(ip, "192.168.1.100".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn test_extract_client_ip_from_x_real_ip() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(10);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		let mut headers = http::HeaderMap::new();
		headers.insert("X-Real-IP", "203.0.113.42".parse().unwrap());

		let request = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.version(http::Version::HTTP_11)
			.headers(headers)
			.body(bytes::Bytes::new())
			.build()
			.unwrap();

		let ip = rate_limit_handler.extract_client_ip(&request);
		assert_eq!(ip, "203.0.113.42".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn test_extract_client_ip_prefers_x_forwarded_for() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(10);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		let mut headers = http::HeaderMap::new();
		headers.insert("X-Forwarded-For", "198.51.100.1".parse().unwrap());
		headers.insert("X-Real-IP", "203.0.113.42".parse().unwrap());

		let request = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.version(http::Version::HTTP_11)
			.headers(headers)
			.body(bytes::Bytes::new())
			.build()
			.unwrap();

		let ip = rate_limit_handler.extract_client_ip(&request);
		// Should prefer X-Forwarded-For over X-Real-IP
		assert_eq!(ip, "198.51.100.1".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn test_extract_client_ip_fallback_to_localhost() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(10);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		let headers = http::HeaderMap::new();

		let request = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.version(http::Version::HTTP_11)
			.headers(headers)
			.body(bytes::Bytes::new())
			.build()
			.unwrap();

		let ip = rate_limit_handler.extract_client_ip(&request);
		// Should fallback to localhost
		assert_eq!(ip, "127.0.0.1".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn test_extract_client_ip_with_invalid_header() {
		let handler = Arc::new(TestHandler);
		let config = RateLimitConfig::per_minute(10);
		let rate_limit_handler = RateLimitHandler::new(handler, config);

		let mut headers = http::HeaderMap::new();
		headers.insert("X-Forwarded-For", "invalid-ip".parse().unwrap());

		let request = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.version(http::Version::HTTP_11)
			.headers(headers)
			.body(bytes::Bytes::new())
			.build()
			.unwrap();

		let ip = rate_limit_handler.extract_client_ip(&request);
		// Should fallback to localhost when parsing fails
		assert_eq!(ip, "127.0.0.1".parse::<IpAddr>().unwrap());
	}
}
