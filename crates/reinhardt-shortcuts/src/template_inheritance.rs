//! Template inheritance support using Tera template engine
//!
//! This module provides Django-style template inheritance with `{% extends %}`
//! and `{% block %}` syntax using the Tera template engine.

#[cfg(feature = "templates")]
use serde::Serialize;
#[cfg(feature = "templates")]
use std::collections::HashMap;
#[cfg(feature = "templates")]
use std::env;
#[cfg(feature = "templates")]
use std::path::PathBuf;
#[cfg(feature = "templates")]
use std::sync::{Arc, OnceLock};
#[cfg(feature = "templates")]
use tera::{Context, Tera};

/// Global Tera template engine instance
#[cfg(feature = "templates")]
static TERA_ENGINE: OnceLock<Arc<Tera>> = OnceLock::new();

/// Get or initialize the global Tera template engine
///
/// The template directory is determined by the `REINHARDT_TEMPLATE_DIR`
/// environment variable. If not set, defaults to the `templates` directory
/// relative to the crate root (determined at compile time).
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::template_inheritance::get_tera_engine;
///
/// let tera = get_tera_engine();
/// ```
#[cfg(feature = "templates")]
pub fn get_tera_engine() -> &'static Arc<Tera> {
    TERA_ENGINE.get_or_init(|| {
        let template_dir = env::var("REINHARDT_TEMPLATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Use CARGO_MANIFEST_DIR to get the crate directory at compile time
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                PathBuf::from(manifest_dir).join("templates")
            });

        let glob_pattern = format!("{}/**/*", template_dir.display());

        match Tera::new(&glob_pattern) {
            Ok(tera) => {
                eprintln!("Tera initialized successfully");
                eprintln!("Template directory: {}", template_dir.display());
                eprintln!("Registered templates:");
                for name in tera.get_template_names() {
                    eprintln!("  - {}", name);
                }
                Arc::new(tera)
            }
            Err(e) => {
                eprintln!("Warning: Failed to initialize Tera: {}", e);
                eprintln!("Template directory: {}", template_dir.display());
                eprintln!("Glob pattern: {}", glob_pattern);
                // Return empty Tera instance as fallback
                Arc::new(Tera::default())
            }
        }
    })
}

/// Render a template with inheritance support
///
/// This function supports full Django/Jinja2-style template syntax including:
/// - Template inheritance: `{% extends "base.html" %}`
/// - Template blocks: `{% block content %}...{% endblock %}`
/// - Variable substitution: `{{ variable }}`
/// - Control structures: `{% if %}`, `{% for %}`, etc.
/// - Filters: `{{ value|lower }}`
///
/// # Arguments
///
/// * `template_name` - The name of the template file (relative to template directory)
/// * `context` - A HashMap containing template variables
///
/// # Returns
///
/// The rendered template as a String
///
/// # Errors
///
/// Returns an error if:
/// - Template file is not found
/// - Template syntax is invalid
/// - Rendering fails
///
/// # Examples
///
/// ```ignore
/// use std::collections::HashMap;
/// use reinhardt_shortcuts::template_inheritance::render_with_inheritance;
///
/// let mut context = HashMap::new();
/// context.insert("title", "My Page");
/// context.insert("content", "Hello, World!");
///
/// let html = render_with_inheritance("page.html", &context)?;
/// ```
#[cfg(feature = "templates")]
pub fn render_with_inheritance<K, V>(
    template_name: &str,
    context: &HashMap<K, V>,
) -> Result<String, tera::Error>
where
    K: AsRef<str>,
    V: Serialize,
{
    let tera = get_tera_engine();

    // Convert HashMap to Tera Context
    let mut tera_context = Context::new();
    for (key, value) in context {
        // Serialize to serde_json::Value for Tera
        if let Ok(json_value) = serde_json::to_value(value) {
            tera_context.insert(key.as_ref(), &json_value);
        }
    }

    tera.render(template_name, &tera_context)
}

