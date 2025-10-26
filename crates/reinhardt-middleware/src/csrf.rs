//! CSRF (Cross-Site Request Forgery) protection middleware for Reinhardt
//!
//! This module provides tower/hyper-based CSRF middleware with:
//! - Automatic token generation and validation
//! - Cookie and session-based token storage
//! - Header and form-based token extraction
//! - Configurable trusted origins
//! - Exempt paths for specific endpoints

use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;

// Re-export CSRF functionality from reinhardt-security
pub use reinhardt_security::csrf::{
    CSRF_ALLOWED_CHARS, CSRF_SECRET_LENGTH, CSRF_SESSION_KEY, CSRF_TOKEN_LENGTH, CsrfConfig,
    CsrfMeta, CsrfToken, InvalidTokenFormat, REASON_BAD_ORIGIN, REASON_BAD_REFERER,
    REASON_CSRF_TOKEN_MISSING, REASON_INCORRECT_LENGTH, REASON_INSECURE_REFERER,
    REASON_INVALID_CHARACTERS, REASON_MALFORMED_REFERER, REASON_NO_CSRF_COOKIE, REASON_NO_REFERER,
    RejectRequest, SameSite, check_origin, check_referer, check_token_hmac as check_token,
    get_secret_bytes as get_secret, get_token_hmac as get_token, is_same_domain,
};

/// CSRF middleware configuration
#[derive(Debug, Clone)]
pub struct CsrfMiddlewareConfig {
    /// Base CSRF configuration
    pub csrf_config: CsrfConfig,
    /// Trusted origins for CSRF validation
    pub trusted_origins: Vec<String>,
    /// Paths exempt from CSRF validation
    pub exempt_paths: HashSet<String>,
    /// Whether to check referer header
    pub check_referer_header: bool,
}

impl Default for CsrfMiddlewareConfig {
    fn default() -> Self {
        Self {
            csrf_config: CsrfConfig::default(),
            trusted_origins: vec!["http://localhost".to_string()],
            exempt_paths: HashSet::new(),
            check_referer_header: true,
        }
    }
}

impl CsrfMiddlewareConfig {
    /// Production configuration with security hardening
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_middleware::csrf::CsrfMiddlewareConfig;
    ///
    /// let config = CsrfMiddlewareConfig::production(vec!["https://example.com".to_string()]);
    /// assert!(config.csrf_config.cookie_secure);
    /// assert!(config.check_referer_header);
    /// ```
    pub fn production(trusted_origins: Vec<String>) -> Self {
        Self {
            csrf_config: CsrfConfig::production(),
            trusted_origins,
            exempt_paths: HashSet::new(),
            check_referer_header: true,
        }
    }

    /// Add an exempt path
    pub fn add_exempt_path(mut self, path: String) -> Self {
        self.exempt_paths.insert(path);
        self
    }
}

/// CSRF protection middleware
pub struct CsrfMiddleware {
    config: CsrfMiddlewareConfig,
    /// Shared CSRF secret for testing
    test_secret: Option<String>,
}

impl CsrfMiddleware {
    /// Get session ID from request
    ///
    /// Priority order:
    /// 1. Session ID from extensions (set by session middleware)
    /// 2. Session cookie
    /// 3. Generated from request metadata
    fn get_session_id(request: &Request) -> String {
        // Try to get session ID from extensions (set by session middleware)
        if let Some(session_id) = request.extensions.get::<String>() {
            return session_id.clone();
        }

        // Try to get session ID from cookie
        if let Some(cookie_header) = request.headers.get("Cookie") {
            if let Ok(cookie_str) = cookie_header.to_str() {
                for cookie in cookie_str.split(';') {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == "sessionid" {
                        return parts[1].to_string();
                    }
                }
            }
        }

        // Fallback: generate from request metadata
        // This is not ideal but ensures CSRF protection works without sessions
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(request.uri.to_string().as_bytes());
        hasher.update(
            request
                .headers
                .get("User-Agent")
                .map(|v| v.as_bytes())
                .unwrap_or(b""),
        );
        hex::encode(hasher.finalize())
    }

