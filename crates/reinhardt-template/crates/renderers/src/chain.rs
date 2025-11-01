//! Renderer chain implementation
//!
//! Chains multiple renderers in pipeline format to transform data progressively.

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::Error;
use serde_json::Value;
use std::sync::Arc;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// Renderer chain builder
///
/// Chains multiple renderers in pipeline format.
/// Data passes through each renderer in sequence and is transformed progressively.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::{RendererChain, JSONRenderer, Renderer};
/// use serde_json::json;
///
/// # #[tokio::main]
/// # async fn main() {
/// let chain = RendererChain::new()
///     .pipe(JSONRenderer::new());
///
/// let data = json!({"message": "hello"});
/// let result = chain.render(&data, None).await;
/// assert!(result.is_ok());
/// # }
/// ```
pub struct RendererChain {
	/// List of renderers in the chain
	renderers: Vec<Arc<dyn Renderer>>,
	/// Default media type (obtained from the last renderer)
	media_type: String,
	/// Default format (obtained from the last renderer)
	format: Option<String>,
}

impl RendererChain {
	/// Create a new renderer chain
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::RendererChain;
	///
	/// let chain = RendererChain::new();
	/// ```
	pub fn new() -> Self {
		Self {
			renderers: Vec::new(),
			media_type: String::new(),
			format: None,
		}
	}

	/// Add a renderer to the chain
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{RendererChain, JSONRenderer};
	///
	/// let chain = RendererChain::new()
	///     .pipe(JSONRenderer::new());
	/// ```
	pub fn pipe<R: Renderer + 'static>(mut self, renderer: R) -> Self {
		self.media_type = renderer.media_type();
		self.format = renderer.format().map(|s| s.to_string());
		self.renderers.push(Arc::new(renderer));
		self
	}

	/// Check if the chain is empty
	pub fn is_empty(&self) -> bool {
		self.renderers.is_empty()
	}

	/// Get the number of renderers in the chain
	pub fn len(&self) -> usize {
		self.renderers.len()
	}
}

impl Default for RendererChain {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Renderer for RendererChain {
	fn media_type(&self) -> String {
		self.media_type.clone()
	}

	fn media_types(&self) -> Vec<String> {
		if let Some(last_renderer) = self.renderers.last() {
			last_renderer.media_types()
		} else {
			Vec::new()
		}
	}

	fn format(&self) -> Option<&str> {
		self.format.as_deref()
	}

	async fn render(&self, data: &Value, context: Option<&RendererContext>) -> RenderResult<Bytes> {
		if self.renderers.is_empty() {
			return Err(Error::Http(
				"RendererChain is empty - no renderers to execute".to_string(),
			));
		}

		// Execute the first renderer
		let mut current_data = data.clone();
		let first_renderer = &self.renderers[0];
		let mut current_bytes = first_renderer.render(&current_data, context).await?;

		// Execute remaining renderers sequentially
		for renderer in self.renderers.iter().skip(1) {
			// Parse the output from the previous renderer as JSON
			let json_str = String::from_utf8(current_bytes.to_vec()).map_err(|e| {
				Error::Serialization(format!("Failed to convert bytes to UTF-8: {}", e))
			})?;

			current_data = serde_json::from_str(&json_str).map_err(|e| {
				Error::Serialization(format!(
					"Failed to parse intermediate result as JSON: {}",
					e
				))
			})?;

			// Execute the next renderer (return immediately if an error occurs)
			current_bytes = renderer.render(&current_data, context).await?;
		}

		Ok(current_bytes)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::JSONRenderer;
	use serde_json::json;

	/// Test transformation renderer (adds fields to data)
	#[derive(Clone)]
	struct TransformRenderer {
		field_name: String,
		field_value: String,
	}

	impl TransformRenderer {
		fn new(field_name: &str, field_value: &str) -> Self {
			Self {
				field_name: field_name.to_string(),
				field_value: field_value.to_string(),
			}
		}
	}

	#[async_trait]
	impl Renderer for TransformRenderer {
		fn media_type(&self) -> String {
			"application/json".to_string()
		}

		fn media_types(&self) -> Vec<String> {
			vec!["application/json".to_string()]
		}

		async fn render(
			&self,
			data: &Value,
			_context: Option<&RendererContext>,
		) -> RenderResult<Bytes> {
			let mut obj = data.as_object().cloned().unwrap_or_default();
			obj.insert(
				self.field_name.clone(),
				Value::String(self.field_value.clone()),
			);
			let result =
				serde_json::to_string(&obj).map_err(|e| Error::Serialization(e.to_string()))?;
			Ok(Bytes::from(result))
		}
	}

	#[tokio::test]
	async fn test_empty_chain() {
		let chain = RendererChain::new();
		let data = json!({"test": "data"});

		let result = chain.render(&data, None).await;
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("RendererChain is empty")
		);
	}

