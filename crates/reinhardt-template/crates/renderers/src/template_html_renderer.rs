//! Template HTML Renderer
//!
//! Renders HTML responses using Askama templates.
//! Supports Django-style template rendering with context data.

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::{Error, Result};
use serde_json::Value;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// HTML renderer that uses Askama templates
///
/// This renderer integrates with the `reinhardt-templates` crate to provide
/// Django-style template rendering for HTML responses.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::TemplateHTMLRenderer;
/// use askama::Template;
/// use serde_json::json;
///
/// #[derive(Template)]
/// #[template(source = "<h1>{{ title }}</h1>", ext = "html")]
/// struct MyTemplate {
///     title: String,
/// }
///
/// let renderer = TemplateHTMLRenderer::new();
/// ```
pub struct TemplateHTMLRenderer {
    /// Default charset for HTML responses
    charset: String,
}

impl TemplateHTMLRenderer {
    /// Creates a new TemplateHTMLRenderer with UTF-8 charset
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_renderers::TemplateHTMLRenderer;
    ///
    /// let renderer = TemplateHTMLRenderer::new();
    /// assert_eq!(renderer.media_type(), "text/html; charset=utf-8");
    /// ```
    pub fn new() -> Self {
        Self {
            charset: "utf-8".to_string(),
        }
    }

    /// Creates a new TemplateHTMLRenderer with a custom charset
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_renderers::TemplateHTMLRenderer;
    ///
    /// let renderer = TemplateHTMLRenderer::with_charset("iso-8859-1");
    /// ```
    pub fn with_charset(charset: impl Into<String>) -> Self {
        Self {
            charset: charset.into(),
        }
    }

    /// Renders a template with the given context data
    ///
    /// The context should contain a special key "template_name" to identify
    /// which template to render, or "template_string" for inline templates.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_renderers::TemplateHTMLRenderer;
    /// use serde_json::json;
    ///
    /// # tokio_test::block_on(async {
    /// let renderer = TemplateHTMLRenderer::new();
    /// let context = json!({
    ///     "template_string": "<h1>{{ title }}</h1>",
    ///     "title": "Hello World"
    /// });
    ///
    /// let result = renderer.render(&context, None).await;
    /// assert!(result.is_ok());
    /// # });
    /// ```
    pub async fn render_template(&self, context: &Value) -> Result<String> {
        // For now, we support inline template strings only
        // Full file-based template support requires integration with FileSystemTemplateLoader

        if let Some(template_str) = context.get("template_string").and_then(|v| v.as_str()) {
            // Simple variable substitution for inline templates
            let mut output = template_str.to_string();

            // Replace {{ variable }} with values from context
            if let Value::Object(map) = context {
                for (key, value) in map {
                    if key == "template_string" || key == "template_name" {
                        continue;
                    }

                    let placeholder = format!("{{{{ {} }}}}", key);
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => String::new(),
                        _ => serde_json::to_string(value).unwrap_or_default(),
                    };

                    output = output.replace(&placeholder, &replacement);
                }
            }

            Ok(output)
        } else if let Some(template_name) = context.get("template_name").and_then(|v| v.as_str()) {
            // Template file loading would go here
            // For now, return an error indicating feature not yet implemented
            Err(Error::Internal(format!(
                "File-based template loading not yet implemented: {}",
                template_name
            )))
        } else {
            Err(Error::Internal(
                "No template_string or template_name provided in context".to_string(),
            ))
        }
    }
}

impl Default for TemplateHTMLRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Renderer for TemplateHTMLRenderer {
    fn media_types(&self) -> Vec<String> {
        vec![format!("text/html; charset={}", self.charset)]
    }

    async fn render(
        &self,
        data: &Value,
        _context: Option<&RendererContext>,
    ) -> RenderResult<Bytes> {
        let html = self.render_template(data).await?;
        Ok(Bytes::from(html))
    }

    fn format(&self) -> Option<&str> {
        Some("html")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_template_html_renderer_new() {
        let renderer = TemplateHTMLRenderer::new();
        assert_eq!(renderer.charset, "utf-8");
        assert_eq!(renderer.media_type(), "text/html; charset=utf-8");
    }

    #[tokio::test]
    async fn test_template_html_renderer_with_charset() {
        let renderer = TemplateHTMLRenderer::with_charset("iso-8859-1");
        assert_eq!(renderer.charset, "iso-8859-1");
        assert_eq!(renderer.media_type(), "text/html; charset=iso-8859-1");
    }

    #[tokio::test]
    async fn test_render_simple_template() {
        let renderer = TemplateHTMLRenderer::new();
        let context = json!({
            "template_string": "<h1>{{ title }}</h1>",
            "title": "Hello World"
        });

        let result = renderer.render(&context, None).await;
        assert!(result.is_ok());

        let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(html, "<h1>Hello World</h1>");
    }

    #[tokio::test]
    async fn test_render_multiple_variables() {
        let renderer = TemplateHTMLRenderer::new();
        let context = json!({
            "template_string": "<h1>{{ title }}</h1><p>{{ message }}</p>",
            "title": "Welcome",
            "message": "This is a test"
        });

        let result = renderer.render(&context, None).await;
        assert!(result.is_ok());

        let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(html, "<h1>Welcome</h1><p>This is a test</p>");
    }

    #[tokio::test]
    async fn test_render_number_variable() {
        let renderer = TemplateHTMLRenderer::new();
        let context = json!({
            "template_string": "<p>Count: {{ count }}</p>",
            "count": 42
        });

        let result = renderer.render(&context, None).await;
        assert!(result.is_ok());

        let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(html, "<p>Count: 42</p>");
    }

    #[tokio::test]
    async fn test_render_boolean_variable() {
        let renderer = TemplateHTMLRenderer::new();
        let context = json!({
            "template_string": "<p>Active: {{ active }}</p>",
            "active": true
        });

        let result = renderer.render(&context, None).await;
        assert!(result.is_ok());

        let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
        assert_eq!(html, "<p>Active: true</p>");
    }

    #[tokio::test]
    async fn test_render_missing_template() {
        let renderer = TemplateHTMLRenderer::new();
        let context = json!({
            "title": "Hello World"
        });

        let result = renderer.render(&context, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_render_file_template_not_implemented() {
        let renderer = TemplateHTMLRenderer::new();
        let context = json!({
            "template_name": "my_template.html",
            "title": "Hello World"
        });

        let result = renderer.render(&context, None).await;
        assert!(result.is_err());

        match result {
            Err(Error::Internal(msg)) => {
                assert!(msg.contains("File-based template loading not yet implemented"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_format() {
        let renderer = TemplateHTMLRenderer::new();
        assert_eq!(renderer.format(), Some("html"));
    }
}
