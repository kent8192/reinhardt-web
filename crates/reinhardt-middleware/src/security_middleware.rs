//! Security Middleware
//!
//! Provides comprehensive security headers and redirects:
//! - HSTS (HTTP Strict Transport Security)
//! - SSL/HTTPS redirects
//! - X-Content-Type-Options
//! - Referrer-Policy
//! - Cross-Origin-Opener-Policy (COOP)

use async_trait::async_trait;
use hyper::header::{HeaderValue, LOCATION};
use hyper::{Method, StatusCode};
use reinhardt_http::{Handler, Middleware, Request, Response, Result};
use std::sync::Arc;

/// Security middleware configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
	/// Enable HSTS (HTTP Strict Transport Security)
	pub hsts_enabled: bool,
	/// HSTS max-age in seconds (default: 31536000 = 1 year)
	pub hsts_seconds: u32,
	/// Include subdomains in HSTS
	pub hsts_include_subdomains: bool,
	/// Include preload directive in HSTS
	pub hsts_preload: bool,
	/// Redirect HTTP to HTTPS
	pub ssl_redirect: bool,
	/// Set X-Content-Type-Options: nosniff
	pub content_type_nosniff: bool,
	/// Referrer-Policy value
	pub referrer_policy: Option<String>,
	/// Cross-Origin-Opener-Policy value
	pub cross_origin_opener_policy: Option<String>,
}

impl Default for SecurityConfig {
	fn default() -> Self {
		Self {
			hsts_enabled: true,
			hsts_seconds: 31536000, // 1 year
			hsts_include_subdomains: false,
			hsts_preload: false,
			ssl_redirect: false,
			content_type_nosniff: true,
			referrer_policy: Some("same-origin".to_string()),
			cross_origin_opener_policy: None,
		}
	}
}

/// Security middleware for HTTP security headers and redirects
pub struct SecurityMiddleware {
	config: SecurityConfig,
}

impl SecurityMiddleware {
	/// Create a new SecurityMiddleware with default configuration
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_middleware::SecurityMiddleware;
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
	/// let middleware = SecurityMiddleware::new();
	/// let handler = Arc::new(TestHandler);
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert("x-forwarded-proto", "https".parse().unwrap());
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
	/// assert!(response.headers.contains_key("Strict-Transport-Security"));
	/// assert_eq!(response.headers.get("X-Content-Type-Options").unwrap(), "nosniff");
	/// # });
	/// ```
	pub fn new() -> Self {
		Self {
			config: SecurityConfig::default(),
		}
	}
	/// Create a new SecurityMiddleware with custom configuration
	///
	/// # Examples
	///
	/// ```
	/// use std::sync::Arc;
	/// use reinhardt_middleware::{SecurityMiddleware, SecurityConfig};
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
	/// let config = SecurityConfig {
	///     hsts_enabled: true,
	///     hsts_seconds: 31536000,
	///     hsts_include_subdomains: true,
	///     hsts_preload: true,
	///     ssl_redirect: false,
	///     content_type_nosniff: true,
	///     referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
	///     cross_origin_opener_policy: Some("same-origin".to_string()),
	/// };
	///
	/// let middleware = SecurityMiddleware::with_config(config);
	/// let handler = Arc::new(TestHandler);
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert("x-forwarded-proto", "https".parse().unwrap());
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/secure")
	///     .version(Version::HTTP_11)
	///     .headers(headers)
	///     .body(Bytes::new())
	///     .build()
	///     .unwrap();
	///
	/// let response = middleware.process(request, handler).await.unwrap();
	/// let hsts = response.headers.get("Strict-Transport-Security").unwrap().to_str().unwrap();
	/// assert!(hsts.contains("max-age=31536000"));
	/// assert!(hsts.contains("includeSubDomains"));
	/// assert!(hsts.contains("preload"));
	/// assert_eq!(response.headers.get("Referrer-Policy").unwrap(), "strict-origin-when-cross-origin");
	/// # });
	/// ```
	pub fn with_config(config: SecurityConfig) -> Self {
		Self { config }
	}