	#[tokio::test]
	async fn test_single_renderer_chain() {
		let chain = RendererChain::new().pipe(JSONRenderer::new());

		let data = json!({"name": "test"});
		let result = chain.render(&data, None).await.unwrap();
		let json_str = String::from_utf8(result.to_vec()).unwrap();

		// Structural validation: Parse as JSON and verify structure
		let parsed: Value =
			serde_json::from_str(&json_str).expect("Renderer output should be valid JSON");
		assert_eq!(
			parsed.get("name").and_then(|v| v.as_str()),
			Some("test"),
			"JSON should contain 'name' field with value 'test'"
		);
	}

	#[tokio::test]
	async fn test_multiple_renderers_chain() {
		let chain = RendererChain::new()
			.pipe(TransformRenderer::new("step1", "value1"))
			.pipe(TransformRenderer::new("step2", "value2"))
			.pipe(JSONRenderer::new());

		let data = json!({"original": "data"});
		let result = chain.render(&data, None).await.unwrap();
		let json_str = String::from_utf8(result.to_vec()).unwrap();

		// Structural validation: Parse as JSON and verify all expected fields
		let parsed: Value =
			serde_json::from_str(&json_str).expect("Renderer chain output should be valid JSON");

		assert_eq!(
			parsed.get("original").and_then(|v| v.as_str()),
			Some("data"),
			"JSON should contain original 'original' field with value 'data'"
		);
		assert_eq!(
			parsed.get("step1").and_then(|v| v.as_str()),
			Some("value1"),
			"JSON should contain 'step1' field added by first transform"
		);
		assert_eq!(
			parsed.get("step2").and_then(|v| v.as_str()),
			Some("value2"),
			"JSON should contain 'step2' field added by second transform"
		);
	}

	#[tokio::test]
	async fn test_chain_media_type() {
		let chain = RendererChain::new()
			.pipe(TransformRenderer::new("test", "value"))
			.pipe(JSONRenderer::new());

		assert_eq!(chain.media_type(), "application/json; charset=utf-8");
	}

	#[tokio::test]
	async fn test_chain_format() {
		let chain = RendererChain::new().pipe(JSONRenderer::new());

		assert_eq!(chain.format(), Some("json"));
	}

	#[tokio::test]
	async fn test_chain_len() {
		let chain = RendererChain::new()
			.pipe(JSONRenderer::new())
			.pipe(TransformRenderer::new("test", "value"));

		assert_eq!(chain.len(), 2);
		assert!(!chain.is_empty());
	}

	#[tokio::test]
	async fn test_empty_chain_properties() {
		let chain = RendererChain::new();

		assert_eq!(chain.len(), 0);
		assert!(chain.is_empty());
	}

	/// Test renderer that generates errors
	struct ErrorRenderer;

	#[async_trait]
	impl Renderer for ErrorRenderer {
		fn media_type(&self) -> String {
			"application/json".to_string()
		}

		fn media_types(&self) -> Vec<String> {
			vec!["application/json".to_string()]
		}

		async fn render(
			&self,
			_data: &Value,
			_context: Option<&RendererContext>,
		) -> RenderResult<Bytes> {
			Err(Error::Serialization("Intentional error".to_string()))
		}
	}

	#[tokio::test]
	async fn test_chain_error_handling_first_renderer() {
		let chain = RendererChain::new().pipe(ErrorRenderer);

		let data = json!({"test": "data"});
		let result = chain.render(&data, None).await;

		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("Intentional error")
		);
	}

	#[tokio::test]
	async fn test_chain_error_handling_middle_renderer() {
		let chain = RendererChain::new()
			.pipe(JSONRenderer::new())
			.pipe(ErrorRenderer)
			.pipe(JSONRenderer::new());

		let data = json!({"test": "data"});
		let result = chain.render(&data, None).await;

		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("Intentional error")
		);
	}

	#[tokio::test]
	async fn test_chain_with_context() {
		let context = RendererContext::new()
			.with_request("GET", "/api/test")
			.with_view("TestView", "Test view description");

		let chain = RendererChain::new().pipe(JSONRenderer::new());

		let data = json!({"test": "data"});
		let result = chain.render(&data, Some(&context)).await;

		assert!(result.is_ok());
	}
}
