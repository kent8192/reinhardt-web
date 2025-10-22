//! HTTP/View Integration Tests for Renderers
//!
//! Integration tests for reinhardt-renderers working with reinhardt-http,
//! reinhardt-negotiation, and reinhardt-views. These tests verify that
//! the renderer system integrates properly with HTTP request/response
//! handling, content negotiation, and view classes.
//!
//! These tests integrate multiple crates:
//! - reinhardt-renderers
//! - reinhardt-http
//! - reinhardt-negotiation
//! - reinhardt-views
//!
//! Based on Django REST Framework's RendererEndToEndTests

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_http::{Request, Response};
use reinhardt_negotiation::{ContentNegotiator, MediaType};
use reinhardt_renderers::{JSONRenderer, Renderer, RendererContext, RendererRegistry};
use reinhardt_views::View;
use serde_json::json;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a test request with specified method and headers
fn create_test_request(method: &str, headers: Vec<(&str, &str)>) -> Request {
    let method = method.parse::<Method>().expect("Invalid HTTP method");
    let uri = "/api/test".parse::<Uri>().expect("Invalid URI");
    let version = Version::HTTP_11;
    let mut header_map = HeaderMap::new();

    for (key, value) in headers {
        if let Ok(header_name) = key.parse::<hyper::header::HeaderName>() {
            if let Ok(header_value) = value.parse::<hyper::header::HeaderValue>() {
                header_map.insert(header_name, header_value);
            }
        }
    }

    let body = Bytes::new();

    Request::new(method, uri, version, header_map, body)
}

/// Create a renderer registry with JSON and custom renderers
fn create_renderer_registry() -> RendererRegistry {
    RendererRegistry::new()
        .register(JSONRenderer::new())
        .register(CustomHTMLRenderer::new())
}

// ============================================================================
// Custom Renderer for Testing
// ============================================================================

/// Custom HTML renderer for testing content negotiation
#[derive(Debug, Clone)]
struct CustomHTMLRenderer;

impl CustomHTMLRenderer {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Renderer for CustomHTMLRenderer {
    fn media_types(&self) -> Vec<String> {
        vec!["text/html".to_string()]
    }

    fn format(&self) -> Option<&str> {
        Some("html")
    }

    async fn render(
        &self,
        data: &serde_json::Value,
        _context: Option<&RendererContext>,
    ) -> reinhardt_renderers::RenderResult<bytes::Bytes> {
        let html = format!(
            "<!DOCTYPE html><html><body><pre>{}</pre></body></html>",
            serde_json::to_string_pretty(data)
                .map_err(|e| reinhardt_renderers::RenderError::SerializationError(e.to_string()))?
        );
        Ok(bytes::Bytes::from(html))
    }
}

// ============================================================================
// Test API View with Content Negotiation
// ============================================================================

struct TestAPIView {
    data: serde_json::Value,
    renderer_registry: RendererRegistry,
    negotiator: ContentNegotiator,
    allowed_methods: Vec<String>,
}

impl TestAPIView {
    fn new(data: serde_json::Value) -> Self {
        Self {
            data,
            renderer_registry: create_renderer_registry(),
            negotiator: ContentNegotiator::new(),
            allowed_methods: vec!["GET".to_string(), "HEAD".to_string(), "POST".to_string()],
        }
    }

    fn with_allowed_methods(mut self, methods: Vec<&str>) -> Self {
        self.allowed_methods = methods.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Select renderer based on Accept header or format parameter
    fn select_renderer(
        &self,
        request: &Request,
    ) -> Result<&dyn Renderer, Box<dyn std::error::Error>> {
        // Check for format query parameter first (higher priority)
        if let Some(format) = request.query_params.get("format") {
            if let Some(renderer) = self.renderer_registry.get_renderer(Some(format)) {
                return Ok(renderer);
            }
        }

        // Fall back to Accept header negotiation
        let accept_header = request
            .headers
            .get("Accept")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("*/*");
        let available_media_types: Vec<MediaType> = vec![
            MediaType::new("application", "json"),
            MediaType::new("text", "html"),
        ];

        let (media_type, _) = self
            .negotiator
            .select_renderer(Some(accept_header), &available_media_types)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        self.renderer_registry
            .get_renderer_by_media_type(&media_type.to_string())
            .ok_or_else(|| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No suitable renderer found",
                )) as Box<dyn std::error::Error>
            })
    }
}