/// Render a template string with inheritance support
///
/// This function renders a template from a string instead of a file.
/// Useful for dynamic template generation or testing.
///
/// Note: Template inheritance (`{% extends %}`) requires file-based templates.
///
/// # Arguments
///
/// * `template_content` - The template source code
/// * `context` - A HashMap containing template variables
///
/// # Returns
///
/// The rendered template as a String
///
/// # Errors
///
/// Returns an error if template syntax is invalid or rendering fails
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use reinhardt_shortcuts::template_inheritance::render_string_with_inheritance;
///
/// let mut context = HashMap::new();
/// context.insert("name", "Alice");
///
/// let result = render_string_with_inheritance(
///     "Hello, {{ name }}!",
///     &context
/// ).unwrap();
/// assert_eq!(result, "Hello, Alice!");
/// ```
#[cfg(feature = "templates")]
pub fn render_string_with_inheritance<K, V>(
    template_content: &str,
    context: &HashMap<K, V>,
) -> Result<String, tera::Error>
where
    K: AsRef<str>,
    V: Serialize,
{
    let mut tera = Tera::default();
    tera.add_raw_template("__dynamic__", template_content)?;

    // Convert HashMap to Tera Context
    let mut tera_context = Context::new();
    for (key, value) in context {
        if let Ok(json_value) = serde_json::to_value(value) {
            tera_context.insert(key.as_ref(), &json_value);
        }
    }

    tera.render("__dynamic__", &tera_context)
}

/// Check if a template exists in the template directory
///
/// # Arguments
///
/// * `template_name` - The name of the template file
///
/// # Returns
///
/// `true` if the template exists, `false` otherwise
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::template_inheritance::template_exists;
///
/// if template_exists("custom.html") {
///     println!("Template found!");
/// }
/// ```
#[cfg(feature = "templates")]
pub fn template_exists(template_name: &str) -> bool {
    let tera = get_tera_engine();
    tera.get_template_names().any(|name| name == template_name)
}

#[cfg(all(test, feature = "templates"))]
mod tests {
    use super::*;

    #[test]
    fn test_render_string_simple() {
        let mut context = HashMap::new();
        context.insert("name", serde_json::json!("Alice"));
        context.insert("age", serde_json::json!("30"));

        let result =
            render_string_with_inheritance("Name: {{ name }}, Age: {{ age }}", &context).unwrap();

        assert_eq!(result, "Name: Alice, Age: 30");
    }

    #[test]
    fn test_render_string_with_if() {
        let mut context = HashMap::new();
        context.insert("show", serde_json::json!(true));
        context.insert("message", serde_json::json!("Hello!"));

        let template = "{% if show %}{{ message }}{% endif %}";
        let result = render_string_with_inheritance(template, &context).unwrap();

        assert_eq!(result, "Hello!");
    }

    #[test]
    fn test_render_string_with_for() {
        let mut context = HashMap::new();
        context.insert("items", serde_json::json!(vec!["a", "b", "c"]));

        let template = "{% for item in items %}{{ item }}{% endfor %}";
        let result = render_string_with_inheritance(template, &context).unwrap();

        assert_eq!(result, "abc");
    }

    #[test]
    fn test_render_string_with_filter() {
        let mut context = HashMap::new();
        context.insert("text", serde_json::json!("HELLO"));

        let template = "{{ text | lower }}";
        let result = render_string_with_inheritance(template, &context).unwrap();

        assert_eq!(result, "hello");
    }

    #[test]
    fn test_render_string_html() {
        let mut context = HashMap::new();
        context.insert("title", serde_json::json!("Test Page"));
        context.insert("content", serde_json::json!("Hello, World!"));

        let template =
            "<html><head><title>{{ title }}</title></head><body>{{ content }}</body></html>";
        let result = render_string_with_inheritance(template, &context).unwrap();

        assert!(result.contains("<title>Test Page</title>"));
        assert!(result.contains("<body>Hello, World!</body>"));
    }

    #[test]
    fn test_render_string_missing_variable() {
        let context: HashMap<String, serde_json::Value> = HashMap::new();

        let template = "Hello {{ name }}";
        let result = render_string_with_inheritance(template, &context);

        // Tera in strict mode returns an error for missing variables
        assert!(result.is_err(), "Expected an error for missing variable");
        let error_msg = result.unwrap_err().to_string();
        // Tera returns "Failed to render" for template errors
        assert!(
            error_msg.contains("Failed to render")
                || error_msg.contains("Variable")
                || error_msg.contains("not found")
                || error_msg.contains("Field")
                || error_msg.contains("name"),
            "Error message should indicate rendering failure, got: {}",
            error_msg
        );
    }

    #[test]
    fn test_render_string_no_variables() {
        let context: HashMap<String, serde_json::Value> = HashMap::new();

        let template = "<h1>Static Content</h1>";
        let result = render_string_with_inheritance(template, &context).unwrap();

        assert_eq!(result, "<h1>Static Content</h1>");
    }

    #[test]
    fn test_template_exists_returns_false_for_nonexistent() {
        // With no templates loaded, any check should return false
        assert!(!template_exists("nonexistent.html"));
    }
}