    /// Create new CSRF middleware with default configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_middleware::csrf::CsrfMiddleware;
    /// use reinhardt_apps::{Handler, Middleware, Request, Response};
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    /// use std::sync::Arc;
    ///
    /// struct TestHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for TestHandler {
    ///     async fn handle(&self, _request: Request) -> reinhardt_apps::Result<Response> {
    ///         Ok(Response::ok().with_body("OK"))
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let middleware = CsrfMiddleware::new();
    /// let handler = Arc::new(TestHandler);
    ///
    /// let request = Request::new(
    ///     Method::GET,
    ///     Uri::from_static("/test"),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::new(),
    /// );
    ///
    /// let response = middleware.process(request, handler).await.unwrap();
    /// assert!(response.headers.contains_key("Set-Cookie"));
    /// # });
    /// ```
    pub fn new() -> Self {
        Self {
            config: CsrfMiddlewareConfig::default(),
            test_secret: None,
        }
    }

    /// Create middleware with custom configuration
    pub fn with_config(config: CsrfMiddlewareConfig) -> Self {
        Self {
            config,
            test_secret: None,
        }
    }

    /// Create middleware with test secret for deterministic testing
    pub fn with_test_secret(secret: String) -> Self {
        Self {
            config: CsrfMiddlewareConfig::default(),
            test_secret: Some(secret),
        }
    }

    /// Extract CSRF token from request
    fn extract_token(&self, request: &Request) -> Option<String> {
        // Try header first
        if let Some(header_value) = request.headers.get(&self.config.csrf_config.header_name) {
            if let Ok(token) = header_value.to_str() {
                return Some(token.to_string());
            }
        }

        // Try cookie
        if let Some(cookie_header) = request.headers.get("Cookie") {
            if let Ok(cookies) = cookie_header.to_str() {
                for cookie in cookies.split(';') {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == self.config.csrf_config.cookie_name {
                        return Some(parts[1].to_string());
                    }
                }
            }
        }

        None
    }

    /// Check if request is from secure connection
    fn is_secure_request(&self, request: &Request) -> bool {
        request.uri.scheme_str() == Some("https")
    }

    /// Get or create CSRF secret
    ///
    /// Returns (secret_bytes, is_new)
    fn get_or_create_secret(&self, request: &Request) -> (Vec<u8>, bool) {
        // Use test secret if available
        if let Some(ref secret) = self.test_secret {
            return (secret.as_bytes().to_vec(), false);
        }

        // For HMAC-based CSRF, we generate a secret based on session_id
        // This ensures consistency across requests from the same session
        let session_id = Self::get_session_id(request);
        let mut hasher = Sha256::new();
        hasher.update(b"csrf_secret");
        hasher.update(session_id.as_bytes());
        let secret_hash = hasher.finalize();

        // Convert to Vec<u8>
        (secret_hash.to_vec(), false)
    }

    /// Build Set-Cookie header
    fn build_set_cookie_header(&self, token: &str) -> String {
        let mut cookie = format!(
            "{}={}; Path={}",
            self.config.csrf_config.cookie_name, token, self.config.csrf_config.cookie_path
        );

        if self.config.csrf_config.cookie_secure {
            cookie.push_str("; Secure");
        }

        if self.config.csrf_config.cookie_httponly {
            cookie.push_str("; HttpOnly");
        }

        match self.config.csrf_config.cookie_samesite {
            SameSite::Strict => cookie.push_str("; SameSite=Strict"),
            SameSite::Lax => cookie.push_str("; SameSite=Lax"),
            SameSite::None => cookie.push_str("; SameSite=None"),
        }

        if let Some(domain) = &self.config.csrf_config.cookie_domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        if let Some(max_age) = self.config.csrf_config.cookie_max_age {
            cookie.push_str(&format!("; Max-Age={}", max_age));
        }

        cookie
    }

