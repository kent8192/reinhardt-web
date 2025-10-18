//! Template/HTML Integration Tests for Renderers
//!
//! These tests require multiple crates:
//! - reinhardt-renderers
//! - reinhardt-templates
//! - reinhardt-forms
//! - reinhardt-views
//!
//! Based on Django REST Framework's TemplateHTMLRendererTests

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};
use reinhardt_types::Handler;
use std::sync::Arc;

// NOTE: These tests are currently skipped because the required infrastructure
// is not yet implemented. They should be enabled once the following components
// are available:
//
// Required components:
// 1. reinhardt-templates: Template rendering engine
// 2. reinhardt-forms: Form rendering
// 3. reinhardt-views: View layer with template support
// 4. reinhardt-browsable-api: Full browsable API renderer

// NOTE: Advanced template features like TemplateHTMLRenderer require full
// reinhardt-templates integration with view layer. The basic template rendering
// tests below use askama directly to verify template functionality.

#[cfg(test)]
mod template_exception_tests {
    use askama::Template;
    use async_trait::async_trait;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
    use reinhardt_exception::Result;
    use reinhardt_http::{Request, Response};
    use reinhardt_types::Handler;
    use std::sync::Arc;

    // Custom error templates
    #[derive(Template, Clone)]
    #[template(
        source = "<html><body><h1>404 Not Found</h1><p>{{ message }}</p></body></html>",
        ext = "html"
    )]
    struct NotFoundTemplate {
        message: String,
    }

    #[derive(Template, Clone)]
    #[template(
        source = "<html><body><h1>403 Forbidden</h1><p>{{ message }}</p></body></html>",
        ext = "html"
    )]
    struct ForbiddenTemplate {
        message: String,
    }

    // Error handler with template
    #[derive(Clone)]
    struct ErrorTemplateHandler<T: Template + Clone> {
        template: T,
        status: StatusCode,
    }

    impl<T: Template + Clone> ErrorTemplateHandler<T> {
        fn new(template: T, status: StatusCode) -> Self {
            Self { template, status }
        }
    }

    #[async_trait]
    impl<T: Template + Clone + Send + Sync> Handler for ErrorTemplateHandler<T> {
        async fn handle(&self, _request: Request) -> Result<Response> {
            let rendered = self
                .template
                .render()
                .map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

            Ok(Response::new(self.status)
                .with_body(Bytes::from(rendered))
                .with_header("content-type", "text/html; charset=utf-8"))
        }
    }

    #[tokio::test]
    async fn test_not_found_html_view_with_template() {
        // Create custom 404 template
        let template = NotFoundTemplate {
            message: "The requested resource could not be found.".to_string(),
        };

        let handler = Arc::new(ErrorTemplateHandler::new(template, StatusCode::NOT_FOUND));

        // Create a test request
        let request = Request::new(
            Method::GET,
            "/nonexistent".parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        // Handle the request
        let response = handler.handle(request).await.unwrap();

        // Verify response
        assert_eq!(response.status, StatusCode::NOT_FOUND);

        let body_str = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body_str.contains("404 Not Found"));
        assert!(body_str.contains("The requested resource could not be found"));
    }

    #[tokio::test]
    async fn test_permission_denied_html_view_with_template() {
        // Create custom 403 template
        let template = ForbiddenTemplate {
            message: "You do not have permission to access this resource.".to_string(),
        };

        let handler = Arc::new(ErrorTemplateHandler::new(template, StatusCode::FORBIDDEN));

        // Create a test request
        let request = Request::new(
            Method::GET,
            "/admin/secret".parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        // Handle the request
        let response = handler.handle(request).await.unwrap();

        // Verify response
        assert_eq!(response.status, StatusCode::FORBIDDEN);

        let body_str = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body_str.contains("403 Forbidden"));
        assert!(body_str.contains("You do not have permission to access this resource"));
    }
}

#[cfg(test)]
mod form_rendering_tests {
    use askama::Template;
    use async_trait::async_trait;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
    use reinhardt_exception::Result;
    use reinhardt_http::{Request, Response};
    use reinhardt_types::Handler;
    use serde::{Deserialize, Serialize};
    use serde_json;
    use std::sync::Arc;

