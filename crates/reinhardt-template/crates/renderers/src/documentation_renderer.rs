//! Documentation Renderer
//!
//! Renders API documentation from OpenAPI schemas in HTML or Markdown format.

use crate::renderer::{RenderResult, Renderer, RendererContext};
use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tera::{Context, Tera};

static TERA_ENGINE: OnceLock<Arc<Tera>> = OnceLock::new();

fn get_tera_engine() -> &'static Arc<Tera> {
	TERA_ENGINE.get_or_init(|| {
		let manifest_dir = env!("CARGO_MANIFEST_DIR");
		let template_dir = PathBuf::from(manifest_dir).join("templates");
		let glob_pattern = format!("{}/**/*", template_dir.display());

		match Tera::new(&glob_pattern) {
			Ok(tera) => Arc::new(tera),
			Err(e) => {
				eprintln!(
					"Warning: Failed to initialize Tera for documentation templates: {}",
					e
				);
				Arc::new(Tera::default())
			}
		}
	})
}

/// Documentation renderer for API documentation
///
/// Renders OpenAPI schema as human-readable documentation in HTML format.
#[derive(Debug, Clone, Default)]
pub struct DocumentationRenderer {
	/// Output format: "html" or "markdown"
	pub format_type: String,
}

impl DocumentationRenderer {
	/// Creates a new DocumentationRenderer with HTML format
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::DocumentationRenderer;
	///
	/// let renderer = DocumentationRenderer::new();
	/// assert_eq!(renderer.format_type, "html");
	/// ```
	pub fn new() -> Self {
		Self {
			format_type: "html".to_string(),
		}
	}

	/// Sets the output format type
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::DocumentationRenderer;
	///
	/// let renderer = DocumentationRenderer::new().format_type("markdown");
	/// assert_eq!(renderer.format_type, "markdown");
	/// ```
	pub fn format_type(mut self, format: impl Into<String>) -> Self {
		self.format_type = format.into();
		self
	}

	/// Converts JSON schema to HTML documentation
	fn to_html(&self, data: &Value) -> String {
		let tera = get_tera_engine();
		let mut context = Context::new();

		if let Some(obj) = data.as_object() {
			// Extract title and description from info
			if let Some(info) = obj.get("info") {
				if let Some(title) = info.get("title").and_then(|t| t.as_str()) {
					context.insert("title", title);
				}
				if let Some(description) = info.get("description").and_then(|d| d.as_str()) {
					context.insert("description", description);
				}
			}

			// Extract endpoints from paths
			if let Some(paths) = obj.get("paths").and_then(|p| p.as_object()) {
				let mut endpoints = Vec::new();
				for (path, methods) in paths {
					if let Some(methods_obj) = methods.as_object() {
						for (method, operation) in methods_obj {
							let desc = operation
								.as_object()
								.and_then(|op| op.get("description").and_then(|d| d.as_str()))
								.unwrap_or("");

							endpoints.push(serde_json::json!({
								"method": method.to_uppercase(),
								"path": path,
								"description": desc
							}));
						}
					}
				}
				context.insert("endpoints", &endpoints);
			}
		}

		// Try to render with template, fallback to hardcoded HTML if fails
		tera.render("documentation.tpl", &context)
			.unwrap_or_else(|e| {
				eprintln!(
					"Warning: Failed to render documentation.tpl template: {}. Using fallback.",
					e
				);
				self.render_fallback_html(data)
			})
	}

