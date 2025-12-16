pub mod auth;
pub mod broken_link;
#[cfg(feature = "compression")]
pub mod brotli;
pub mod cache;
pub mod circuit_breaker;
pub mod common;
pub mod conditional;
#[cfg(feature = "cors")]
pub mod cors;
pub mod csp;
pub mod csp_helpers;
pub mod csrf;
pub mod etag;
pub mod flatpages;
#[cfg(feature = "compression")]
pub mod gzip;
pub mod https_redirect;
pub mod locale;
pub mod logging;
pub mod messages;
pub mod metrics;
#[cfg(feature = "rate-limit")]
pub mod rate_limit;
pub mod redirect_fallback;
pub mod request_id;
#[cfg(feature = "security")]
pub mod security_middleware;
pub mod session;
pub mod site;
pub mod timeout;
pub mod tracing;
pub mod xframe;

// Re-export core middleware traits from reinhardt-types
pub use reinhardt_core::types::{Handler, Middleware, MiddlewareChain};

#[cfg(feature = "sessions")]
pub use auth::AuthenticationMiddleware;
pub use broken_link::{BrokenLinkConfig, BrokenLinkEmailsMiddleware};
#[cfg(feature = "compression")]
pub use brotli::{BrotliConfig, BrotliMiddleware, BrotliQuality};
pub use cache::{CacheConfig, CacheKeyStrategy, CacheMiddleware, CacheStore};
pub use circuit_breaker::{CircuitBreakerConfig, CircuitBreakerMiddleware, CircuitState};
pub use common::{CommonConfig, CommonMiddleware};
pub use conditional::ConditionalGetMiddleware;
#[cfg(feature = "cors")]
pub use cors::CorsMiddleware;
pub use csp::{CspConfig, CspMiddleware, CspNonce};
pub use csp_helpers::{csp_nonce_attr, get_csp_nonce};
pub use csrf::{
	CSRF_ALLOWED_CHARS, CSRF_SECRET_LENGTH, CSRF_SESSION_KEY, CSRF_TOKEN_LENGTH, CsrfConfig,
	CsrfMeta, CsrfMiddleware, CsrfMiddlewareConfig, CsrfToken, InvalidTokenFormat,
	REASON_BAD_ORIGIN, REASON_BAD_REFERER, REASON_CSRF_TOKEN_MISSING, REASON_INCORRECT_LENGTH,
	REASON_INSECURE_REFERER, REASON_INVALID_CHARACTERS, REASON_MALFORMED_REFERER,
	REASON_NO_CSRF_COOKIE, REASON_NO_REFERER, RejectRequest, SameSite, check_origin, check_referer,
	check_token, get_secret, get_token, is_same_domain,
};
pub use etag::{ETagConfig, ETagMiddleware};
pub use flatpages::{Flatpage, FlatpageStore, FlatpagesConfig, FlatpagesMiddleware};
#[cfg(feature = "compression")]
pub use gzip::{GZipConfig, GZipMiddleware};
pub use https_redirect::{HttpsRedirectConfig, HttpsRedirectMiddleware};
pub use locale::{LocaleConfig, LocaleMiddleware};
pub use logging::{LoggingConfig, LoggingMiddleware};
pub use messages::{CookieStorage, Message, MessageLevel, MessageStorage, SessionStorage};
pub use metrics::{MetricsConfig, MetricsMiddleware, MetricsStore};
#[cfg(feature = "rate-limit")]
pub use rate_limit::{RateLimitConfig, RateLimitMiddleware, RateLimitStore, RateLimitStrategy};
pub use redirect_fallback::{RedirectFallbackMiddleware, RedirectResponseConfig};
pub use request_id::{REQUEST_ID_HEADER, RequestIdConfig, RequestIdMiddleware};
#[cfg(feature = "security")]
pub use security_middleware::{SecurityConfig, SecurityMiddleware};
pub use session::{SessionConfig, SessionData, SessionMiddleware, SessionStore};
pub use site::{SITE_ID_HEADER, Site, SiteConfig, SiteMiddleware, SiteRegistry};
pub use timeout::{TimeoutConfig, TimeoutMiddleware};
pub use tracing::{
	PARENT_SPAN_ID_HEADER, SPAN_ID_HEADER, Span, SpanStatus, TRACE_ID_HEADER, TraceStore,
	TracingConfig, TracingMiddleware,
};
pub use xframe::{XFrameOptions, XFrameOptionsMiddleware};

#[cfg(all(test, feature = "cors"))]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Version};
	use reinhardt_core::{
		Handler, Middleware,
		http::{Request, Response},
	};
	use std::sync::Arc;

	struct TestHandler;

	#[async_trait::async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
			Ok(Response::ok().with_body("test response".as_bytes()))
		}
	}

	#[tokio::test]
	async fn test_cors_middleware_simple_request() {
		use cors::CorsConfig;

		let config = CorsConfig {
			allow_origins: vec!["http://example.com".to_string()],
			allow_methods: vec!["GET".to_string(), "POST".to_string()],
			allow_headers: vec!["Content-Type".to_string()],
			allow_credentials: false,
			max_age: Some(3600),
		};

		let middleware = CorsMiddleware::new(config);
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
			response.headers.get("Access-Control-Allow-Origin").unwrap(),
			"http://example.com"
		);
	}

	#[tokio::test]
	async fn test_cors_middleware_preflight_request() {
		use cors::CorsConfig;

		let config = CorsConfig {
			allow_origins: vec!["http://example.com".to_string()],
			allow_methods: vec!["GET".to_string(), "POST".to_string()],
			allow_headers: vec!["Content-Type".to_string()],
			allow_credentials: false,
			max_age: Some(3600),
		};

		let middleware = CorsMiddleware::new(config);
		let handler = Arc::new(TestHandler);
		let request = Request::builder()
			.method(Method::OPTIONS)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = middleware.process(request, handler).await.unwrap();

		assert_eq!(response.status, StatusCode::NO_CONTENT);
		assert!(response.headers.contains_key("Access-Control-Allow-Origin"));
		assert!(
			response
				.headers
				.contains_key("Access-Control-Allow-Methods")
		);
		assert!(
			response
				.headers
				.contains_key("Access-Control-Allow-Headers")
		);
	}

	#[tokio::test]
	async fn test_cors_middleware_permissive() {
		let middleware = CorsMiddleware::permissive();
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

		assert!(response.headers.contains_key("Access-Control-Allow-Origin"));
	}

	#[tokio::test]
	async fn test_logging_middleware() {
		let middleware = LoggingMiddleware::new();
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
	}
}
