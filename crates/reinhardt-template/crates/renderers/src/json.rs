use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::Error;
use serde_json::Value;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// JSON renderer with pretty printing support
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct JSONRenderer {
	/// Whether to pretty print the output
	pub pretty: bool,
	/// Custom JSON encoder settings
	pub ensure_ascii: bool,
}


impl JSONRenderer {
	/// Creates a new JSON renderer with default settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::JSONRenderer;
	///
	/// let renderer = JSONRenderer::new();
	/// assert!(!renderer.pretty);
	/// assert!(!renderer.ensure_ascii);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}
	/// Sets whether to pretty print the JSON output
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::JSONRenderer;
	///
	/// let renderer = JSONRenderer::new().pretty(true);
	/// assert!(renderer.pretty);
	/// ```
	pub fn pretty(mut self, pretty: bool) -> Self {
		self.pretty = pretty;
		self
	}
	/// Sets whether to ensure ASCII output
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::JSONRenderer;
	///
	/// let renderer = JSONRenderer::new().ensure_ascii(true);
	/// assert!(renderer.ensure_ascii);
	/// ```
	pub fn ensure_ascii(mut self, ensure: bool) -> Self {
		self.ensure_ascii = ensure;
		self
	}
}

#[async_trait]
impl Renderer for JSONRenderer {
	fn media_type(&self) -> String {
		"application/json; charset=utf-8".to_string()
	}

	fn media_types(&self) -> Vec<String> {
		vec![
			"application/json".to_string(),
			"application/json; charset=utf-8".to_string(),
		]
	}

	fn format(&self) -> Option<&str> {
		Some("json")
	}

	async fn render(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Bytes> {
		let json_string = if self.pretty {
			serde_json::to_string_pretty(data)
		} else {
			serde_json::to_string(data)
		}
		.map_err(|e| Error::Serialization(e.to_string()))?;

		Ok(Bytes::from(json_string))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[tokio::test]
	async fn test_json_renderer() {
		let renderer = JSONRenderer::new();
		let data = json!({"name": "test", "value": 123});

		let result = renderer.render(&data, None).await.unwrap();
		let json_str = String::from_utf8(result.to_vec()).unwrap();

		assert!(json_str.contains("test"));
		assert!(json_str.contains("123"));
	}

	#[tokio::test]
	async fn test_json_renderer_pretty() {
		let renderer = JSONRenderer::new().pretty(true);
		let data = json!({"name": "test", "value": 123});

		let result = renderer.render(&data, None).await.unwrap();
		let json_str = String::from_utf8(result.to_vec()).unwrap();

		assert!(json_str.contains("\n"));
		assert!(json_str.contains("  "));
	}
}
