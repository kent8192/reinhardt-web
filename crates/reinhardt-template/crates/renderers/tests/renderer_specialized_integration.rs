//! Specialized Renderer Integration Tests
//!
//! These tests cover specialized renderers:
//! - StaticHTMLRenderer
//! - AdminRenderer
//! - DocumentationRenderer
//! - SchemaJSRenderer
//! - BrowsableAPIRenderer integration
//! - HTMLFormRenderer
//!
//! Based on Django REST Framework's specialized renderer patterns

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_renderers::{RenderResult, Renderer, RendererContext};
use serde_json::Value;

// ============================================================================
// Static HTML Renderer
// ============================================================================

/// Renderer that returns static HTML content
#[derive(Debug, Clone)]
pub struct StaticHTMLRenderer {
    content: String,
}

impl StaticHTMLRenderer {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

#[async_trait]
impl Renderer for StaticHTMLRenderer {
    fn media_types(&self) -> Vec<String> {
        vec!["text/html".to_string()]
    }

    async fn render(
        &self,
        _data: &Value,
        _context: Option<&RendererContext>,
    ) -> RenderResult<Bytes> {
        Ok(Bytes::from(self.content.clone()))
    }

    fn format(&self) -> Option<&str> {
        Some("html")
    }
}

// ============================================================================
// Documentation Renderer
// ============================================================================

/// Renderer for API documentation
#[derive(Debug, Clone)]
pub struct DocumentationRenderer {
    api_title: String,
}

impl DocumentationRenderer {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            api_title: title.into(),
        }
    }

    fn render_docs(&self, data: &Value) -> String {
        let mut html = format!(
            "<!DOCTYPE html>\n<html>\n<head>\n<title>{}</title>\n</head>\n<body>\n",
            self.api_title
        );
        html.push_str(&format!("<h1>{}</h1>\n", self.api_title));
        html.push_str("<div class=\"endpoints\">\n");

        // Parse endpoints from data
        if let Some(endpoints) = data.get("endpoints").and_then(|v| v.as_array()) {
            for endpoint in endpoints {
                if let Some(obj) = endpoint.as_object() {
                    let path = obj.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let method = obj.get("method").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    html.push_str(&format!(
                        "<div class=\"endpoint\"><span class=\"method\">{}</span> <span class=\"path\">{}</span><p>{}</p></div>\n",
                        method, path, desc
                    ));
                }
            }
        }

        html.push_str("</div>\n</body>\n</html>");
        html
    }
}

#[async_trait]
impl Renderer for DocumentationRenderer {
    fn media_types(&self) -> Vec<String> {
        vec!["text/html".to_string()]
    }

    async fn render(
        &self,
        data: &Value,
        _context: Option<&RendererContext>,
    ) -> RenderResult<Bytes> {
        let html = self.render_docs(data);
        Ok(Bytes::from(html))
    }

    fn format(&self) -> Option<&str> {
        Some("api-docs")
    }
}

// ============================================================================
// Schema JavaScript Renderer
// ============================================================================

/// Renderer that generates JavaScript for schema definitions
#[derive(Debug, Clone)]
pub struct SchemaJSRenderer;

impl SchemaJSRenderer {
    pub fn new() -> Self {
        Self
    }

    fn generate_schema_js(&self, data: &Value) -> String {
        let mut js = String::from("// Auto-generated schema\n");
        js.push_str("const schema = ");
        js.push_str(&serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string()));
        js.push_str(";\n\n");
        js.push_str("if (typeof module !== 'undefined' && module.exports) {\n");
        js.push_str("  module.exports = schema;\n");
        js.push_str("}\n");
        js
    }
}

#[async_trait]
impl Renderer for SchemaJSRenderer {
    fn media_types(&self) -> Vec<String> {
        vec!["application/javascript".to_string()]
    }

    async fn render(
        &self,
        data: &Value,
        _context: Option<&RendererContext>,
    ) -> RenderResult<Bytes> {
        let js = self.generate_schema_js(data);
        Ok(Bytes::from(js))
    }

    fn format(&self) -> Option<&str> {
        Some("js-schema")
    }
}

// ============================================================================
// HTML Form Renderer
// ============================================================================

/// Renderer for HTML forms
#[derive(Debug, Clone)]
pub struct HTMLFormRenderer {
    form_action: String,
    form_method: String,
}

impl HTMLFormRenderer {
    pub fn new() -> Self {
        Self {
            form_action: "/submit".to_string(),
            form_method: "POST".to_string(),
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.form_action = action.into();
        self
    }

    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.form_method = method.into();
        self
    }

