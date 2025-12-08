//! Template HTML Renderer
//!
//! Renders HTML responses using Tera templates.
//! Supports Django-style template rendering with full Tera syntax including:
//! - Variable substitution: `{{ variable }}`
//! - Conditionals: `{% if condition %}...{% endif %}`
//! - Loops: `{% for item in items %}...{% endfor %}`
//! - Filters: `{{ variable | filter }}`

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_core::exception::{Error, Result};
use serde_json::Value;
use tera::{Context, Tera};

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// HTML renderer that uses Tera templates
///
/// This renderer integrates with the `reinhardt-templates` crate to provide
/// Django-style template rendering for HTML responses.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::TemplateHTMLRenderer;
/// use serde_json::json;
///
/// let renderer = TemplateHTMLRenderer::new();
/// ```
pub struct TemplateHTMLRenderer {
	/// Default charset for HTML responses
	charset: String,
	/// Template directory path (optional)
	template_dir: Option<std::path::PathBuf>,
}

impl TemplateHTMLRenderer {
	/// Creates a new TemplateHTMLRenderer with UTF-8 charset
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{TemplateHTMLRenderer, Renderer};
	///
	/// let renderer = TemplateHTMLRenderer::new();
	/// assert_eq!(renderer.media_types(), vec!["text/html; charset=utf-8"]);
	/// ```
	pub fn new() -> Self {
		Self {
			charset: "utf-8".to_string(),
			template_dir: None,
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
			template_dir: None,
		}
	}

	/// Creates a new TemplateHTMLRenderer with a custom template directory
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::TemplateHTMLRenderer;
	/// use std::path::PathBuf;
	///
	/// let renderer = TemplateHTMLRenderer::with_template_dir(PathBuf::from("templates"));
	/// ```
	pub fn with_template_dir(template_dir: std::path::PathBuf) -> Self {
		Self {
			charset: "utf-8".to_string(),
			template_dir: Some(template_dir),
		}
	}

	/// Set the template directory for this renderer
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::TemplateHTMLRenderer;
	/// use std::path::PathBuf;
	///
	/// let renderer = TemplateHTMLRenderer::new()
	///     .set_template_dir(PathBuf::from("templates"));
	/// ```
	pub fn set_template_dir(mut self, template_dir: std::path::PathBuf) -> Self {
		self.template_dir = Some(template_dir);
		self
	}

	/// Builds a Tera context from a JSON value
	///
	/// Extracts all key-value pairs from the JSON object and inserts them
	/// into a Tera context, excluding reserved keys like `template_string`
	/// and `template_name`.
	fn build_tera_context(json_context: &Value) -> Context {
		let mut tera_context = Context::new();

		if let Value::Object(map) = json_context {
			for (key, value) in map {
				// Skip reserved template keys
				if key == "template_string" || key == "template_name" {
					continue;
				}

				// Insert value directly - Tera handles JSON Value natively
				tera_context.insert(key, value);
			}
		}

		tera_context
	}

	/// Renders a template with the given context data using Tera
	///
	/// Supports full Tera template syntax including:
	/// - Variable substitution: `{{ variable }}`
	/// - Conditionals: `{% if condition %}...{% endif %}`
	/// - Loops: `{% for item in items %}...{% endfor %}`
	/// - Filters: `{{ variable | filter }}`
	///
	/// The context should contain a special key "template_name" to identify
	/// which template to render, or "template_string" for inline templates.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{TemplateHTMLRenderer, Renderer};
	/// use serde_json::json;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// let renderer = TemplateHTMLRenderer::new();
	///
	/// // Simple variable substitution
	/// let context = json!({
	///     "template_string": "<h1>{{ title }}</h1>",
	///     "title": "Hello World"
	/// });
	/// let result = renderer.render(&context, None).await;
	/// assert!(result.is_ok());
	///
	/// // Conditional rendering
	/// let context = json!({
	///     "template_string": "{% if age >= 18 %}Adult{% else %}Minor{% endif %}",
	///     "age": 25
	/// });
	/// let result = renderer.render(&context, None).await;
	/// assert!(result.is_ok());
	/// # });
	/// ```
	pub async fn render_template(&self, context: &Value) -> Result<String> {
		let tera_context = Self::build_tera_context(context);

		if let Some(template_str) = context.get("template_string").and_then(|v| v.as_str()) {
			// Render inline template using Tera
			Tera::one_off(template_str, &tera_context, true)
				.map_err(|e| Error::Internal(format!("Template rendering error: {}", e)))
		} else if let Some(template_name) = context.get("template_name").and_then(|v| v.as_str()) {
			// Load template from file system
			let template_dir = self.template_dir.as_ref().ok_or_else(|| {
				Error::Internal(
					"Template directory not configured. Use set_template_dir() or with_template_dir()."
						.to_string(),
				)
			})?;

			let template_path = template_dir.join(template_name);

			// Read template file
			let template_str = std::fs::read_to_string(&template_path).map_err(|e| {
				Error::Internal(format!(
					"Failed to read template file '{}': {}",
					template_path.display(),
					e
				))
			})?;

			// Render using Tera
			Tera::one_off(&template_str, &tera_context, true)
				.map_err(|e| Error::Internal(format!("Template rendering error: {}", e)))
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
				assert!(msg.contains("Template directory not configured"));
			}
			_ => panic!("Expected Internal error"),
		}
	}

	#[test]
	fn test_format() {
		let renderer = TemplateHTMLRenderer::new();
		assert_eq!(renderer.format(), Some("html"));
	}

	// Tests for Tera template features

	#[tokio::test]
	async fn test_render_if_conditional_true() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{% if age >= 18 %}Adult{% else %}Minor{% endif %}",
			"age": 25
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "Adult");
	}

	#[tokio::test]
	async fn test_render_if_conditional_false() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{% if age >= 18 %}Adult{% else %}Minor{% endif %}",
			"age": 15
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "Minor");
	}

	#[tokio::test]
	async fn test_render_for_loop() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{% for item in items %}{{ item }},{% endfor %}",
			"items": ["a", "b", "c"]
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "a,b,c,");
	}

	#[tokio::test]
	async fn test_render_for_loop_with_objects() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{% for user in users %}<p>{{ user.name }}</p>{% endfor %}",
			"users": [
				{"name": "Alice"},
				{"name": "Bob"}
			]
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "<p>Alice</p><p>Bob</p>");
	}

	#[tokio::test]
	async fn test_render_filter_upper() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ name | upper }}",
			"name": "alice"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "ALICE");
	}

	#[tokio::test]
	async fn test_render_filter_lower() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ name | lower }}",
			"name": "ALICE"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "alice");
	}

	#[tokio::test]
	async fn test_render_filter_length() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ items | length }}",
			"items": [1, 2, 3, 4, 5]
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "5");
	}

	#[tokio::test]
	async fn test_render_nested_if_for() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{% for user in users %}{% if user.active %}{{ user.name }},{% endif %}{% endfor %}",
			"users": [
				{"name": "Alice", "active": true},
				{"name": "Bob", "active": false},
				{"name": "Charlie", "active": true}
			]
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "Alice,Charlie,");
	}

	#[tokio::test]
	async fn test_render_consecutive_variables() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ a }}{{ b }}{{ c }}",
			"a": "Hello",
			"b": " ",
			"c": "World"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "Hello World");
	}

	#[tokio::test]
	async fn test_render_empty_template() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": ""
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "");
	}

	#[tokio::test]
	async fn test_render_no_variables() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "<h1>Static Content</h1>"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "<h1>Static Content</h1>");
	}

	#[tokio::test]
	async fn test_render_complex_user_profile() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": r#"