	/// Check if request is secure (HTTPS)
	fn is_secure(&self, request: &Request) -> bool {
		// Check X-Forwarded-Proto header
		if let Some(proto) = request.headers.get("x-forwarded-proto")
			&& let Ok(proto_str) = proto.to_str()
		{
			return proto_str.eq_ignore_ascii_case("https");
		}

		// Check if URI scheme is https
		request.uri.scheme_str() == Some("https")
	}

	/// Build HSTS header value
	fn build_hsts_header(&self) -> String {
		let mut parts = vec![format!("max-age={}", self.config.hsts_seconds)];

		if self.config.hsts_include_subdomains {
			parts.push("includeSubDomains".to_string());
		}

		if self.config.hsts_preload {
			parts.push("preload".to_string());
		}

		parts.join("; ")
	}

	/// Build redirect URL for HTTPS
	fn build_https_url(&self, request: &Request) -> String {
		let host = request
			.headers
			.get(hyper::header::HOST)
			.and_then(|h| h.to_str().ok())
			.unwrap_or("localhost");

		let path_and_query = request
			.uri
			.path_and_query()
			.map(|pq| pq.as_str())
			.unwrap_or("/");

		format!("https://{}{}", host, path_and_query)
	}

	/// Add security headers to response
	fn add_security_headers(&self, response: &mut Response, is_secure: bool) {
		// HSTS header (only on HTTPS)
		if self.config.hsts_enabled && is_secure {
			let hsts_value = self.build_hsts_header();
			response
				.headers
				.insert("Strict-Transport-Security", hsts_value.parse().unwrap());
		}

		// X-Content-Type-Options
		if self.config.content_type_nosniff {
			response.headers.insert(
				"X-Content-Type-Options",
				HeaderValue::from_static("nosniff"),
			);
		}

		// Referrer-Policy
		if let Some(ref policy) = self.config.referrer_policy {
			response
				.headers
				.insert("Referrer-Policy", policy.parse().unwrap());
		}

		// Cross-Origin-Opener-Policy
		if let Some(ref policy) = self.config.cross_origin_opener_policy {
			response
				.headers
				.insert("Cross-Origin-Opener-Policy", policy.parse().unwrap());
		}
	}
}

impl Default for SecurityMiddleware {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Middleware for SecurityMiddleware {
	async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
		let is_secure = self.is_secure(&request);

		// SSL redirect (only for GET and HEAD requests)
		if self.config.ssl_redirect
			&& !is_secure
			&& (request.method == Method::GET || request.method == Method::HEAD)
		{
			let redirect_url = self.build_https_url(&request);
			let mut response = Response::new(StatusCode::MOVED_PERMANENTLY);
			response
				.headers
				.insert(LOCATION, redirect_url.parse().unwrap());
			return Ok(response);
		}

		// Call handler
		let mut response = handler.handle(request).await?;

		// Add security headers
		self.add_security_headers(&mut response, is_secure);

		Ok(response)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Version};
	use rstest::rstest;

	struct TestHandler;