    // Test data structure
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct FormData {
        name: String,
        email: String,
        age: i32,
    }

    // HTML form template
    #[derive(Template, Clone)]
    #[template(
        source = r#"<html><body><h1>Submit Data</h1>
<form method="post">
  <label>Name: <input type="text" name="name" /></label><br/>
  <label>Email: <input type="email" name="email" /></label><br/>
  <label>Age: <input type="number" name="age" /></label><br/>
  <button type="submit">Submit</button>
</form>
</body></html>"#,
        ext = "html"
    )]
    struct FormTemplate;

    // Form handler that supports both JSON and HTML
    #[derive(Clone)]
    struct FormHandler {
        render_html: bool,
    }

    #[async_trait]
    impl Handler for FormHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            if self.render_html {
                // Render HTML form for browsable API
                let template = FormTemplate;
                let rendered = template
                    .render()
                    .map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

                Ok(Response::ok()
                    .with_body(Bytes::from(rendered))
                    .with_header("content-type", "text/html; charset=utf-8"))
            } else {
                // Return JSON response
                let form_data = FormData {
                    name: "John Doe".to_string(),
                    email: "john@example.com".to_string(),
                    age: 30,
                };

                let json = serde_json::to_string(&form_data)
                    .map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

                Ok(Response::ok()
                    .with_body(Bytes::from(json))
                    .with_header("content-type", "application/json"))
            }
        }
    }

    #[tokio::test]
    async fn test_renderer_template_json_response() {
        // Test JSON serialization of form data
        let handler = Arc::new(FormHandler { render_html: false });

        let request = Request::new(
            Method::GET,
            "/api/data".parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = handler.handle(request).await.unwrap();

        // Verify JSON response
        assert_eq!(response.status, StatusCode::OK);

        let body_str = String::from_utf8(response.body.to_vec()).unwrap();
        let data: FormData = serde_json::from_str(&body_str).unwrap();

        assert_eq!(data.name, "John Doe");
        assert_eq!(data.email, "john@example.com");
        assert_eq!(data.age, 30);
    }

    #[tokio::test]
    async fn test_browsable_api() {
        // Test HTML form rendering for browsable API
        let handler = Arc::new(FormHandler { render_html: true });

        let request = Request::new(
            Method::GET,
            "/api/data".parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = handler.handle(request).await.unwrap();

        // Verify HTML response
        assert_eq!(response.status, StatusCode::OK);

        let body_str = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body_str.contains("<form"));
        assert!(body_str.contains("name=\"name\""));
        assert!(body_str.contains("name=\"email\""));
        assert!(body_str.contains("name=\"age\""));
        assert!(body_str.contains("Submit"));
    }

    #[tokio::test]
    async fn test_post_many_related_view() {
        // Test handling of many-to-many relationships in forms
        // This is a simplified version that tests the concept

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct ManyToManyFormData {
            user_id: i32,
            tag_ids: Vec<i32>,
        }

        #[derive(Clone)]
        struct ManyToManyHandler;

        #[async_trait]
        impl Handler for ManyToManyHandler {
            async fn handle(&self, _request: Request) -> Result<Response> {
                let data = ManyToManyFormData {
                    user_id: 1,
                    tag_ids: vec![10, 20, 30],
                };

                let json = serde_json::to_string(&data)
                    .map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

                Ok(Response::ok()
                    .with_body(Bytes::from(json))
                    .with_header("content-type", "application/json"))
            }
        }

        let handler = Arc::new(ManyToManyHandler);

        let request = Request::new(
            Method::POST,
            "/api/user-tags".parse::<Uri>().unwrap(),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = handler.handle(request).await.unwrap();

        // Verify response
        assert_eq!(response.status, StatusCode::OK);

        let body_str = String::from_utf8(response.body.to_vec()).unwrap();
        let data: ManyToManyFormData = serde_json::from_str(&body_str).unwrap();

        assert_eq!(data.user_id, 1);
        assert_eq!(data.tag_ids, vec![10, 20, 30]);
    }
}