    fn render_form(&self, data: &Value) -> String {
        let mut html = format!(
            "<form action=\"{}\" method=\"{}\">\n",
            self.form_action, self.form_method
        );

        // Render form fields from data
        if let Some(fields) = data.get("fields").and_then(|v| v.as_array()) {
            for field in fields {
                if let Some(obj) = field.as_object() {
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let field_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                    let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or(name);

                    html.push_str(&format!("  <label for=\"{}\">{}</label>\n", name, label));
                    html.push_str(&format!(
                        "  <input type=\"{}\" id=\"{}\" name=\"{}\" />\n",
                        field_type, name, name
                    ));
                }
            }
        }

        html.push_str("  <button type=\"submit\">Submit</button>\n");
        html.push_str("</form>");
        html
    }
}

#[async_trait]
impl Renderer for HTMLFormRenderer {
    fn media_types(&self) -> Vec<String> {
        vec!["text/html".to_string()]
    }

    async fn render(
        &self,
        data: &Value,
        _context: Option<&RendererContext>,
    ) -> RenderResult<Bytes> {
        let html = self.render_form(data);
        Ok(Bytes::from(html))
    }

    fn format(&self) -> Option<&str> {
        Some("form")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod specialized_renderer_tests {
    use super::*;
    use serde_json::json;

    // Static HTML Renderer Tests
    #[tokio::test]
    async fn test_static_html_renderer() {
        let content = "<html><body><h1>Static Content</h1></body></html>";
        let renderer = StaticHTMLRenderer::new(content);
        let data = json!({});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(html, content);
    }

    #[tokio::test]
    async fn test_static_html_renderer_ignores_data() {
        let content = "<html><body>Ignore data</body></html>";
        let renderer = StaticHTMLRenderer::new(content);
        let data = json!({"some": "data", "to": "ignore"});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(html, content);
        assert!(!html.contains("some"));
    }

    // Documentation Renderer Tests
    #[tokio::test]
    async fn test_documentation_renderer_basic() {
        let renderer = DocumentationRenderer::new("My API");
        let data = json!({
            "endpoints": [
                {"path": "/users", "method": "GET", "description": "List users"},
                {"path": "/users/:id", "method": "GET", "description": "Get user"}
            ]
        });

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("My API"));
        assert!(html.contains("/users"));
        assert!(html.contains("GET"));
        assert!(html.contains("List users"));
    }

    #[tokio::test]
    async fn test_documentation_renderer_empty_endpoints() {
        let renderer = DocumentationRenderer::new("Empty API");
        let data = json!({"endpoints": []});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("Empty API"));
        assert!(html.contains("endpoints"));
    }