	/// Fallback HTML rendering (original hardcoded logic)
	fn render_fallback_html(&self, data: &Value) -> String {
		let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
		html.push_str("<meta charset=\"UTF-8\">\n");
		html.push_str("<title>API Documentation</title>\n");
		html.push_str("<style>\n");
		html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
		html.push_str("h1 { color: #333; }\n");
		html.push_str(".endpoint { margin: 20px 0; padding: 15px; border: 1px solid #ddd; }\n");
		html.push_str(".method { font-weight: bold; color: #0066cc; }\n");
		html.push_str(".description { color: #666; margin: 10px 0; }\n");
		html.push_str("</style>\n");
		html.push_str("</head>\n<body>\n");

		if let Some(obj) = data.as_object() {
			if let Some(info) = obj.get("info") {
				if let Some(title) = info.get("title").and_then(|t| t.as_str()) {
					html.push_str(&format!("<h1>{}</h1>\n", title));
				}
				if let Some(description) = info.get("description").and_then(|d| d.as_str()) {
					html.push_str(&format!("<p class=\"description\">{}</p>\n", description));
				}
			}

			if let Some(paths) = obj.get("paths").and_then(|p| p.as_object()) {
				html.push_str("<h2>Endpoints</h2>\n");
				for (path, methods) in paths {
					if let Some(methods_obj) = methods.as_object() {
						for (method, operation) in methods_obj {
							html.push_str("<div class=\"endpoint\">\n");
							html.push_str(&format!(
								"<span class=\"method\">{}</span> {}\n",
								method.to_uppercase(),
								path
							));

							if let Some(op_obj) = operation.as_object()
								&& let Some(desc) =
									op_obj.get("description").and_then(|d| d.as_str())
								{
									html.push_str(&format!(
										"<p class=\"description\">{}</p>\n",
										desc
									));
								}
							html.push_str("</div>\n");
						}
					}
				}
			}
		}

		html.push_str("</body>\n</html>");
		html
	}

	/// Converts JSON schema to Markdown documentation
	fn to_markdown(&self, data: &Value) -> String {
		let mut md = String::new();

		if let Some(obj) = data.as_object() {
			// Title
			if let Some(info) = obj.get("info") {
				if let Some(title) = info.get("title").and_then(|t| t.as_str()) {
					md.push_str(&format!("# {}\n\n", title));
				}
				if let Some(description) = info.get("description").and_then(|d| d.as_str()) {
					md.push_str(&format!("{}\n\n", description));
				}
			}

			// Paths/Endpoints
			if let Some(paths) = obj.get("paths").and_then(|p| p.as_object()) {
				md.push_str("## Endpoints\n\n");
				for (path, methods) in paths {
					if let Some(methods_obj) = methods.as_object() {
						for (method, operation) in methods_obj {
							md.push_str(&format!("### {} {}\n\n", method.to_uppercase(), path));

							if let Some(op_obj) = operation.as_object()
								&& let Some(desc) =
									op_obj.get("description").and_then(|d| d.as_str())
								{
									md.push_str(&format!("{}\n\n", desc));
								}
						}
					}
				}
			}
		}

		md
	}
}

#[async_trait]
impl Renderer for DocumentationRenderer {
	fn media_types(&self) -> Vec<String> {
		if self.format_type == "markdown" {
			vec!["text/markdown".to_string()]
		} else {
			vec!["text/html".to_string()]
		}
	}

	fn format(&self) -> Option<&str> {
		Some("docs")
	}

	async fn render(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Bytes> {
		let content = if self.format_type == "markdown" {
			self.to_markdown(data)
		} else {
			self.to_html(data)
		};

		Ok(Bytes::from(content))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[tokio::test]
	async fn test_documentation_renderer_html() {
		let renderer = DocumentationRenderer::new();
		let data = json!({
			"info": {
				"title": "Test API",
				"description": "A test API"
			},
			"paths": {
				"/users": {
					"get": {
						"description": "Get all users"
					}
				}
			}
		});

		let result = renderer.render(&data, None).await.unwrap();
		let html = String::from_utf8(result.to_vec()).unwrap();

		assert!(html.contains("<!DOCTYPE html>") || html.contains("<!doctype html>"));
		assert!(html.contains("Test API"));
		assert!(html.contains("GET"));
		// Path is HTML-escaped by Tera (/ becomes &#x2F;)
		assert!(html.contains("&#x2F;users") || html.contains("/users"));
	}

	#[tokio::test]
	async fn test_documentation_renderer_markdown() {
		let renderer = DocumentationRenderer::new().format_type("markdown");
		let data = json!({
			"info": {
				"title": "Test API",
				"description": "A test API"
			},
			"paths": {
				"/users": {
					"get": {
						"description": "Get all users"
					}
				}
			}
		});

		let result = renderer.render(&data, None).await.unwrap();
		let md = String::from_utf8(result.to_vec()).unwrap();

		assert!(md.contains("# Test API"));
		assert!(md.contains("## Endpoints"));
		assert!(md.contains("### GET /users"));
	}
}