    /// Validate CSRF token for unsafe methods
    fn validate_csrf(&self, request: &Request) -> Result<()> {
        // Check referer if configured
        if self.config.check_referer_header {
            let referer = request.headers.get("Referer").and_then(|v| v.to_str().ok());
            let is_secure = self.is_secure_request(request);

            check_referer(referer, &self.config.trusted_origins, is_secure).map_err(|e| {
                reinhardt_apps::Error::Authorization(format!(
                    "CSRF validation failed: {}",
                    e.reason
                ))
            })?;
        }

        // Extract and validate token
        let token = self.extract_token(request).ok_or_else(|| {
            reinhardt_apps::Error::Authorization(REASON_CSRF_TOKEN_MISSING.to_string())
        })?;

        // Get secret and session_id
        let (secret, _) = self.get_or_create_secret(request);
        let session_id = Self::get_session_id(request);

        // Validate token using HMAC
        check_token(&token, &secret, &session_id).map_err(|e| {
            reinhardt_apps::Error::Authorization(format!(
                "CSRF token validation failed: {}",
                e.reason
            ))
        })?;

        Ok(())
    }
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for CsrfMiddleware {
    async fn process(&self, request: Request, handler: Arc<dyn Handler>) -> Result<Response> {
        // Check if path is exempt
        let path = request.uri.path();
        if self.config.exempt_paths.contains(path) {
            return handler.handle(request).await;
        }

        // Get or create CSRF secret
        let (secret, _is_new) = self.get_or_create_secret(&request);

        // Safe methods (GET, HEAD, OPTIONS, TRACE) don't require CSRF validation
        let is_safe_method = matches!(
            request.method,
            Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
        );

        // Validate CSRF for unsafe methods
        if !is_safe_method {
            self.validate_csrf(&request)?;
        }

        // Generate HMAC-based token using session_id (before moving request)
        let session_id = Self::get_session_id(&request);
        let token = get_token(&secret, &session_id);

        // Process request
        let mut response = handler.handle(request).await?;
        let cookie_header = self.build_set_cookie_header(&token);
        response
            .headers
            .insert("Set-Cookie", cookie_header.parse().unwrap());

        Ok(response)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, StatusCode, Uri, Version};

    struct TestHandler;

    #[async_trait]
    impl Handler for TestHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            Ok(Response::ok().with_body("Test response"))
        }
    }

    #[tokio::test]
    async fn test_csrf_middleware_get_request_sets_cookie() {
        let middleware = CsrfMiddleware::new();
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        assert_eq!(response.status, StatusCode::OK);
        assert!(response.headers.contains_key("Set-Cookie"));

        let cookie = response
            .headers
            .get("Set-Cookie")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.contains("csrftoken="));
        assert!(cookie.contains("Path=/"));
    }

    #[tokio::test]
    async fn test_csrf_middleware_post_without_token_fails() {
        let middleware = CsrfMiddleware::new();
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::POST,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let result = middleware.process(request, handler).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_csrf_middleware_post_with_valid_token_succeeds() {
        let secret = "abcdefghijklmnopqrstuvwxyz012345";
        let mut config = CsrfMiddlewareConfig::default();
        config.check_referer_header = false;

        let mut csrf_middleware = CsrfMiddleware::with_config(config);
        csrf_middleware.test_secret = Some(secret.to_string());

        let handler = Arc::new(TestHandler);
        let session_id = "test_session_id";
        let token = get_token(secret.as_bytes(), session_id);

        let mut headers = HeaderMap::new();
        headers.insert("X-CSRFToken", token.parse().unwrap());
        // Add session cookie with session_id
        headers.insert(
            "Cookie",
            format!("csrftoken={}; sessionid={}", token, session_id)
                .parse()
                .unwrap(),
        );

        let request = Request::new(
            Method::POST,
            Uri::from_static("/test"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let response = csrf_middleware.process(request, handler).await.unwrap();
        assert_eq!(response.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_middleware_post_with_invalid_token_fails() {
        let secret = "abcdefghijklmnopqrstuvwxyz012345";
        let middleware = CsrfMiddleware::with_test_secret(secret.to_string());
        let handler = Arc::new(TestHandler);

        let mut headers = HeaderMap::new();
        headers.insert("X-CSRFToken", "invalid_token_here".parse().unwrap());

        let request = Request::new(
            Method::POST,
            Uri::from_static("/test"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = middleware.process(request, handler).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_csrf_middleware_exempt_paths() {
        let mut config = CsrfMiddlewareConfig::default();
        config.exempt_paths.insert("/exempt".to_string());

        let middleware = CsrfMiddleware::with_config(config);
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::POST,
            Uri::from_static("/exempt"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();
        assert_eq!(response.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_middleware_safe_methods() {
        let middleware = CsrfMiddleware::new();
        let handler = Arc::new(TestHandler);

        for method in &[Method::GET, Method::HEAD, Method::OPTIONS] {
            let request = Request::new(
                method.clone(),
                Uri::from_static("/test"),
                Version::HTTP_11,
                HeaderMap::new(),
                Bytes::new(),
            );

            let response = middleware.process(request, handler.clone()).await.unwrap();
            assert_eq!(response.status, StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_csrf_middleware_token_from_cookie() {
        let secret = "abcdefghijklmnopqrstuvwxyz012345";
        let mut config = CsrfMiddlewareConfig::default();
        config.check_referer_header = false;

        let mut csrf_middleware = CsrfMiddleware::with_config(config);
        csrf_middleware.test_secret = Some(secret.to_string());

        let handler = Arc::new(TestHandler);
        let session_id = "test_session_id";
        let token = get_token(secret.as_bytes(), session_id);

        let mut headers = HeaderMap::new();
        // Add session cookie with session_id
        headers.insert(
            "Cookie",
            format!("csrftoken={}; sessionid={}", token, session_id)
                .parse()
                .unwrap(),
        );
        headers.insert("X-CSRFToken", token.parse().unwrap());

        let request = Request::new(
            Method::POST,
            Uri::from_static("/test"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let response = csrf_middleware.process(request, handler).await.unwrap();
        assert_eq!(response.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_middleware_production_config() {
        let config = CsrfMiddlewareConfig::production(vec!["https://example.com".to_string()]);
        let middleware = CsrfMiddleware::with_config(config);
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();
        assert!(response.headers.contains_key("Set-Cookie"));

        let cookie = response
            .headers
            .get("Set-Cookie")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
    }

    #[tokio::test]
    async fn test_build_set_cookie_header() {
        let middleware = CsrfMiddleware::new();
        let token = "test_token_1234567890";
        let cookie = middleware.build_set_cookie_header(token);

        assert!(cookie.contains("csrftoken=test_token_1234567890"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("SameSite=Lax"));
    }

    #[tokio::test]
    async fn test_extract_token_from_header() {
        let middleware = CsrfMiddleware::new();

        let mut headers = HeaderMap::new();
        headers.insert("X-CSRFToken", "my_token_value".parse().unwrap());

        let request = Request::new(
            Method::POST,
            Uri::from_static("/test"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let token = middleware.extract_token(&request);
        assert_eq!(token, Some("my_token_value".to_string()));
    }

    #[tokio::test]
    async fn test_extract_token_from_cookie() {
        let middleware = CsrfMiddleware::new();

        let mut headers = HeaderMap::new();
        headers.insert("Cookie", "csrftoken=cookie_token_value".parse().unwrap());

        let request = Request::new(
            Method::POST,
            Uri::from_static("/test"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let token = middleware.extract_token(&request);
        assert_eq!(token, Some("cookie_token_value".to_string()));
    }

    #[tokio::test]
    async fn test_is_secure_request() {
        let middleware = CsrfMiddleware::new();

        let request_http = Request::new(
            Method::GET,
            Uri::from_static("http://example.com/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );
        assert!(!middleware.is_secure_request(&request_http));

        let request_https = Request::new(
            Method::GET,
            Uri::from_static("https://example.com/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );
        assert!(middleware.is_secure_request(&request_https));
    }

    #[tokio::test]
    async fn test_csrf_middleware_config_add_exempt_path() {
        let config = CsrfMiddlewareConfig::default()
            .add_exempt_path("/api/webhook".to_string())
            .add_exempt_path("/health".to_string());

        assert!(config.exempt_paths.contains("/api/webhook"));
        assert!(config.exempt_paths.contains("/health"));
    }
}
