//! Security Middleware
//!
//! Provides comprehensive security headers and redirects:
//! - HSTS (HTTP Strict Transport Security)
//! - SSL/HTTPS redirects
//! - X-Content-Type-Options
//! - X-Frame-Options
//! - Referrer-Policy
//! - Cross-Origin-Opener-Policy (COOP)

use async_trait::async_trait;
use hyper::StatusCode;
use hyper::header::{HeaderValue, LOCATION};
use reinhardt_conf::SecuritySettings;
use reinhardt_http::{Handler, Middleware, Request, Response, Result};
use std::sync::Arc;

/// Security middleware for HTTP security headers and redirects
///
/// # Construction
///
/// Use [`SecurityMiddleware::new`] for sensible defaults, or
/// [`SecurityMiddleware::from_security_settings`] to build from a
/// [`SecuritySettings`] fragment loaded via `reinhardt-conf`.
///
/// Individual fields can be customized via `with_*` builder methods:
///
/// ```
/// use reinhardt_middleware::SecurityMiddleware;
///
/// let middleware = SecurityMiddleware::new()
///     .with_hsts_include_subdomains(true)
///     .with_hsts_preload(true)
///     .with_referrer_policy("strict-origin-when-cross-origin");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SecurityMiddleware {
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
	/// X-Frame-Options value (e.g., "DENY", "SAMEORIGIN")
	pub x_frame_options: Option<String>,
	/// Proxy SSL header name and expected value for identifying secure requests
	pub secure_proxy_ssl_header: Option<(String, String)>,
}

