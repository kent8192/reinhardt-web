//! Admin Renderer
//!
//! Renderer for Django-like admin interface responses.
//! Based on Django REST Framework's AdminRenderer.

use crate::renderer::{RenderResult, Renderer, RendererContext};
use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value;

/// Renderer for admin interface responses
///
/// This renderer formats data for admin-style interfaces,
/// handling resource creation confirmations, CRUD operations,
/// and dictionary data display.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::{AdminRenderer, Renderer};
/// use serde_json::json;
///
/// let renderer = AdminRenderer::new();
/// assert_eq!(renderer.media_types(), vec!["text/html"]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct AdminRenderer {
    /// Base URL for admin interface
    pub base_url: String,
}

impl AdminRenderer {
    /// Creates a new admin renderer
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_renderers::AdminRenderer;
    ///
    /// let renderer = AdminRenderer::new();
    /// assert_eq!(renderer.base_url, "/admin");
    /// ```
    pub fn new() -> Self {
        Self {
            base_url: "/admin".to_string(),
        }
    }

    /// Sets the base URL for the admin interface
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_renderers::AdminRenderer;
    ///
    /// let renderer = AdminRenderer::new().base_url("/custom-admin");
    /// assert_eq!(renderer.base_url, "/custom-admin");
    /// ```
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Generates a result URL for a created/updated resource
    fn get_result_url(&self, data: &Value) -> Option<String> {
        // Check if data has an 'id' field to construct a detail URL
        if let Some(id) = data.get("id") {
            if let Some(id_str) = id.as_str() {
                return Some(format!("{}/{}", self.base_url, id_str));
            } else if let Some(id_num) = id.as_i64() {
                return Some(format!("{}/{}", self.base_url, id_num));
            }
        }
        None
    }

    /// Renders admin HTML for the given data
    fn render_admin_html(&self, data: &Value, _context: Option<&RendererContext>) -> String {
        let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Admin Interface</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str(".success { color: green; font-weight: bold; }\n");
        html.push_str(".data-table { border-collapse: collapse; width: 100%; }\n");
        html.push_str(".data-table th, .data-table td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str(".data-table th { background-color: #f2f2f2; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        // Check if this is a resource creation response
        if let Some(result_url) = self.get_result_url(data) {
            html.push_str("<div class=\"success\">Resource created successfully!</div>\n");
            html.push_str(&format!(
                "<p>View at: <a href=\"{}\">{}</a></p>\n",
                result_url, result_url
            ));
        }

        // Render data as a table
        html.push_str("<h2>Data</h2>\n");
        html.push_str("<table class=\"data-table\">\n");

        match data {
            Value::Object(map) => {
                html.push_str("<thead><tr><th>Field</th><th>Value</th></tr></thead>\n<tbody>\n");
                for (key, value) in map {
                    let value_str = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        Value::Array(_) | Value::Object(_) => {
                            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
                        }
                    };
                    html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td></tr>\n",
                        key, value_str
                    ));
                }
                html.push_str("</tbody>\n");
            }
            Value::Array(arr) => {
                // For arrays, render each item as a row
                if let Some(first) = arr.first() {
                    if let Value::Object(obj) = first {
                        // Use keys from first object as headers
                        html.push_str("<thead><tr>");
                        for key in obj.keys() {
                            html.push_str(&format!("<th>{}</th>", key));
                        }
                        html.push_str("</tr></thead>\n<tbody>\n");
                    }
                }

                for item in arr {
                    if let Value::Object(obj) = item {
                        html.push_str("<tr>");
                        for value in obj.values() {
                            let value_str = match value {
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "null".to_string(),
                                _ => serde_json::to_string(value)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            };
                            html.push_str(&format!("<td>{}</td>", value_str));
                        }
                        html.push_str("</tr>\n");
                    }
                }
                html.push_str("</tbody>\n");
            }
            _ => {
                html.push_str("<tbody><tr><td colspan=\"2\">");
                html.push_str(
                    &serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string()),
                );
                html.push_str("</td></tr></tbody>\n");
            }
        }

        html.push_str("</table>\n");
        html.push_str("</body>\n</html>");
        html
    }
}

#[async_trait]
impl Renderer for AdminRenderer {
    fn media_types(&self) -> Vec<String> {
        vec!["text/html".to_string()]
    }

    fn format(&self) -> Option<&str> {
        Some("admin")
    }

    async fn render(&self, data: &Value, context: Option<&RendererContext>) -> RenderResult<Bytes> {
        let html = self.render_admin_html(data, context);
        Ok(Bytes::from(html))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_admin_renderer_basic() {
        let renderer = AdminRenderer::new();
        let data = json!({"name": "Test Item", "value": 123});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("Admin Interface"));
        assert!(html.contains("Test Item"));
        assert!(html.contains("123"));
    }

    #[tokio::test]
    async fn test_admin_renderer_with_id() {
        let renderer = AdminRenderer::new();
        let data = json!({"id": "42", "name": "Created Item"});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("Resource created successfully"));
        assert!(html.contains("/admin/42"));
    }

    #[tokio::test]
    async fn test_admin_renderer_custom_base_url() {
        let renderer = AdminRenderer::new().base_url("/custom");
        let data = json!({"id": 5, "name": "Item"});

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("/custom/5"));
    }

    #[tokio::test]
    async fn test_admin_renderer_array_data() {
        let renderer = AdminRenderer::new();
        let data = json!([
            {"id": 1, "name": "Item 1"},
            {"id": 2, "name": "Item 2"}
        ]);

        let result = renderer.render(&data, None).await.unwrap();
        let html = String::from_utf8(result.to_vec()).unwrap();

        assert!(html.contains("Item 1"));
        assert!(html.contains("Item 2"));
        assert!(html.contains("<table"));
    }

    #[tokio::test]
    async fn test_admin_renderer_media_types() {
        let renderer = AdminRenderer::new();
        assert_eq!(renderer.media_types(), vec!["text/html"]);
    }

    #[tokio::test]
    async fn test_admin_renderer_format() {
        let renderer = AdminRenderer::new();
        assert_eq!(renderer.format(), Some("admin"));
    }
}
