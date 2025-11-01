//! Static HTML Renderer
//!
//! Renderer that returns pre-defined static HTML content.
//! Based on Django REST Framework's StaticHTMLRenderer.

use crate::renderer::{RenderResult, Renderer, RendererContext};
use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value;

/// Renderer that returns static HTML content
///
/// This renderer ignores the data passed to it and returns
/// pre-configured static HTML content. Useful for serving
/// static pages or templates that don't depend on data.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::{StaticHTMLRenderer, Renderer};
///
/// let content = "<html><body><h1>Hello</h1></body></html>";
/// let renderer = StaticHTMLRenderer::new(content);
/// assert_eq!(renderer.media_types(), vec!["text/html"]);
/// ```
#[derive(Debug, Clone)]
pub struct StaticHTMLRenderer {
	content: String,
}

impl StaticHTMLRenderer {
	/// Creates a new static HTML renderer with the given content
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::StaticHTMLRenderer;
	///
	/// let renderer = StaticHTMLRenderer::new("<h1>Static Content</h1>");
	/// ```
	pub fn new(content: impl Into<String>) -> Self {
		Self {
			content: content.into(),
		}
	}

	/// Returns the static content
	pub fn content(&self) -> &str {
		&self.content
	}
}

#[async_trait]
impl Renderer for StaticHTMLRenderer {
	fn media_types(&self) -> Vec<String> {
		vec!["text/html".to_string()]
	}

	fn format(&self) -> Option<&str> {
		Some("html")
	}

	async fn render(
		&self,
		_data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Bytes> {
		Ok(Bytes::from(self.content.clone()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

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

	#[tokio::test]
	async fn test_static_html_renderer_media_types() {
		let renderer = StaticHTMLRenderer::new("test");
		assert_eq!(renderer.media_types(), vec!["text/html"]);
	}

	#[tokio::test]
	async fn test_static_html_renderer_format() {
		let renderer = StaticHTMLRenderer::new("test");
		assert_eq!(renderer.format(), Some("html"));
	}
}