impl Default for SecurityMiddleware {
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
			x_frame_options: Some("DENY".to_string()),
			secure_proxy_ssl_header: None,
		}
	}
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
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/api/data")
	///     .version(Version::HTTP_11)
	///     .headers(HeaderMap::new())
	///     .secure(true)
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
		Self::default()
	}

	/// Create a new SecurityMiddleware from a [`SecuritySettings`] fragment
	///
	/// Maps security-related fields from `SecuritySettings` to the middleware
	/// configuration. Middleware-specific defaults (e.g., `content_type_nosniff`,
	/// `referrer_policy`) are preserved from [`SecurityMiddleware::default`].
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_conf::SecuritySettings;
	/// use reinhardt_middleware::SecurityMiddleware;
	///
	/// let settings = SecuritySettings {
	///     secure_ssl_redirect: true,
	///     secure_hsts_seconds: Some(31536000),
	///     ..Default::default()
	/// };
	///
	/// let middleware = SecurityMiddleware::from_security_settings(&settings);
	/// ```
	pub fn from_security_settings(settings: &SecuritySettings) -> Self {
		let hsts_enabled = settings.secure_hsts_seconds.is_some();
		let hsts_seconds = settings
			.secure_hsts_seconds
			.map(|s| u32::try_from(s).unwrap_or(u32::MAX))
			.unwrap_or(0);

		Self {
			ssl_redirect: settings.secure_ssl_redirect,
			hsts_enabled,
			hsts_seconds,
			hsts_include_subdomains: settings.secure_hsts_include_subdomains,
			hsts_preload: settings.secure_hsts_preload,
			secure_proxy_ssl_header: settings.secure_proxy_ssl_header.clone(),
			..Self::default()
		}
	}

	/// Set whether HSTS is enabled
	pub fn with_hsts(mut self, enabled: bool) -> Self {
		self.hsts_enabled = enabled;
		self
	}

	/// Set the HSTS max-age in seconds
	pub fn with_hsts_seconds(mut self, seconds: u32) -> Self {
		self.hsts_seconds = seconds;
		self
	}

	/// Set whether to include subdomains in HSTS
	pub fn with_hsts_include_subdomains(mut self, include: bool) -> Self {
		self.hsts_include_subdomains = include;
		self
	}

	/// Set whether to include preload directive in HSTS
	pub fn with_hsts_preload(mut self, preload: bool) -> Self {
		self.hsts_preload = preload;
		self
	}

	/// Set whether to redirect HTTP to HTTPS
	pub fn with_ssl_redirect(mut self, redirect: bool) -> Self {
		self.ssl_redirect = redirect;
		self
	}

	/// Set whether to add X-Content-Type-Options: nosniff
	pub fn with_content_type_nosniff(mut self, nosniff: bool) -> Self {
		self.content_type_nosniff = nosniff;
		self
	}

	/// Set the Referrer-Policy header value
	pub fn with_referrer_policy(mut self, policy: impl Into<String>) -> Self {
		self.referrer_policy = Some(policy.into());
		self
	}

	/// Remove the Referrer-Policy header
	pub fn without_referrer_policy(mut self) -> Self {
		self.referrer_policy = None;
		self
	}

	/// Set the Cross-Origin-Opener-Policy header value
	pub fn with_cross_origin_opener_policy(mut self, policy: impl Into<String>) -> Self {
		self.cross_origin_opener_policy = Some(policy.into());
		self
	}

	/// Remove the Cross-Origin-Opener-Policy header
	pub fn without_cross_origin_opener_policy(mut self) -> Self {
		self.cross_origin_opener_policy = None;
		self
	}

	/// Set the X-Frame-Options header value
	pub fn with_x_frame_options(mut self, value: impl Into<String>) -> Self {
		self.x_frame_options = Some(value.into());
		self
	}

	/// Remove the X-Frame-Options header
	pub fn without_x_frame_options(mut self) -> Self {
		self.x_frame_options = None;
		self
	}

	/// Set the proxy SSL header name and expected value
	pub fn with_secure_proxy_ssl_header(
		mut self,
		header: impl Into<String>,
		value: impl Into<String>,
	) -> Self {
		self.secure_proxy_ssl_header = Some((header.into(), value.into()));
		self
	}

	/// Remove the proxy SSL header configuration
	pub fn without_secure_proxy_ssl_header(mut self) -> Self {
		self.secure_proxy_ssl_header = None;
		self
	}

	/// Check if request is secure (HTTPS)
	///
	/// Delegates to `Request::is_secure()` which already validates trusted proxies
	/// before honoring X-Forwarded-Proto headers. If a custom `secure_proxy_ssl_header`
	/// is configured, it is only trusted when the request comes from a trusted proxy.
	fn is_secure(&self, request: &Request) -> bool {
		// Check configured proxy SSL header (only from trusted proxies)
		if let Some((ref header_name, ref header_value)) = self.secure_proxy_ssl_header
			&& let Some(val) = request.headers.get(header_name.as_str())
			&& let Ok(val_str) = val.to_str()
		{
			// Only trust the custom header when request is from a trusted proxy
			if request.is_from_trusted_proxy() {
				return val_str.eq_ignore_ascii_case(header_value);
			}
			// Untrusted source: ignore the spoofable header
		}

		// Delegate to Request::is_secure() which checks:
		// 1. Actual TLS connection (is_secure flag)
		// 2. X-Forwarded-Proto only from trusted proxies
		request.is_secure()
	}

	/// Build HSTS header value
	fn build_hsts_header(&self) -> String {
		let mut parts = vec![format!("max-age={}", self.hsts_seconds)];

		if self.hsts_include_subdomains {
			parts.push("includeSubDomains".to_string());
		}

		if self.hsts_preload {
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
		if self.hsts_enabled && is_secure {
			let hsts_value = self.build_hsts_header();
			if let Ok(header_value) = hsts_value.parse() {
				response
					.headers
					.insert("Strict-Transport-Security", header_value);
			}
		}

		// X-Content-Type-Options
		if self.content_type_nosniff {
			response.headers.insert(
				"X-Content-Type-Options",
				HeaderValue::from_static("nosniff"),
			);
		}

		// Referrer-Policy
		if let Some(ref policy) = self.referrer_policy
			&& let Ok(header_value) = policy.parse()
		{
			response.headers.insert("Referrer-Policy", header_value);
		}

		// Cross-Origin-Opener-Policy
		if let Some(ref policy) = self.cross_origin_opener_policy
			&& let Ok(header_value) = policy.parse()
		{
			response
				.headers
				.insert("Cross-Origin-Opener-Policy", header_value);
		}

		// X-Frame-Options
		if let Some(ref value) = self.x_frame_options
			&& let Ok(header_value) = value.parse()
		{
			response.headers.insert("X-Frame-Options", header_value);
		}
	}
}

