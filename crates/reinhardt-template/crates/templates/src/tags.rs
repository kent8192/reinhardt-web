//! Custom template tags for Tera
//!
//! Provides a system for registering and using custom template tags beyond
//! the standard filters. Template tags can:
//! - Generate complex HTML structures
//! - Perform conditional logic
//! - Loop over data
//! - Include other templates
//!
//! This module provides helpers for common tag patterns and custom tag registration.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Template tag function type
///
/// Takes arguments as a HashMap and returns a rendered string
pub type TagFunction = Arc<dyn Fn(&HashMap<String, String>) -> String + Send + Sync>;

/// Registry for custom template tags
#[derive(Clone)]
pub struct TemplateTagRegistry {
	tags: Arc<RwLock<HashMap<String, TagFunction>>>,
}

impl TemplateTagRegistry {
	/// Create a new template tag registry
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_templates::TemplateTagRegistry;
	///
	/// let registry = TemplateTagRegistry::new();
	/// ```
	pub fn new() -> Self {
		Self {
			tags: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Register a custom tag
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_templates::TemplateTagRegistry;
	/// use std::collections::HashMap;
	///
	/// let mut registry = TemplateTagRegistry::new();
	/// registry.register("greet", |args| {
	///     let name = args.get("name").map(|s| s.as_str()).unwrap_or("World");
	///     format!("Hello, {}!", name)
	/// });
	///
	/// let mut args = HashMap::new();
	/// args.insert("name".to_string(), "Alice".to_string());
	/// let result = registry.render("greet", &args).unwrap();
	/// assert_eq!(result, "Hello, Alice!");
	/// ```
	pub fn register<F>(&mut self, name: impl Into<String>, tag_fn: F)
	where
		F: Fn(&HashMap<String, String>) -> String + Send + Sync + 'static,
	{
		if let Ok(mut tags) = self.tags.write() {
			tags.insert(name.into(), Arc::new(tag_fn));
		}
	}

	/// Render a registered tag
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_templates::TemplateTagRegistry;
	/// use std::collections::HashMap;
	///
	/// let mut registry = TemplateTagRegistry::new();
	/// registry.register("simple", |_| "Simple tag output".to_string());
	///
	/// let args = HashMap::new();
	/// let result = registry.render("simple", &args).unwrap();
	/// assert_eq!(result, "Simple tag output");
	///
	/// let error = registry.render("nonexistent", &args);
	/// assert!(error.is_none());
	/// ```
	pub fn render(&self, name: &str, args: &HashMap<String, String>) -> Option<String> {
		if let Ok(tags) = self.tags.read() {
			tags.get(name).map(|tag_fn| tag_fn(args))
		} else {
			None
		}
	}

	/// Check if a tag is registered
	pub fn has_tag(&self, name: &str) -> bool {
		if let Ok(tags) = self.tags.read() {
			tags.contains_key(name)
		} else {
			false
		}
	}

	/// Clear all registered tags
	pub fn clear(&mut self) {
		if let Ok(mut tags) = self.tags.write() {
			tags.clear();
		}
	}
}

impl Default for TemplateTagRegistry {
	fn default() -> Self {
		Self::new()
	}
}

/// Built-in tag: Generate a CSS class attribute
///
/// # Examples
///
/// ```
/// use reinhardt_templates::css_class_tag;
/// use std::collections::HashMap;
///
/// let mut args = HashMap::new();
/// args.insert("class".to_string(), "btn btn-primary".to_string());
/// let result = css_class_tag(&args);
/// assert_eq!(result, r#"class="btn btn-primary""#);
///
/// let empty = css_class_tag(&HashMap::new());
/// assert_eq!(empty, "");
/// ```
pub fn css_class_tag(args: &HashMap<String, String>) -> String {
	if let Some(class) = args.get("class") {
		format!(r#"class="{}""#, class)
	} else {
		String::new()
	}
}

/// Built-in tag: Generate an HTML link
///
/// # Examples
///
/// ```
/// use reinhardt_templates::link_tag;
/// use std::collections::HashMap;
///
/// let mut args = HashMap::new();
/// args.insert("href".to_string(), "/home".to_string());
/// args.insert("text".to_string(), "Home".to_string());
/// let result = link_tag(&args);
/// assert_eq!(result, r#"<a href="/home">Home</a>"#);
///
/// let mut args = HashMap::new();
/// args.insert("href".to_string(), "/about".to_string());
/// args.insert("text".to_string(), "About".to_string());
/// args.insert("class".to_string(), "nav-link".to_string());
/// let result = link_tag(&args);
/// assert_eq!(result, r#"<a href="/about" class="nav-link">About</a>"#);
/// ```
pub fn link_tag(args: &HashMap<String, String>) -> String {
	let href = args.get("href").map(|s| s.as_str()).unwrap_or("#");
	let text = args.get("text").map(|s| s.as_str()).unwrap_or("");
	let class = args.get("class");

	if let Some(class) = class {
		format!(r#"<a href="{}" class="{}">{}</a>"#, href, class, text)
	} else {
		format!(r#"<a href="{}">{}</a>"#, href, text)
	}
}

/// Built-in tag: Generate an HTML image
///
/// # Examples
///
/// ```
/// use reinhardt_templates::image_tag;
/// use std::collections::HashMap;
///
/// let mut args = HashMap::new();
/// args.insert("src".to_string(), "/static/logo.png".to_string());
/// args.insert("alt".to_string(), "Logo".to_string());
/// let result = image_tag(&args);
/// assert_eq!(result, r#"<img src="/static/logo.png" alt="Logo" />"#);
///
/// let mut args = HashMap::new();
/// args.insert("src".to_string(), "/image.jpg".to_string());
/// args.insert("alt".to_string(), "Image".to_string());
/// args.insert("class".to_string(), "img-fluid".to_string());
/// let result = image_tag(&args);
/// assert_eq!(result, r#"<img src="/image.jpg" alt="Image" class="img-fluid" />"#);
/// ```
pub fn image_tag(args: &HashMap<String, String>) -> String {
	let src = args.get("src").map(|s| s.as_str()).unwrap_or("");
	let alt = args.get("alt").map(|s| s.as_str()).unwrap_or("");
	let class = args.get("class");

	if let Some(class) = class {
		format!(r#"<img src="{}" alt="{}" class="{}" />"#, src, alt, class)
	} else {
		format!(r#"<img src="{}" alt="{}" />"#, src, alt)
	}
}

/// Built-in tag: Generate a Bootstrap alert
///
/// # Examples
///
/// ```
/// use reinhardt_templates::alert_tag;
/// use std::collections::HashMap;
///
/// let mut args = HashMap::new();
/// args.insert("message".to_string(), "Success!".to_string());
/// args.insert("type".to_string(), "success".to_string());
/// let result = alert_tag(&args);
/// assert!(result.contains("alert-success"));
/// assert!(result.contains("Success!"));
/// ```
pub fn alert_tag(args: &HashMap<String, String>) -> String {
	let message = args.get("message").map(|s| s.as_str()).unwrap_or("");
	let alert_type = args.get("type").map(|s| s.as_str()).unwrap_or("info");

	format!(
		r#"<div class="alert alert-{}" role="alert">{}</div>"#,
		alert_type, message
	)
}

/// Built-in tag: Generate a breadcrumb navigation
///
/// # Examples
///
/// ```
/// use reinhardt_templates::breadcrumb_tag;
/// use std::collections::HashMap;
///
/// let mut args = HashMap::new();
/// args.insert("items".to_string(), "Home,/;Products,/products".to_string());
/// let result = breadcrumb_tag(&args);
/// assert!(result.contains("breadcrumb"));
/// assert!(result.contains("Home"));
/// assert!(result.contains("Products"));
/// ```
pub fn breadcrumb_tag(args: &HashMap<String, String>) -> String {
	let items = args.get("items").map(|s| s.as_str()).unwrap_or("");

	let mut html = String::from(r#"<nav aria-label="breadcrumb"><ol class="breadcrumb">"#);

	for item in items.split(';') {
		let parts: Vec<&str> = item.split(',').collect();
		if parts.len() == 2 {
			let (text, href) = (parts[0], parts[1]);
			html.push_str(&format!(
				r#"<li class="breadcrumb-item"><a href="{}">{}</a></li>"#,
				href, text
			));
		}
	}

	html.push_str("</ol></nav>");
	html
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_registry_new() {
		let registry = TemplateTagRegistry::new();
		assert!(!registry.has_tag("any"));
	}

	#[test]
	fn test_registry_register() {
		let mut registry = TemplateTagRegistry::new();
		registry.register("test", |_| "test output".to_string());

		assert!(registry.has_tag("test"));
	}

	#[test]
	fn test_registry_render() {
		let mut registry = TemplateTagRegistry::new();
		registry.register("greet", |args| {
			let name = args.get("name").map(|s| s.as_str()).unwrap_or("World");
			format!("Hello, {}!", name)
		});

		let mut args = HashMap::new();
		args.insert("name".to_string(), "Alice".to_string());

		let result = registry.render("greet", &args);
		assert_eq!(result, Some("Hello, Alice!".to_string()));
	}

	#[test]
	fn test_registry_render_nonexistent() {
		let registry = TemplateTagRegistry::new();
		let result = registry.render("nonexistent", &HashMap::new());
		assert!(result.is_none());
	}

	#[test]
	fn test_registry_clear() {
		let mut registry = TemplateTagRegistry::new();
		registry.register("test", |_| "test".to_string());

		assert!(registry.has_tag("test"));

		registry.clear();
		assert!(!registry.has_tag("test"));
	}

	#[test]
	fn test_css_class_tag() {
		let mut args = HashMap::new();
		args.insert("class".to_string(), "btn btn-primary".to_string());

		let result = css_class_tag(&args);
		assert_eq!(result, r#"class="btn btn-primary""#);

		let empty = css_class_tag(&HashMap::new());
		assert_eq!(empty, "");
	}

	#[test]
	fn test_link_tag() {
		let mut args = HashMap::new();
		args.insert("href".to_string(), "/home".to_string());
		args.insert("text".to_string(), "Home".to_string());

		let result = link_tag(&args);
		assert_eq!(result, r#"<a href="/home">Home</a>"#);

		args.insert("class".to_string(), "nav-link".to_string());
		let result = link_tag(&args);
		assert_eq!(result, r#"<a href="/home" class="nav-link">Home</a>"#);
	}

	#[test]
	fn test_image_tag() {
		let mut args = HashMap::new();
		args.insert("src".to_string(), "/logo.png".to_string());
		args.insert("alt".to_string(), "Logo".to_string());

		let result = image_tag(&args);
		assert_eq!(result, r#"<img src="/logo.png" alt="Logo" />"#);

		args.insert("class".to_string(), "img-fluid".to_string());
		let result = image_tag(&args);
		assert_eq!(
			result,
			r#"<img src="/logo.png" alt="Logo" class="img-fluid" />"#
		);
	}

	#[test]
	fn test_alert_tag() {
		let mut args = HashMap::new();
		args.insert("message".to_string(), "Success!".to_string());
		args.insert("type".to_string(), "success".to_string());

		let result = alert_tag(&args);
		assert!(result.contains("alert-success"));
		assert!(result.contains("Success!"));
	}

	#[test]
	fn test_breadcrumb_tag() {
		let mut args = HashMap::new();
		args.insert("items".to_string(), "Home,/;Products,/products".to_string());

		let result = breadcrumb_tag(&args);
		assert!(result.contains("breadcrumb"));
		assert!(result.contains("Home"));
		assert!(result.contains("Products"));
	}
}
