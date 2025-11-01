//! Schema.js Renderer
//!
//! Renders OpenAPI schemas as JavaScript for use with the Schema.js library.

use crate::renderer::{RenderResult, Renderer, RendererContext};
use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value;

/// Schema.js renderer for OpenAPI schemas
///
/// Converts OpenAPI schema to JavaScript format compatible with Schema.js library.
#[derive(Debug, Clone, Default)]
pub struct SchemaJSRenderer;

impl SchemaJSRenderer {
	/// Creates a new SchemaJSRenderer
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::SchemaJSRenderer;
	///
	/// let renderer = SchemaJSRenderer::new();
	/// ```
	pub fn new() -> Self {
		Self
	}

	/// Converts OpenAPI schema to JavaScript
	fn to_javascript(&self, data: &Value) -> String {
		let mut js = String::from("// Generated Schema.js\n");
		js.push_str("const apiSchema = ");

		// Convert JSON to JavaScript object notation
		js.push_str(&self.value_to_js(data, 0));
		js.push_str(";\n\n");

		// Add helper functions
		js.push_str("// Helper function to get endpoint by path and method\n");
		js.push_str("function getEndpoint(path, method) {\n");
		js.push_str("  if (!apiSchema.paths || !apiSchema.paths[path]) return null;\n");
		js.push_str("  return apiSchema.paths[path][method.toLowerCase()];\n");
		js.push_str("}\n\n");

		js.push_str("// Helper function to get all paths\n");
		js.push_str("function getAllPaths() {\n");
		js.push_str("  return Object.keys(apiSchema.paths || {});\n");
		js.push_str("}\n\n");

		// Export for use
		js.push_str("// Export schema and helpers\n");
		js.push_str("if (typeof module !== 'undefined' && module.exports) {\n");
		js.push_str("  module.exports = { apiSchema, getEndpoint, getAllPaths };\n");
		js.push_str("}\n");

		js
	}

	/// Converts a JSON value to JavaScript notation
	fn value_to_js(&self, value: &Value, indent: usize) -> String {
		let indent_str = "  ".repeat(indent);
		let next_indent_str = "  ".repeat(indent + 1);

		match value {
			Value::Object(map) => {
				if map.is_empty() {
					return "{}".to_string();
				}

				let mut result = "{\n".to_string();
				let entries: Vec<_> = map.iter().collect();

				for (i, (key, val)) in entries.iter().enumerate() {
					result.push_str(&next_indent_str);

					// Use quotes for keys with special characters or that are not valid JS identifiers
					if key.chars().all(|c| c.is_alphanumeric() || c == '_')
						&& !key.chars().next().unwrap_or('0').is_numeric()
					{
						result.push_str(key);
					} else {
						result.push_str(&format!("\"{}\"", key));
					}

					result.push_str(": ");
					result.push_str(&self.value_to_js(val, indent + 1));

					if i < entries.len() - 1 {
						result.push(',');
					}
					result.push('\n');
				}
				result.push_str(&indent_str);
				result.push('}');
				result
			}
			Value::Array(arr) => {
				if arr.is_empty() {
					return "[]".to_string();
				}

				let mut result = "[\n".to_string();
				for (i, item) in arr.iter().enumerate() {
					result.push_str(&next_indent_str);
					result.push_str(&self.value_to_js(item, indent + 1));
					if i < arr.len() - 1 {
						result.push(',');
					}
					result.push('\n');
				}
				result.push_str(&indent_str);
				result.push(']');
				result
			}
			Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
			Value::Number(n) => n.to_string(),
			Value::Bool(b) => b.to_string(),
			Value::Null => "null".to_string(),
		}
	}
}

#[async_trait]
impl Renderer for SchemaJSRenderer {
	fn media_types(&self) -> Vec<String> {
		vec!["application/javascript".to_string()]
	}

	fn format(&self) -> Option<&str> {
		Some("schemajs")
	}

	async fn render(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Bytes> {
		let js_content = self.to_javascript(data);
		Ok(Bytes::from(js_content))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[tokio::test]
	async fn test_schemajs_renderer_basic() {
		let renderer = SchemaJSRenderer::new();
		let data = json!({
			"openapi": "3.0.0",
			"info": {
				"title": "Test API",
				"version": "1.0.0"
			},
			"paths": {
				"/users": {
					"get": {
						"summary": "List users"
					}
				}
			}
		});

		let result = renderer.render(&data, None).await.unwrap();
		let js_str = String::from_utf8(result.to_vec()).unwrap();

		// Verify JavaScript content
		assert!(js_str.contains("const apiSchema"));
		assert!(js_str.contains("Test API"));
		assert!(js_str.contains("/users"));
		assert!(js_str.contains("function getEndpoint"));
		assert!(js_str.contains("function getAllPaths"));
	}

	#[tokio::test]
	async fn test_schemajs_renderer_media_type() {
		let renderer = SchemaJSRenderer::new();
		let media_types = renderer.media_types();

		assert_eq!(media_types.len(), 1);
		assert_eq!(media_types[0], "application/javascript");
	}

	#[tokio::test]
	async fn test_schemajs_renderer_format() {
		let renderer = SchemaJSRenderer::new();
		assert_eq!(renderer.format(), Some("schemajs"));
	}

	#[tokio::test]
	async fn test_schemajs_javascript_syntax() {
		let renderer = SchemaJSRenderer::new();
		let data = json!({
			"paths": {
				"/test": {
					"get": {
						"description": "Test endpoint"
					}
				}
			}
		});

		let result = renderer.render(&data, None).await.unwrap();
		let js_str = String::from_utf8(result.to_vec()).unwrap();

		// Verify valid JavaScript syntax elements
		assert!(js_str.contains("paths:"));
		assert!(js_str.contains("\"/test\""));
		assert!(js_str.contains("get:"));
		assert!(js_str.contains("module.exports"));
	}
}
