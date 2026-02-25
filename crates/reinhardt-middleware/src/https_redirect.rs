//! HTTPS Redirect Middleware
//!
//! Automatically redirects HTTP requests to HTTPS.
//! Similar to Django's SECURE_SSL_REDIRECT setting.

use async_trait::async_trait;
use hyper::StatusCode;
use reinhardt_http::{Handler, Middleware, Request, Response, Result};
use std::sync::Arc;

/// Configuration for HTTPS redirect middleware
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct HttpsRedirectConfig {
	/// Enable HTTPS redirect
	pub enabled: bool,
	/// Exempt paths from HTTPS redirect (e.g., health checks)
	pub exempt_paths: Vec<String>,
	/// Redirect status code (301 or 302)
	pub status_code: StatusCode,
}

impl Default for HttpsRedirectConfig {
	fn default() -> Self {
		Self {
			enabled: true,
			exempt_paths: vec![],
			status_code: StatusCode::MOVED_PERMANENTLY, // 301
		}
	}
}

/// Middleware to redirect HTTP requests to HTTPS
pub struct HttpsRedirectMiddleware {
	config: HttpsRedirectConfig,
}

impl HttpsRedirectMiddleware {
	/// Create a new HttpsRedirectMiddleware with the given configuration
	///
	/// # Arguments
	///
	/// * `config` - HTTPS redirect configuration
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_middleware::{HttpsRedirectMiddleware, HttpsRedirectConfig};
	/// use reinhardt_http::{Handler, Middleware, Request, Response};
	/// use hyper::{StatusCode, Method, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// struct TestHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for TestHandler {
	///     async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::new(StatusCode::OK))
	///     }
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let config = HttpsRedirectConfig {
	///     enabled: true,
	///     exempt_paths: vec!["/health".to_string()],
	///     status_code: StatusCode::MOVED_PERMANENTLY,
	/// };
	///
	/// let middleware = HttpsRedirectMiddleware::new(config);
	/// let handler = Arc::new(TestHandler);
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert(hyper::header::HOST, "example.com".parse().unwrap());
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/api/data")
	///     .version(Version::HTTP_11)
	///     .headers(headers)
	///     .body(Bytes::new())
	///     .build()
	///     .unwrap();
	///
	/// let response = middleware.process(request, handler).await.unwrap();
	/// assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
	/// assert_eq!(response.headers.get("Location").unwrap(), "https://example.com/api/data");
	/// # });
	/// ```
	pub fn new(config: HttpsRedirectConfig) -> Self {
		Self { config }
	}
	/// Create with default configuration
	///
	/// Default configuration enables HTTPS redirect with 301 status code and no exempt paths.
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_middleware::HttpsRedirectMiddleware;
	/// use reinhardt_http::{Handler, Middleware, Request, Response};
	/// use hyper::{StatusCode, Method, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// struct TestHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for TestHandler {
	///     async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::new(StatusCode::OK))
	///     }
	/// }
	///
	/// # tokio_test::block_on(async {
	/// let middleware = HttpsRedirectMiddleware::default_config();
	/// let handler = Arc::new(TestHandler);
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert(hyper::header::HOST, "api.example.com".parse().unwrap());
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/users?page=1")
	///     .version(Version::HTTP_11)
	///     .headers(headers)
	///     .body(Bytes::new())
	///     .build()
	///     .unwrap();
	///
	/// let response = middleware.process(request, handler).await.unwrap();
	/// assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
	/// assert_eq!(response.headers.get("Location").unwrap(), "https://api.example.com/users?page=1");
	/// # });
	/// ```
	pub fn default_config() -> Self {
		Self {
			config: HttpsRedirectConfig::default(),
		}
	}

	/// Check if a path is exempt from HTTPS redirect
	fn is_exempt(&self, path: &str) -> bool {
		self.config
			.exempt_paths
			.iter()
			.any(|exempt| path.starts_with(exempt))
	}
}

#[async_trait]
impl Middleware for HttpsRedirectMiddleware {
	async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
		// If HTTPS redirect is disabled, just pass through
		if !self.config.enabled {
			return handler.handle(request).await;
		}

		// If request is already secure, pass through
		if request.is_secure() {
			return handler.handle(request).await;
		}

		// If path is exempt, pass through
		if self.is_exempt(request.path()) {
			return handler.handle(request).await;
		}

		// Build HTTPS redirect URL
		let https_url = format!(
			"https://{}{}",
			request
				.headers
				.get(hyper::header::HOST)
				.and_then(|h| h.to_str().ok())
				.unwrap_or("localhost"),
			request
				.uri
				.path_and_query()
				.map(|pq| pq.as_str())
				.unwrap_or("/")
		);

		// Return redirect response
		let mut response = Response::new(self.config.status_code);
		response
			.headers
			.insert(hyper::header::LOCATION, https_url.parse().unwrap());
		Ok(response)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Version};
	use reinhardt_http::Request;

	struct TestHandler;

	#[async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::ok().with_body(Bytes::from("test")))
		}
	}

	#[tokio::test]
	async fn test_redirect_http_to_https() {
		let middleware = HttpsRedirectMiddleware::default_config();
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert(hyper::header::HOST, "example.com".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
		assert_eq!(
			response.headers.get("Location").unwrap(),
			"https://example.com/test"
		);
	}

	#[tokio::test]
	async fn test_no_redirect_for_https() {
		let middleware = HttpsRedirectMiddleware::default_config();
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert(hyper::header::HOST, "example.com".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.secure(true)
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
	}

	#[tokio::test]
	async fn test_exempt_paths() {
		let config = HttpsRedirectConfig {
			enabled: true,
			exempt_paths: vec!["/health".to_string()],
			status_code: StatusCode::MOVED_PERMANENTLY,
		};
		let middleware = HttpsRedirectMiddleware::new(config);
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert(hyper::header::HOST, "example.com".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/health")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		// Should not redirect exempt paths
		assert_eq!(response.status, StatusCode::OK);
	}
}
