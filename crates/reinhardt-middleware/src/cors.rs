use async_trait::async_trait;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use std::sync::Arc;

/// CORS middleware configuration
pub struct CorsConfig {
    pub allow_origins: Vec<String>,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allow_origins: vec!["*".to_string()],
            allow_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "PATCH".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            allow_credentials: false,
            max_age: Some(3600),
        }
    }
}

/// CORS middleware
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    /// Create a new CORS middleware with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - CORS configuration specifying allowed origins, methods, headers, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use reinhardt_middleware::{CorsMiddleware, cors::CorsConfig};
    /// use reinhardt_apps::{Handler, Middleware, Request, Response};
    /// use hyper::{StatusCode, Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    /// struct TestHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for TestHandler {
    ///     async fn handle(&self, _request: Request) -> reinhardt_apps::Result<Response> {
    ///         Ok(Response::new(StatusCode::OK).with_body(Bytes::from("OK")))
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let config = CorsConfig {
    ///     allow_origins: vec!["https://example.com".to_string()],
    ///     allow_methods: vec!["GET".to_string(), "POST".to_string()],
    ///     allow_headers: vec!["Content-Type".to_string()],
    ///     allow_credentials: true,
    ///     max_age: Some(3600),
    /// };
    ///
    /// let middleware = CorsMiddleware::new(config);
    /// let handler = Arc::new(TestHandler);
    ///
    /// let request = Request::new(
    ///     Method::GET,
    ///     Uri::from_static("/api/data"),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::new(),
    /// );
    ///
    /// let response = middleware.process(request, handler).await.unwrap();
    /// assert_eq!(response.headers.get("Access-Control-Allow-Origin").unwrap(), "https://example.com");
    /// assert_eq!(response.headers.get("Access-Control-Allow-Credentials").unwrap(), "true");
    /// # });
    /// ```
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }
    /// Create a permissive CORS middleware that allows all origins
    ///
    /// This is useful for development but should be used with caution in production.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use reinhardt_middleware::CorsMiddleware;
    /// use reinhardt_apps::{Handler, Middleware, Request, Response};
    /// use hyper::{StatusCode, Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    /// struct TestHandler;
    ///
    /// #[async_trait::async_trait]
    /// impl Handler for TestHandler {
    ///     async fn handle(&self, _request: Request) -> reinhardt_apps::Result<Response> {
    ///         Ok(Response::new(StatusCode::OK))
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let middleware = CorsMiddleware::permissive();
    /// let handler = Arc::new(TestHandler);
    ///
    // Preflight request
    /// let request = Request::new(
    ///     Method::OPTIONS,
    ///     Uri::from_static("/api/users"),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::new(),
    /// );
    ///
    /// let response = middleware.process(request, handler).await.unwrap();
    /// assert_eq!(response.status, StatusCode::NO_CONTENT);
    /// assert!(response.headers.contains_key("Access-Control-Allow-Origin"));
    /// assert!(response.headers.contains_key("Access-Control-Allow-Methods"));
    /// # });
    /// ```
    pub fn permissive() -> Self {
        Self::new(CorsConfig::default())
    }
}

#[async_trait]
impl Middleware for CorsMiddleware {
    async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
        // Handle preflight OPTIONS request
        if request.method.as_str() == "OPTIONS" {
            let mut response = Response::no_content();

            response.headers.insert(
                hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                hyper::header::HeaderValue::from_str(&self.config.allow_origins.join(", "))
                    .unwrap_or_else(|_| hyper::header::HeaderValue::from_static("*")),
            );

            response.headers.insert(
                hyper::header::ACCESS_CONTROL_ALLOW_METHODS,
                hyper::header::HeaderValue::from_str(&self.config.allow_methods.join(", "))
                    .unwrap_or_else(|_| hyper::header::HeaderValue::from_static("*")),
            );

            response.headers.insert(
                hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
                hyper::header::HeaderValue::from_str(&self.config.allow_headers.join(", "))
                    .unwrap_or_else(|_| hyper::header::HeaderValue::from_static("*")),
            );

            if let Some(max_age) = self.config.max_age {
                response.headers.insert(
                    hyper::header::ACCESS_CONTROL_MAX_AGE,
                    hyper::header::HeaderValue::from_str(&max_age.to_string())
                        .unwrap_or_else(|_| hyper::header::HeaderValue::from_static("3600")),
                );
            }

            if self.config.allow_credentials {
                response.headers.insert(
                    hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                    hyper::header::HeaderValue::from_static("true"),
                );
            }

            return Ok(response);
        }