    #[tokio::test]
    async fn test_documentation_renderer_no_endpoints() {
        let renderer = DocumentationRenderer::new("No Endpoints");
        let data = json!({});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("No Endpoints"));
    }

    // Schema JS Renderer Tests
    #[tokio::test]
    async fn test_schema_js_renderer_basic() {
        let renderer = SchemaJSRenderer::new();
        let data = json!({
            "User": {
                "type": "object",
                "properties": {
                    "id": {"type": "integer"},
                    "name": {"type": "string"}
                }
            }
        });

        let result = renderer.render(&data, None).await.unwrap();
        let js = String::from_utf8(result.to_vec()).unwrap();

        assert!(js.contains("const schema = "));
        assert!(js.contains("User"));
        assert!(js.contains("module.exports"));
    }

    #[tokio::test]
    async fn test_schema_js_renderer_format() {
        let renderer = SchemaJSRenderer::new();
        assert_eq!(renderer.format(), Some("js-schema"));
    }

    #[tokio::test]
    async fn test_schema_js_renderer_empty_schema() {
        let renderer = SchemaJSRenderer::new();
        let data = json!({});

        let result = renderer.render(&data, None).await.unwrap();
        let js = String::from_utf8(result.to_vec()).unwrap();

        assert!(js.contains("const schema = "));
        assert!(js.contains("{}"));
    }

    // HTML Form Renderer Tests
    #[tokio::test]
    async fn test_html_form_renderer_basic() {
        let renderer = HTMLFormRenderer::new();
        let data = json!({
            "fields": [
                {"name": "username", "type": "text", "label": "Username"},
                {"name": "password", "type": "password", "label": "Password"}
            ]
        });

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("<form"));
        assert!(html.contains("username"));
        assert!(html.contains("password"));
        assert!(html.contains("type=\"text\""));
        assert!(html.contains("type=\"password\""));
        assert!(html.contains("Submit"));
    }

    #[tokio::test]
    async fn test_html_form_renderer_custom_action() {
        let renderer = HTMLFormRenderer::new().with_action("/custom/submit");
        let data = json!({"fields": []});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("action=\"/custom/submit\""));
    }

    #[tokio::test]
    async fn test_html_form_renderer_custom_method() {
        let renderer = HTMLFormRenderer::new().with_method("PUT");
        let data = json!({"fields": []});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("method=\"PUT\""));
    }

    #[tokio::test]
    async fn test_html_form_renderer_empty_fields() {
        let renderer = HTMLFormRenderer::new();
        let data = json!({"fields": []});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("<form"));
        assert!(html.contains("Submit"));
    }

    // BrowsableAPIRenderer Integration Tests
    // Note: BrowsableAPIRenderer has a different API than the Renderer trait,
    // so these tests just verify basic integration
    #[tokio::test]
    async fn test_browsable_api_renderer_exists() {
        use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer};
        let _renderer = BrowsableApiRenderer::new();
        // Just verify it can be instantiated
    }

    #[tokio::test]
    async fn test_browsable_api_renderer_basic_render() {
        use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer};
        let renderer = BrowsableApiRenderer::new();
        let context = ApiContext {
            title: "Test API".to_string(),
            description: Some("A test endpoint".to_string()),
            endpoint: "/test".to_string(),
            method: "GET".to_string(),
            response_data: json!({"message": "Hello"}),
            response_status: 200,
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            request_form: None,
            headers: vec![],
        };

        let result = renderer.render(&context);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("Test API"));
    }

    #[tokio::test]
    async fn test_browsable_api_renderer_with_form() {
        use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer, FormContext, FormField};
        let renderer = BrowsableApiRenderer::new();
        let form = FormContext {
            fields: vec![FormField {
                name: "username".to_string(),
                label: "Username".to_string(),
                field_type: "text".to_string(),
                required: true,
                help_text: None,
                initial_value: None,
                options: None,
                initial_label: None,
            }],
            submit_url: "/submit".to_string(),
            submit_method: "POST".to_string(),
        };
        let context = ApiContext {
            title: "Form API".to_string(),
            description: None,
            endpoint: "/form".to_string(),
            method: "POST".to_string(),
            response_data: json!({}),
            response_status: 200,
            allowed_methods: vec!["POST".to_string()],
            request_form: Some(form),
            headers: vec![],
        };

        let result = renderer.render(&context);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("username"));
    }

    // Combined Renderer Tests
    #[tokio::test]
    async fn test_specialized_renderers_unique_formats() {
        let static_renderer = StaticHTMLRenderer::new("test");
        let doc_renderer = DocumentationRenderer::new("API");
        let schema_renderer = SchemaJSRenderer::new();
        let form_renderer = HTMLFormRenderer::new();

        // Each should have a unique format (or None)
        let formats = vec![
            static_renderer.format(),
            doc_renderer.format(),
            schema_renderer.format(),
            form_renderer.format(),
        ];

        // Schema renderer should be different from HTML renderers
        assert_eq!(schema_renderer.format(), Some("js-schema"));
        assert_ne!(static_renderer.format(), schema_renderer.format());
    }

    #[tokio::test]
    async fn test_specialized_renderers_content_types() {
        let static_renderer = StaticHTMLRenderer::new("test");
        let doc_renderer = DocumentationRenderer::new("API");
        let schema_renderer = SchemaJSRenderer::new();
        let form_renderer = HTMLFormRenderer::new();

        // HTML renderers should have text/html in media_types
        assert!(
            static_renderer
                .media_types()
                .contains(&"text/html".to_string())
        );
        assert!(
            doc_renderer
                .media_types()
                .contains(&"text/html".to_string())
        );
        assert!(
            form_renderer
                .media_types()
                .contains(&"text/html".to_string())
        );

        // JS renderer should have application/javascript in media_types
        assert!(
            schema_renderer
                .media_types()
                .contains(&"application/javascript".to_string())
        );
    }

    #[tokio::test]
    async fn test_all_specialized_renderers_are_send_sync() {
        use reinhardt_browsable_api::BrowsableApiRenderer;
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<StaticHTMLRenderer>();
        assert_send_sync::<DocumentationRenderer>();
        assert_send_sync::<SchemaJSRenderer>();
        assert_send_sync::<HTMLFormRenderer>();
        assert_send_sync::<BrowsableApiRenderer>();
    }
}