<div class="profile">
    <h1>{{ name }}</h1>
    <p>Email: {{ email }}</p>
    <p>Age: {{ age }}</p>
    {% if age >= 18 %}
    <span class="badge">Adult</span>
    {% endif %}
</div>
"#,
			"name": "Alice",
			"email": "alice@example.com",
			"age": 25
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert!(html.contains("<h1>Alice</h1>"));
		assert!(html.contains("Email: alice@example.com"));
		assert!(html.contains("Age: 25"));
		assert!(html.contains("Adult"));
	}

	#[tokio::test]
	async fn test_render_html_escaping() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "<div>{{ content }}</div>",
			"content": "<script>alert('xss')</script>"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		// Tera auto-escapes HTML by default
		assert!(html.contains("&lt;script&gt;"));
		assert!(!html.contains("<script>alert"));
	}

	#[tokio::test]
	async fn test_render_safe_filter_no_escaping() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "<div>{{ content | safe }}</div>",
			"content": "<strong>Bold</strong>"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		// Using | safe filter bypasses escaping
		assert_eq!(html, "<div><strong>Bold</strong></div>");
	}

	#[tokio::test]
	async fn test_render_undefined_variable_error() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ undefined_var }}"
		});

		// Tera in strict mode (autoescape=true) will error on undefined variables
		let result = renderer.render(&context, None).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_render_default_filter() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ name | default(value=\"Guest\") }}"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "Guest");
	}

	#[tokio::test]
	async fn test_render_math_operations() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ a + b }}",
			"a": 10,
			"b": 5
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "15");
	}

	#[tokio::test]
	async fn test_render_string_concatenation() {
		let renderer = TemplateHTMLRenderer::new();
		let context = json!({
			"template_string": "{{ first ~ \" \" ~ last }}",
			"first": "Hello",
			"last": "World"
		});

		let result = renderer.render(&context, None).await;
		assert!(result.is_ok());

		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		assert_eq!(html, "Hello World");
	}

	#[tokio::test]
	async fn test_render_performance_many_variables() {
		use std::time::Instant;

		let renderer = TemplateHTMLRenderer::new();

		// Build template and context with 100 variables
		let mut template = String::new();
		let mut context_map = serde_json::Map::new();

		for i in 0..100 {
			template.push_str(&format!("{{{{ var{} }}}}", i));
			context_map.insert(format!("var{}", i), json!(format!("value{}", i)));
		}
		context_map.insert("template_string".to_string(), json!(template));

		let context = Value::Object(context_map);

		let start = Instant::now();
		let result = renderer.render(&context, None).await;
		let duration = start.elapsed();

		assert!(result.is_ok());

		// Verify output contains all values
		let html = String::from_utf8(result.unwrap().to_vec()).unwrap();
		for i in 0..100 {
			assert!(html.contains(&format!("value{}", i)));
		}

		// Performance: Should complete in less than 100ms
		assert!(
			duration.as_millis() < 100,
			"Template rendering took {}ms, expected < 100ms",
			duration.as_millis()
		);
	}
}