	#[async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::new(StatusCode::OK).with_body(Bytes::from("content")))
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_hsts_header_on_secure_request() {
		let config = SecurityConfig {
			hsts_enabled: true,
			hsts_seconds: 31536000,
			hsts_include_subdomains: false,
			hsts_preload: false,
			ssl_redirect: false,
			content_type_nosniff: true,
			referrer_policy: None,
			cross_origin_opener_policy: None,
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert("x-forwarded-proto", "https".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(
			response.headers.get("Strict-Transport-Security").unwrap(),
			"max-age=31536000"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_security_middleware_hsts_full() {
		let config = SecurityConfig {
			hsts_enabled: true,
			hsts_seconds: 63072000,
			hsts_include_subdomains: true,
			hsts_preload: true,
			ssl_redirect: false,
			content_type_nosniff: true,
			referrer_policy: None,
			cross_origin_opener_policy: None,
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert("x-forwarded-proto", "https".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		let hsts_header = response
			.headers
			.get("Strict-Transport-Security")
			.unwrap()
			.to_str()
			.unwrap();
		assert!(hsts_header.contains("max-age=63072000"));
		assert!(hsts_header.contains("includeSubDomains"));
		assert!(hsts_header.contains("preload"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_no_hsts_on_insecure_request() {
		let config = SecurityConfig {
			hsts_enabled: true,
			hsts_seconds: 31536000,
			hsts_include_subdomains: false,
			hsts_preload: false,
			ssl_redirect: false,
			content_type_nosniff: true,
			referrer_policy: None,
			cross_origin_opener_policy: None,
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
		assert!(!response.headers.contains_key("Strict-Transport-Security"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_ssl_redirect() {
		let config = SecurityConfig {
			hsts_enabled: false,
			hsts_seconds: 0,
			hsts_include_subdomains: false,
			hsts_preload: false,
			ssl_redirect: true,
			content_type_nosniff: false,
			referrer_policy: None,
			cross_origin_opener_policy: None,
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert(hyper::header::HOST, "example.com".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test?key=value")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
		assert_eq!(
			response.headers.get(LOCATION).unwrap(),
			"https://example.com/test?key=value"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_ssl_redirect_only_for_get_and_head() {
		let config = SecurityConfig {
			hsts_enabled: false,
			hsts_seconds: 0,
			hsts_include_subdomains: false,
			hsts_preload: false,
			ssl_redirect: true,
			content_type_nosniff: false,
			referrer_policy: None,
			cross_origin_opener_policy: None,
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert(hyper::header::HOST, "example.com".parse().unwrap());

		let request = Request::builder()
			.method(Method::POST)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		// POST should not be redirected
		assert_eq!(response.status, StatusCode::OK);
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_nosniff_header() {
		let middleware = SecurityMiddleware::new();
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(
			response.headers.get("X-Content-Type-Options").unwrap(),
			"nosniff"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_referrer_policy_header() {
		let config = SecurityConfig {
			hsts_enabled: false,
			hsts_seconds: 0,
			hsts_include_subdomains: false,
			hsts_preload: false,
			ssl_redirect: false,
			content_type_nosniff: false,
			referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
			cross_origin_opener_policy: None,
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(
			response.headers.get("Referrer-Policy").unwrap(),
			"strict-origin-when-cross-origin"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_cross_origin_opener_policy_header() {
		let config = SecurityConfig {
			hsts_enabled: false,
			hsts_seconds: 0,
			hsts_include_subdomains: false,
			hsts_preload: false,
			ssl_redirect: false,
			content_type_nosniff: false,
			referrer_policy: None,
			cross_origin_opener_policy: Some("same-origin".to_string()),
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(
			response.headers.get("Cross-Origin-Opener-Policy").unwrap(),
			"same-origin"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_all_security_headers_together() {
		let config = SecurityConfig {
			hsts_enabled: true,
			hsts_seconds: 31536000,
			hsts_include_subdomains: true,
			hsts_preload: false,
			ssl_redirect: false,
			content_type_nosniff: true,
			referrer_policy: Some("no-referrer".to_string()),
			cross_origin_opener_policy: Some("same-origin-allow-popups".to_string()),
		};
		let middleware = SecurityMiddleware::with_config(config);
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert("x-forwarded-proto", "https".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert!(response.headers.contains_key("Strict-Transport-Security"));
		assert_eq!(
			response.headers.get("X-Content-Type-Options").unwrap(),
			"nosniff"
		);
		assert_eq!(
			response.headers.get("Referrer-Policy").unwrap(),
			"no-referrer"
		);
		assert_eq!(
			response.headers.get("Cross-Origin-Opener-Policy").unwrap(),
			"same-origin-allow-popups"
		);
	}
}
