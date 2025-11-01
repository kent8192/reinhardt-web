//! Admin Renderer
//!
//! Renderer for Django-like admin interface responses.
//! Based on Django REST Framework's AdminRenderer.

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
					"Warning: Failed to initialize Tera for admin templates: {}",
					e
				);
				Arc::new(Tera::default())
			}
		}
	})
}

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
		let tera = get_tera_engine();
		let mut context = Context::new();

		// Add result_url if available
		if let Some(result_url) = self.get_result_url(data) {
			context.insert("result_url", &result_url);
		}

		// Build context based on data type
		match data {
			Value::Object(map) => {
				context.insert("is_object", &true);
				context.insert("is_array", &false);

				let fields: Vec<serde_json::Value> = map
					.iter()
					.map(|(key, value)| {
						let value_str = match value {
							Value::String(s) => s.clone(),
							Value::Number(n) => n.to_string(),
							Value::Bool(b) => b.to_string(),
							Value::Null => "null".to_string(),
							Value::Array(_) | Value::Object(_) => {
								serde_json::to_string_pretty(value)
									.unwrap_or_else(|_| "{}".to_string())
							}
						};
						serde_json::json!({"key": key, "value": value_str})
					})
					.collect();
				context.insert("fields", &fields);
			}
			Value::Array(arr) => {
				context.insert("is_object", &false);
				context.insert("is_array", &true);

				// Extract headers from first object
				let headers: Vec<String> = if let Some(Value::Object(obj)) = arr.first() {
					obj.keys().cloned().collect()
				} else {
					vec![]
				};
				context.insert("headers", &headers);

				// Build rows
				let rows: Vec<Vec<String>> = arr
					.iter()
					.filter_map(|item| {
						if let Value::Object(obj) = item {
							Some(
								headers
									.iter()
									.map(|key| {
										obj.get(key).map_or("".to_string(), |value| match value {
											Value::String(s) => s.clone(),
											Value::Number(n) => n.to_string(),
											Value::Bool(b) => b.to_string(),
											Value::Null => "null".to_string(),
											_ => serde_json::to_string(value)
												.unwrap_or_else(|_| "{}".to_string()),
										})
									})
									.collect(),
							)
						} else {
							None
						}
					})
					.collect();
				context.insert("rows", &rows);
			}
			_ => {
				context.insert("is_object", &false);
				context.insert("is_array", &false);
				context.insert(
					"data",
					&serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string()),
				);
			}
		}

		// Try to render with template, fallback to hardcoded HTML if fails
		tera.render("admin.tpl", &context).unwrap_or_else(|e| {
			eprintln!(
				"Warning: Failed to render admin.tpl template: {}. Using fallback.",
				e
			);
			self.render_fallback_html(data)
		})
	}

	/// Fallback HTML rendering (original hardcoded logic)
	fn render_fallback_html(&self, data: &Value) -> String {
		let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
		html.push_str("<title>Admin Interface</title>\n");
		html.push_str("<style>\n");
		html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
		html.push_str(".success { color: green; font-weight: bold; }\n");
		html.push_str(".data-table { border-collapse: collapse; width: 100%; }\n");
		html.push_str(
			".data-table th, .data-table td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n",
		);
		html.push_str(".data-table th { background-color: #f2f2f2; }\n");
		html.push_str("</style>\n");
		html.push_str("</head>\n<body>\n");

		if let Some(result_url) = self.get_result_url(data) {
			html.push_str("<div class=\"success\">Resource created successfully!</div>\n");
			html.push_str(&format!(
				"<p>View at: <a href=\"{}\">{}</a></p>\n",
				result_url, result_url
			));
		}

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
				if let Some(first) = arr.first() {
					if let Value::Object(obj) = first {
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
		// URL path is HTML-escaped by Tera (/ becomes &#x2F;)
		assert!(html.contains("&#x2F;admin&#x2F;42") || html.contains("/admin/42"));
	}

	#[tokio::test]
	async fn test_admin_renderer_custom_base_url() {
		let renderer = AdminRenderer::new().base_url("/custom");
		let data = json!({"id": 5, "name": "Item"});

		let result = renderer.render(&data, None).await.unwrap();
		let html = String::from_utf8(result.to_vec()).unwrap();

		// URL path is HTML-escaped by Tera (/ becomes &#x2F;)
		assert!(html.contains("&#x2F;custom&#x2F;5") || html.contains("/custom/5"));
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