        // Process request and add CORS headers to response
        let mut response = next.handle(request).await?;

        response.headers.insert(
            hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
            hyper::header::HeaderValue::from_str(&self.config.allow_origins.join(", "))
                .unwrap_or_else(|_| hyper::header::HeaderValue::from_static("*")),
        );

        if self.config.allow_credentials {
            response.headers.insert(
                hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                hyper::header::HeaderValue::from_static("true"),
            );
        }

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
            Ok(Response::new(StatusCode::OK).with_body(Bytes::from("test response")))
        }
    }

    #[tokio::test]
    async fn test_preflight_request() {
        let config = CorsConfig {
            allow_origins: vec!["https://example.com".to_string()],
            allow_methods: vec!["GET".to_string(), "POST".to_string()],
            allow_headers: vec!["Content-Type".to_string()],
            allow_credentials: true,
            max_age: Some(7200),
        };
        let middleware = CorsMiddleware::new(config);
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::OPTIONS,
            Uri::from_static("/api/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        // Preflight returns 204 No Content
        assert_eq!(response.status, StatusCode::NO_CONTENT);

        // CORS headers should be present
        assert!(response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN));
        assert!(response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_ALLOW_METHODS));
        assert!(response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_ALLOW_HEADERS));
        assert!(response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_MAX_AGE));
        assert!(response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS));

        // Check max age value
        assert_eq!(
            response
                .headers
                .get(hyper::header::ACCESS_CONTROL_MAX_AGE)
                .unwrap(),
            "7200"
        );

        // Check credentials header
        assert_eq!(
            response
                .headers
                .get(hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .unwrap(),
            "true"
        );
    }

    #[tokio::test]
    async fn test_regular_request_with_cors_headers() {
        let config = CorsConfig {
            allow_origins: vec!["https://app.example.com".to_string()],
            allow_methods: vec!["GET".to_string()],
            allow_headers: vec!["Authorization".to_string()],
            allow_credentials: false,
            max_age: None,
        };
        let middleware = CorsMiddleware::new(config);
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::GET,
            Uri::from_static("/api/data"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        // Regular request returns handler's status
        assert_eq!(response.status, StatusCode::OK);

        // Response body should be preserved
        assert_eq!(response.body, Bytes::from("test response"));

        // CORS origin header should be present
        assert_eq!(
            response
                .headers
                .get(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "https://app.example.com"
        );

        // Credentials header should not be present (allow_credentials = false)
        assert!(!response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS));
    }

    #[tokio::test]
    async fn test_permissive_mode() {
        let middleware = CorsMiddleware::permissive();
        let handler = Arc::new(TestHandler);

        // Test OPTIONS request
        let request = Request::new(
            Method::OPTIONS,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler.clone()).await.unwrap();

        assert_eq!(response.status, StatusCode::NO_CONTENT);
        assert!(response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN));
        assert!(response
            .headers
            .contains_key(hyper::header::ACCESS_CONTROL_ALLOW_METHODS));

        // Permissive mode should allow common methods
        let methods_header = response
            .headers
            .get(hyper::header::ACCESS_CONTROL_ALLOW_METHODS)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(methods_header.contains("GET"));
        assert!(methods_header.contains("POST"));
        assert!(methods_header.contains("PUT"));
        assert!(methods_header.contains("DELETE"));
    }

    #[tokio::test]
    async fn test_multiple_origins() {
        let config = CorsConfig {
            allow_origins: vec![
                "https://app1.example.com".to_string(),
                "https://app2.example.com".to_string(),
            ],
            allow_methods: vec!["GET".to_string()],
            allow_headers: vec!["Content-Type".to_string()],
            allow_credentials: true,
            max_age: Some(3600),
        };
        let middleware = CorsMiddleware::new(config);
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::GET,
            Uri::from_static("/api/resource"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();

        // Multiple origins should be joined with comma
        let origin_header = response
            .headers
            .get(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(origin_header.contains("https://app1.example.com"));
        assert!(origin_header.contains("https://app2.example.com"));

        // Credentials header should be present
        assert_eq!(
            response
                .headers
                .get(hyper::header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .unwrap(),
            "true"
        );
    }
}