#[async_trait]
impl Middleware for SecurityMiddleware {
	async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
		let is_secure = self.is_secure(&request);

		// SSL redirect for all HTTP methods
		if self.ssl_redirect && !is_secure {
			let redirect_url = self.build_https_url(&request);
			let mut response = Response::new(StatusCode::PERMANENT_REDIRECT);
			response.headers.insert(
				LOCATION,
				redirect_url
					.parse()
					.unwrap_or_else(|_| HeaderValue::from_static("/")),
			);
			return Ok(response);
		}

		// Call handler — convert errors to responses so security headers are
		// always applied, even when invoked outside MiddlewareChain.
		let mut response = match handler.handle(request).await {
			Ok(resp) => resp,
			Err(e) => Response::from(e),
		};

		// Add security headers
		self.add_security_headers(&mut response, is_secure);

		Ok(response)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use reinhardt_conf::SecuritySettings;
	use reinhardt_http::{Error, TrustedProxies};
	use rstest::rstest;
	use std::net::{IpAddr, Ipv4Addr, SocketAddr};

	struct TestHandler;

	#[async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::new(StatusCode::OK).with_body(Bytes::from("content")))
		}
	}

	#[tokio::test]
	async fn test_hsts_header_on_secure_request() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(31536000)
			.without_referrer_policy()
			.without_x_frame_options()
			.with_content_type_nosniff(true);
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.secure(true)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(
			response.headers.get("Strict-Transport-Security").unwrap(),
			"max-age=31536000"
		);
	}

	#[tokio::test]
	async fn test_security_middleware_hsts_full() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(63072000)
			.with_hsts_include_subdomains(true)
			.with_hsts_preload(true)
			.without_referrer_policy()
			.without_x_frame_options()
			.with_content_type_nosniff(true);
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.secure(true)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
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

	#[tokio::test]
	async fn test_no_hsts_on_insecure_request() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(31536000)
			.without_referrer_policy()
			.without_x_frame_options()
			.with_content_type_nosniff(true);
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::OK);
		assert!(!response.headers.contains_key("Strict-Transport-Security"));
	}

	#[tokio::test]
	async fn test_ssl_redirect() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(false)
			.with_ssl_redirect(true)
			.with_content_type_nosniff(false)
			.without_referrer_policy()
			.without_x_frame_options();
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

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
		assert_eq!(
			response.headers.get(LOCATION).unwrap(),
			"https://example.com/test?key=value"
		);
	}

	#[tokio::test]
	async fn test_ssl_redirect_applies_to_all_methods() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(false)
			.with_ssl_redirect(true)
			.with_content_type_nosniff(false)
			.without_referrer_policy()
			.without_x_frame_options();

		let mut headers = HeaderMap::new();
		headers.insert(hyper::header::HOST, "example.com".parse().unwrap());

		// Act & Assert - POST should also be redirected to HTTPS
		let handler = Arc::new(TestHandler);
		let request = Request::builder()
			.method(Method::POST)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers.clone())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = middleware.process(request, handler).await.unwrap();
		assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);

		// PUT should also be redirected to HTTPS
		let handler = Arc::new(TestHandler);
		let request = Request::builder()
			.method(Method::PUT)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers.clone())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = middleware.process(request, handler).await.unwrap();
		assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);

		// DELETE should also be redirected to HTTPS
		let handler = Arc::new(TestHandler);
		let request = Request::builder()
			.method(Method::DELETE)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers.clone())
			.body(Bytes::new())
			.build()
			.unwrap();
		let response = middleware.process(request, handler).await.unwrap();
		assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
	}

	#[tokio::test]
	async fn test_content_type_nosniff_header() {
		// Arrange
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

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
		assert_eq!(
			response.headers.get("X-Content-Type-Options").unwrap(),
			"nosniff"
		);
	}

	#[tokio::test]
	async fn test_referrer_policy_header() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(false)
			.with_content_type_nosniff(false)
			.with_referrer_policy("strict-origin-when-cross-origin")
			.without_x_frame_options();
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
		assert_eq!(
			response.headers.get("Referrer-Policy").unwrap(),
			"strict-origin-when-cross-origin"
		);
	}

	#[tokio::test]
	async fn test_cross_origin_opener_policy_header() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(false)
			.with_content_type_nosniff(false)
			.without_referrer_policy()
			.with_cross_origin_opener_policy("same-origin")
			.without_x_frame_options();
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
		assert_eq!(
			response.headers.get("Cross-Origin-Opener-Policy").unwrap(),
			"same-origin"
		);
	}

	#[tokio::test]
	async fn test_all_security_headers_together() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(31536000)
			.with_hsts_include_subdomains(true)
			.with_content_type_nosniff(true)
			.with_referrer_policy("no-referrer")
			.with_cross_origin_opener_policy("same-origin-allow-popups")
			.without_x_frame_options();
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.secure(true)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
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

	#[tokio::test]
	async fn test_is_secure_with_proxy_ssl_header() {
		// Arrange - custom proxy SSL header from a trusted proxy
		let middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(31536000)
			.without_referrer_policy()
			.without_x_frame_options()
			.with_content_type_nosniff(true)
			.with_secure_proxy_ssl_header("X-Custom-Proto", "https");
		let handler = Arc::new(TestHandler);

		let proxy_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
		let proxy_addr = SocketAddr::new(proxy_ip, 8080);

		let mut headers = HeaderMap::new();
		headers.insert("X-Custom-Proto", "https".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.remote_addr(proxy_addr)
			.body(Bytes::new())
			.build()
			.unwrap();
		request.set_trusted_proxies(TrustedProxies::new(vec![proxy_ip]));

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(
			response.headers.get("Strict-Transport-Security").unwrap(),
			"max-age=31536000"
		);
	}

	#[tokio::test]
	async fn test_is_secure_proxy_ssl_header_mismatch() {
		// Arrange - custom proxy header with wrong value from trusted proxy
		let middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(31536000)
			.without_referrer_policy()
			.without_x_frame_options()
			.with_content_type_nosniff(true)
			.with_secure_proxy_ssl_header("X-Custom-Proto", "https");
		let handler = Arc::new(TestHandler);

		let proxy_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
		let proxy_addr = SocketAddr::new(proxy_ip, 8080);

		let mut headers = HeaderMap::new();
		headers.insert("X-Custom-Proto", "http".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.remote_addr(proxy_addr)
			.body(Bytes::new())
			.build()
			.unwrap();
		request.set_trusted_proxies(TrustedProxies::new(vec![proxy_ip]));

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert - HSTS should not be set because request is not secure
		assert_eq!(response.status, StatusCode::OK);
		assert!(!response.headers.contains_key("Strict-Transport-Security"));
	}

	#[rstest::rstest]
	#[case::custom_proxy_ssl_header_untrusted(
		Some(("X-Custom-Proto".to_string(), "https".to_string())),
		"X-Custom-Proto".to_string(),
		"https".to_string(),
		true,
		"custom proxy SSL header from untrusted source"
	)]
	#[case::x_forwarded_proto_untrusted(
		None,
		"x-forwarded-proto".to_string(),
		"https".to_string(),
		false,
		"X-Forwarded-Proto spoofed without trusted proxy"
	)]
	#[tokio::test]
	async fn test_proxy_header_ignored_without_trusted_proxy(
		#[case] secure_proxy_ssl_header: Option<(String, String)>,
		#[case] header_name: String,
		#[case] header_value: String,
		#[case] content_type_nosniff: bool,
		#[case] _scenario: String,
	) {
		// Arrange - proxy-related header from untrusted source should be ignored
		let mut middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(31536000)
			.with_content_type_nosniff(content_type_nosniff)
			.without_referrer_policy()
			.without_x_frame_options();
		if let Some((h, v)) = secure_proxy_ssl_header {
			middleware = middleware.with_secure_proxy_ssl_header(h, v);
		}
		let handler = Arc::new(TestHandler);

		let mut headers = HeaderMap::new();
		headers.insert(
			hyper::header::HeaderName::from_bytes(header_name.as_bytes()).unwrap(),
			header_value.parse().unwrap(),
		);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert - HSTS should NOT be set because header is from untrusted source
		assert_eq!(response.status, StatusCode::OK);
		assert!(!response.headers.contains_key("Strict-Transport-Security"));
	}

	/// Handler that always returns an error to simulate inner handler failure.
	struct ErrorHandler;

	#[async_trait]
	impl Handler for ErrorHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Err(Error::Http("handler error".to_string()))
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_security_headers_applied_on_handler_error() {
		// Arrange
		let middleware = SecurityMiddleware::new()
			.with_hsts(true)
			.with_hsts_seconds(31536000)
			.with_content_type_nosniff(true)
			.with_referrer_policy("same-origin")
			.with_x_frame_options("DENY");
		let handler: Arc<dyn Handler> = Arc::new(ErrorHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.secure(true)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Act
		let response = middleware.process(request, handler).await.unwrap();

		// Assert — error is converted to response with security headers applied
		assert!(response.status.is_client_error());
		assert_eq!(
			response.headers.get("X-Content-Type-Options").unwrap(),
			"nosniff"
		);
		assert_eq!(
			response.headers.get("Strict-Transport-Security").unwrap(),
			"max-age=31536000"
		);
		assert_eq!(
			response.headers.get("Referrer-Policy").unwrap(),
			"same-origin"
		);
		assert_eq!(response.headers.get("X-Frame-Options").unwrap(), "DENY");
	}

	#[rstest]
	fn test_from_security_settings_conversion() {
		// Arrange
		let settings = SecuritySettings {
			secure_ssl_redirect: true,
			secure_hsts_seconds: Some(63072000),
			secure_hsts_include_subdomains: true,
			secure_hsts_preload: true,
			secure_proxy_ssl_header: Some(("X-Forwarded-Proto".to_string(), "https".to_string())),
			..Default::default()
		};

		// Act
		let middleware = SecurityMiddleware::from_security_settings(&settings);

		// Assert
		assert!(middleware.ssl_redirect);
		assert!(middleware.hsts_enabled);
		assert_eq!(middleware.hsts_seconds, 63072000);
		assert!(middleware.hsts_include_subdomains);
		assert!(middleware.hsts_preload);
		assert_eq!(
			middleware.secure_proxy_ssl_header,
			Some(("X-Forwarded-Proto".to_string(), "https".to_string()))
		);
		// Middleware-specific defaults preserved
		assert!(middleware.content_type_nosniff);
		assert_eq!(middleware.referrer_policy, Some("same-origin".to_string()));
		assert_eq!(middleware.x_frame_options, Some("DENY".to_string()));
	}

	#[rstest]
	fn test_from_security_settings_defaults() {
		// Arrange
		let settings = SecuritySettings::default();

		// Act
		let middleware = SecurityMiddleware::from_security_settings(&settings);

		// Assert
		assert!(!middleware.ssl_redirect);
		assert!(!middleware.hsts_enabled);
		assert_eq!(middleware.hsts_seconds, 0);
		assert!(!middleware.hsts_include_subdomains);
		assert!(!middleware.hsts_preload);
		assert!(middleware.secure_proxy_ssl_header.is_none());
		// Middleware-specific defaults preserved
		assert!(middleware.content_type_nosniff);
		assert_eq!(middleware.referrer_policy, Some("same-origin".to_string()));
	}

	#[tokio::test]
	async fn test_from_security_settings_constructor() {
		// Arrange
		let settings = SecuritySettings {
			secure_ssl_redirect: false,
			secure_hsts_seconds: Some(31536000),
			..Default::default()
		};

		// Act
		let middleware = SecurityMiddleware::from_security_settings(&settings);
		let handler = Arc::new(TestHandler);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.secure(true)
			.body(Bytes::new())
			.build()
			.unwrap();

		// Assert
		let response = middleware.process(request, handler).await.unwrap();
		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(
			response.headers.get("Strict-Transport-Security").unwrap(),
			"max-age=31536000"
		);
		assert_eq!(
			response.headers.get("X-Content-Type-Options").unwrap(),
			"nosniff"
		);
	}
}
