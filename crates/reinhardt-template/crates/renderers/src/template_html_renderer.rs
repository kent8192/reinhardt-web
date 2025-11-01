//! Template HTML Renderer
//!
//! Renders HTML responses using Tera templates.
//! Supports Django-style template rendering with context data.

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;

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

	/// Performs single-pass variable substitution in a template string
	///
	/// Complexity: O(n + m) where n is template length, m is number of variables
	/// This replaces the previous O(n × m) multi-pass approach.
	///
	/// # Arguments
	///
	/// * `template` - Template string with `{{variable}}` placeholders
	/// * `context` - HashMap of variable names to their string values
	///
	/// # Returns
	///
	/// String with all variables substituted
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::TemplateHTMLRenderer;
	/// use std::collections::HashMap;
	///
	/// let mut context = HashMap::new();
	/// context.insert("title".to_string(), "Hello".to_string());
	/// context.insert("name".to_string(), "World".to_string());
	///
	/// let result = TemplateHTMLRenderer::substitute_variables_single_pass(
	///     "<h1>{{ title }}</h1><p>{{ name }}</p>",
	///     &context
	/// );
	///
	/// assert_eq!(result, "<h1>Hello</h1><p>World</p>");
	/// ```
	pub fn substitute_variables_single_pass(
		template: &str,
		context: &HashMap<String, String>,
	) -> String {
		let mut result = String::with_capacity(template.len());
		let mut chars = template.chars().peekable();

		while let Some(ch) = chars.next() {
			if ch == '{' && chars.peek() == Some(&'{') {
				chars.next(); // Skip second '{'

				// Extract variable name
				let mut var_name = String::new();
				let mut found_closing = false;

				// Collect characters until we find "}}"
				while let Some(&c) = chars.peek() {
					if c == '}' {
						chars.next(); // Consume first '}'
						if chars.peek() == Some(&'}') {
							chars.next(); // Consume second '}'
							found_closing = true;
							break;
						} else {
							// Not a closing }}, add it to var_name
							var_name.push('}');
						}
					} else if c == '{' {
						// Nested {{ is invalid, stop parsing
						break;
					} else {
						var_name.push(c);
						chars.next();
					}
				}

				if found_closing {
					// Trim whitespace from variable name for lookup
					let var_name_trimmed = var_name.trim();

					// HashMap lookup: O(1) amortized
					if let Some(value) = context.get(var_name_trimmed) {
						result.push_str(value);
					} else {
						// Variable not found, preserve placeholder with original spacing
						result.push_str("{{ ");
						result.push_str(var_name_trimmed);
						result.push_str(" }}");
					}
				} else {
					// Invalid format, restore original characters
					result.push_str("{{");
					result.push_str(&var_name);
				}
			} else {
				result.push(ch);
			}
		}

		result
	}

	/// Renders a template with the given context data
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
			// Build context HashMap for single-pass substitution
			let mut var_context = HashMap::new();

			if let Value::Object(map) = context {
				for (key, value) in map {
					if key == "template_string" || key == "template_name" {
						continue;
					}

					let replacement = match value {
						Value::String(s) => s.clone(),
						Value::Number(n) => n.to_string(),
						Value::Bool(b) => b.to_string(),
						Value::Null => String::new(),
						_ => serde_json::to_string(value).unwrap_or_default(),
					};

					var_context.insert(key.clone(), replacement);
				}
			}

			// Single-pass variable substitution: O(n + m)
			let output = Self::substitute_variables_single_pass(template_str, &var_context);
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

	// Tests for single-pass variable substitution

	#[test]
	fn test_substitute_single_variable() {
		let mut context = HashMap::new();
		context.insert("title".to_string(), "Hello World".to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"<h1>{{ title }}</h1>",
			&context,
		);

		assert_eq!(result, "<h1>Hello World</h1>");
	}

	#[test]
	fn test_substitute_multiple_variables() {
		let mut context = HashMap::new();
		context.insert("title".to_string(), "Welcome".to_string());
		context.insert("name".to_string(), "Alice".to_string());
		context.insert("count".to_string(), "42".to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"<h1>{{ title }}</h1><p>Hello, {{ name }}!</p><span>Count: {{ count }}</span>",
			&context,
		);

		assert_eq!(
			result,
			"<h1>Welcome</h1><p>Hello, Alice!</p><span>Count: 42</span>"
		);
	}

	#[test]
	fn test_substitute_consecutive_variables() {
		let mut context = HashMap::new();
		context.insert("a".to_string(), "foo".to_string());
		context.insert("b".to_string(), "bar".to_string());

		let result =
			TemplateHTMLRenderer::substitute_variables_single_pass("{{ a }}{{ b }}", &context);

		assert_eq!(result, "foobar");
	}

	#[test]
	fn test_substitute_with_whitespace() {
		let mut context = HashMap::new();
		context.insert("var".to_string(), "value".to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"{{var}} {{ var }} {{  var  }}",
			&context,
		);

		assert_eq!(result, "value value value");
	}

	#[test]
	fn test_substitute_missing_variable() {
		let context = HashMap::new();

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"<h1>{{ missing }}</h1>",
			&context,
		);

		// Missing variables should be preserved as placeholders
		assert_eq!(result, "<h1>{{ missing }}</h1>");
	}

	#[test]
	fn test_substitute_empty_template() {
		let context = HashMap::new();

		let result = TemplateHTMLRenderer::substitute_variables_single_pass("", &context);

		assert_eq!(result, "");
	}

	#[test]
	fn test_substitute_empty_context() {
		let context = HashMap::new();

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"<h1>No variables here</h1>",
			&context,
		);

		assert_eq!(result, "<h1>No variables here</h1>");
	}

	#[test]
	fn test_substitute_invalid_single_brace() {
		let mut context = HashMap::new();
		context.insert("var".to_string(), "value".to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"{ var } {var} {{var}",
			&context,
		);

		// Single braces should be preserved, incomplete {{ should be preserved
		assert_eq!(result, "{ var } {var} {{var}");
	}

	#[test]
	fn test_substitute_nested_braces() {
		let mut context = HashMap::new();
		context.insert("a".to_string(), "value".to_string());

		let result =
			TemplateHTMLRenderer::substitute_variables_single_pass("{{{{ a }}}}", &context);

		// Nested {{ is invalid: outer {{ is ignored, inner {{ a }} is processed
		// Result: "{{" + "value" + "}}"
		assert_eq!(result, "{{value}}");
	}

	#[test]
	fn test_substitute_mismatched_braces() {
		let mut context = HashMap::new();
		context.insert("var".to_string(), "value".to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass("{{ var }", &context);

		// Missing closing }} should preserve the template
		assert_eq!(result, "{{ var }");
	}

	#[test]
	fn test_substitute_many_variables() {
		let mut context = HashMap::new();
		let mut template = String::new();
		let mut expected = String::new();

		// Create 100 variables
		for i in 0..100 {
			let var_name = format!("var{}", i);
			let var_value = format!("value{}", i);
			context.insert(var_name.clone(), var_value.clone());

			template.push_str(&format!("{{{{ {} }}}}", var_name));
			expected.push_str(&var_value);
		}

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(&template, &context);

		assert_eq!(result, expected);
	}

	#[test]
	fn test_substitute_special_characters() {
		let mut context = HashMap::new();
		context.insert(
			"html".to_string(),
			"<script>alert('xss')</script>".to_string(),
		);
		context.insert("quotes".to_string(), r#"He said "Hello""#.to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"<div>{{ html }}</div><p>{{ quotes }}</p>",
			&context,
		);

		// Note: This test shows that the renderer does NOT escape HTML
		// HTML escaping should be handled by a separate layer
		assert_eq!(
			result,
			r#"<div><script>alert('xss')</script></div><p>He said "Hello"</p>"#
		);
	}

	#[tokio::test]
	async fn test_render_with_single_pass_algorithm() {
		let renderer = TemplateHTMLRenderer::new();

		// Create a template with multiple variables
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

	#[test]
	fn test_substitute_performance_many_variables() {
		use std::time::Instant;

		let mut context = HashMap::new();
		let mut template = String::new();

		// Create 1000 variables
		for i in 0..1000 {
			let var_name = format!("var{}", i);
			let var_value = format!("value{}", i);
			context.insert(var_name.clone(), var_value.clone());

			template.push_str(&format!("<div>{{{{ {} }}}}</div>", var_name));
		}

		// Measure single-pass substitution time
		let start = Instant::now();
		let result = TemplateHTMLRenderer::substitute_variables_single_pass(&template, &context);
		let duration = start.elapsed();

		// Verify correctness
		for i in 0..1000 {
			let expected = format!("<div>value{}</div>", i);
			assert!(result.contains(&expected));
		}

		// Performance assertion: Should complete in less than 100ms
		// On typical hardware, single-pass should be much faster
		assert!(
			duration.as_millis() < 100,
			"Single-pass substitution took {}ms, expected < 100ms",
			duration.as_millis()
		);
	}

	#[test]
	fn test_substitute_performance_long_template() {
		use std::time::Instant;

		let mut context = HashMap::new();
		context.insert("title".to_string(), "Test Title".to_string());
		context.insert("content".to_string(), "Test Content".to_string());

		// Create a template with 10,000 lines
		let mut template = String::new();
		for _ in 0..10_000 {
			template.push_str("<h1>{{ title }}</h1><p>{{ content }}</p>");
		}

		let start = Instant::now();
		let result = TemplateHTMLRenderer::substitute_variables_single_pass(&template, &context);
		let duration = start.elapsed();

		// Verify correctness
		assert!(result.contains("<h1>Test Title</h1>"));
		assert!(result.contains("<p>Test Content</p>"));

		// Performance assertion: Should complete in less than 200ms
		assert!(
			duration.as_millis() < 200,
			"Long template substitution took {}ms, expected < 200ms",
			duration.as_millis()
		);
	}

	#[test]
	fn test_substitute_edge_case_empty_variable_name() {
		let context = HashMap::new();

		let result = TemplateHTMLRenderer::substitute_variables_single_pass("{{  }}", &context);

		// Empty variable name should be preserved as placeholder
		assert_eq!(result, "{{  }}");
	}

	#[test]
	fn test_substitute_edge_case_single_closing_brace_in_variable() {
		let mut context = HashMap::new();
		context.insert("var".to_string(), "value".to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass("{{ var} }}", &context);

		// Variable name is "var} ", which doesn't exist in context
		assert_eq!(result, "{{ var} }}");
	}

	#[test]
	fn test_substitute_unicode_variable_names() {
		let mut context = HashMap::new();
		context.insert("変数".to_string(), "値".to_string());
		context.insert("título".to_string(), "contenido".to_string());

		let result = TemplateHTMLRenderer::substitute_variables_single_pass(
			"<p>{{ 変数 }}</p><h1>{{ título }}</h1>",
			&context,
		);

		assert_eq!(result, "<p>値</p><h1>contenido</h1>");
	}
}