#[async_trait]
impl View for TestAPIView {
    async fn dispatch(&self, request: Request) -> reinhardt_http::Result<Response> {
        // Check if method is allowed
        if !self.allowed_methods.contains(&request.method.to_string()) {
            return Ok(Response::new(hyper::StatusCode::METHOD_NOT_ALLOWED));
        }

        // Select renderer based on content negotiation
        let renderer = match self.select_renderer(&request) {
            Ok(r) => r,
            Err(_) => {
                // Return 406 Not Acceptable
                return Ok(Response::new(hyper::StatusCode::NOT_ACCEPTABLE));
            }
        };

        // Render the data
        let context = RendererContext::new()
            .with_request(request.method.as_str(), request.uri.path())
            .with_view("TestAPIView", "Test view for renderer integration");

        let rendered = renderer
            .render(&self.data, Some(&context))
            .await
            .map_err(|e| reinhardt_http::Error::Serialization(e.to_string()))?;

        // Handle HEAD requests - return headers but no body
        let response = if request.method == "HEAD" {
            Response::new(hyper::StatusCode::OK)
                .with_typed_header(
                    hyper::header::CONTENT_TYPE,
                    hyper::header::HeaderValue::from_str(&renderer.content_type()).unwrap(),
                )
                .with_typed_header(
                    hyper::header::CONTENT_LENGTH,
                    hyper::header::HeaderValue::from_str(&rendered.len().to_string()).unwrap(),
                )
        } else {
            Response::new(hyper::StatusCode::OK)
                .with_body(rendered)
                .with_typed_header(
                    hyper::header::CONTENT_TYPE,
                    hyper::header::HeaderValue::from_str(&renderer.content_type()).unwrap(),
                )
        };

        Ok(response)
    }
}

// ============================================================================
// HTTP/View Integration Tests
// ============================================================================

#[cfg(test)]
mod http_renderer_tests {
    use super::*;

    #[tokio::test]
    async fn test_default_renderer_serializes_content() {
        // Test: If the Accept header is not set, the default renderer should serialize the response.
        let view = TestAPIView::new(json!({"name": "test", "value": 123}));
        let request = create_test_request("GET", vec![]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(
            response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
            "application/json; charset=utf-8"
        );

        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("test"));
        assert!(body.contains("123"));
    }

    #[tokio::test]
    async fn test_renderers_head_no_body() {
        // Test: No response body must be included in HEAD requests.
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("HEAD", vec![("Accept", "application/json")]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(
            response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
            "application/json; charset=utf-8"
        );
        assert!(response
            .headers
            .get(hyper::header::CONTENT_LENGTH)
            .is_some());
        // Body should be empty for HEAD request
        assert_eq!(response.body.len(), 0);
    }

    #[tokio::test]
    async fn test_renderer_used_from_format_query_param() {
        // Test: If a 'format' query parameter is specified, the renderer with the matching
        // format attribute should serialize the response.
        let view = TestAPIView::new(json!({"name": "test"}));
        let mut request = create_test_request("GET", vec![("Accept", "text/html")]);

        // Add format query parameter
        request
            .query_params
            .insert("format".to_string(), "json".to_string());

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::OK);
        // Format parameter should override Accept header
        assert_eq!(
            response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
            "application/json; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn test_not_acceptable() {
        // Test: If the Accept header is unsatisfiable, we should return a 406 Not Acceptable response.
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "application/xml")]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::NOT_ACCEPTABLE);
    }

    #[tokio::test]
    async fn test_not_acceptable_if_no_renderer_found() {
        // Test: If no renderer is found, we should return a 406 Not Acceptable response.
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "image/png")]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::NOT_ACCEPTABLE);
    }

    #[tokio::test]
    async fn test_not_acceptable_on_bad_accept() {
        // Test: Malformed Accept headers should result in 406 or use default renderer
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "invalid/type/format")]);

        let response = view.dispatch(request).await.unwrap();

        // Should either return 406 or use default renderer
        assert!(
            response.status == hyper::StatusCode::NOT_ACCEPTABLE
                || response.status == hyper::StatusCode::OK
        );
    }

    #[tokio::test]
    async fn test_browser_format_in_accept() {
        // Test: Browser Accept headers (e.g., text/html) should work correctly
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "text/html")]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(
            response
                .headers
                .get(hyper::header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html; charset=utf-8"
        );

        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("<!DOCTYPE html>"));
    }

    #[tokio::test]
    async fn test_specified_renderer_serializes_content() {
        // Test: If the Accept header is set, the specified renderer should serialize the response.
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "application/json")]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::OK);
        assert_eq!(
            response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
            "application/json; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn test_underspecified_renderer_serializes_content() {
        // Test: If the Accept header is set to */*, the default renderer should serialize the response.
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "*/*")]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::OK);
        // Default renderer should be used (JSON)
        assert_eq!(
            response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
            "application/json; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn test_renderer_used_if_no_get_param() {
        // Test: Without format parameter, Accept header should be used
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "text/html")]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::OK);
        assert!(response
            .headers
            .get(hyper::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/html"));
    }

    #[tokio::test]
    async fn test_renderer_set_on_response() {
        // Test: Renderer should set correct Content-Type on response
        let view = TestAPIView::new(json!({"name": "test"}));
        let request = create_test_request("GET", vec![("Accept", "application/json")]);

        let response = view.dispatch(request).await.unwrap();

        assert!(response.headers.contains_key(hyper::header::CONTENT_TYPE));
        assert_eq!(
            response
                .headers
                .get(hyper::header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "application/json; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn test_method_not_allowed() {
        // Test: Methods not in allowed_methods should return 405
        let view = TestAPIView::new(json!({"name": "test"})).with_allowed_methods(vec!["GET"]);
        let request = create_test_request("POST", vec![]);

        let response = view.dispatch(request).await.unwrap();

        assert_eq!(response.status, hyper::StatusCode::METHOD_NOT_ALLOWED);
    }
}
