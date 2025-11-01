use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::Error;
use serde_json::Value;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// OpenAPI renderer for API documentation
///
/// Renders OpenAPI specifications in JSON or YAML format.
#[derive(Debug, Clone)]
pub struct OpenAPIRenderer {
	/// Output format: "json" or "yaml"
	pub format: String,
	/// Whether to pretty print the output
	pub pretty: bool,
}

impl Default for OpenAPIRenderer {
	fn default() -> Self {
		Self {
			format: "json".to_string(),
			pretty: true,
		}
	}
}

impl OpenAPIRenderer {
	/// Creates a new OpenAPI renderer with JSON format
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::OpenAPIRenderer;
	///
	/// let renderer = OpenAPIRenderer::new();
	/// assert_eq!(renderer.format, "json");
	/// assert!(renderer.pretty);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the output format (json or yaml)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::OpenAPIRenderer;
	///
	/// let renderer = OpenAPIRenderer::new().format("yaml");
	/// assert_eq!(renderer.format, "yaml");
	/// ```
	pub fn format(mut self, format: impl Into<String>) -> Self {
		self.format = format.into();
		self
	}

	/// Sets whether to pretty print the output
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::OpenAPIRenderer;
	///
	/// let renderer = OpenAPIRenderer::new().pretty(false);
	/// assert!(!renderer.pretty);
	/// ```
	pub fn pretty(mut self, pretty: bool) -> Self {
		self.pretty = pretty;
		self
	}

	/// Render OpenAPI spec from Value
	fn render_openapi(&self, data: &Value) -> Result<String, Error> {
		// If data is already a serialized OpenAPI spec (as JSON Value),
		// render it in the requested format
		match self.format.as_str() {
			"yaml" => serde_yaml::to_string(data).map_err(|e| Error::Serialization(e.to_string())),
			"json" | _ => if self.pretty {
				serde_json::to_string_pretty(data)
			} else {
				serde_json::to_string(data)
			}
			.map_err(|e| Error::Serialization(e.to_string())),
		}
	}
}

#[async_trait]
impl Renderer for OpenAPIRenderer {
	fn media_types(&self) -> Vec<String> {
		match self.format.as_str() {
			"yaml" => vec!["application/vnd.oai.openapi".to_string()],
			_ => vec!["application/vnd.oai.openapi+json".to_string()],
		}
	}

	fn format(&self) -> Option<&str> {
		Some("openapi")
	}

	async fn render(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Bytes> {
		let output = self.render_openapi(data)?;
		Ok(Bytes::from(output))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[tokio::test]
	async fn test_openapi_renderer_json() {
		let renderer = OpenAPIRenderer::new();
		let data = json!({
			"openapi": "3.0.0",
			"info": {
				"title": "Test API",
				"version": "1.0.0"
			},
			"paths": {}
		});

		let result = renderer.render(&data, None).await.unwrap();
		let output = String::from_utf8(result.to_vec()).unwrap();

		assert!(output.contains("openapi"));
		assert!(output.contains("3.0.0"));
		assert!(output.contains("Test API"));
	}

	#[tokio::test]
	async fn test_openapi_renderer_yaml() {
		let renderer = OpenAPIRenderer::new().format("yaml");
		let data = json!({
			"openapi": "3.0.0",
			"info": {
				"title": "Test API",
				"version": "1.0.0"
			}
		});

		let result = renderer.render(&data, None).await.unwrap();
		let output = String::from_utf8(result.to_vec()).unwrap();

		assert!(output.contains("openapi:"));
		assert!(output.contains("info:"));
	}

	#[tokio::test]
	async fn test_openapi_renderer_pretty() {
		let renderer = OpenAPIRenderer::new().pretty(true);
		let data = json!({"openapi": "3.0.0"});

		let result = renderer.render(&data, None).await.unwrap();
		let output = String::from_utf8(result.to_vec()).unwrap();

		// Pretty printed JSON should have newlines
		assert!(output.contains('\n'));
	}

	#[tokio::test]
	async fn test_openapi_renderer_compact() {
		let renderer = OpenAPIRenderer::new().pretty(false);
		let data = json!({"openapi": "3.0.0", "info": {"title": "API"}});

		let result = renderer.render(&data, None).await.unwrap();
		let output = String::from_utf8(result.to_vec()).unwrap();

		// Compact JSON should not have extra whitespace
		assert!(!output.contains("  "));
	}
}
