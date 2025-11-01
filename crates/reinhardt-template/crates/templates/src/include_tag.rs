//! Include tag implementation for templates
//!
//! Provides functionality to include one template within another.
//! This is useful for:
//! - Reusing common template fragments (headers, footers, navigation)
//! - Breaking down large templates into smaller, manageable pieces
//! - Sharing template components across multiple pages
//!
//! Note: Tera already provides `{% include %}` syntax. This module provides
//! additional helpers for dynamic template inclusion and context management.

use reinhardt_exception::{Error, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Template include manager
pub struct TemplateIncludeManager {
	/// Base directory for template files
	base_dir: PathBuf,
	/// Cache of loaded templates
	cache: HashMap<String, String>,
}

impl TemplateIncludeManager {
	/// Create a new template include manager
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_templates::TemplateIncludeManager;
	/// use std::path::Path;
	///
	/// let manager = TemplateIncludeManager::new(Path::new("templates"));
	/// ```
	pub fn new(base_dir: impl AsRef<Path>) -> Self {
		Self {
			base_dir: base_dir.as_ref().to_path_buf(),
			cache: HashMap::new(),
		}
	}

	/// Load a template from the file system
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_templates::TemplateIncludeManager;
	/// use std::path::Path;
	///
	/// let manager = TemplateIncludeManager::new(Path::new("templates"));
	/// let content = manager.load_template("header.html").unwrap();
	/// ```
	pub fn load_template(&self, name: &str) -> Result<String> {
		let path = self.base_dir.join(name);

		if !path.exists() {
			return Err(Error::TemplateNotFound(name.to_string()));
		}

		fs::read_to_string(&path).map_err(|e| {
			Error::ImproperlyConfigured(format!("Failed to read template '{}': {}", name, e))
		})
	}

	/// Load a template with caching
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_templates::TemplateIncludeManager;
	/// use std::path::Path;
	///
	/// let mut manager = TemplateIncludeManager::new(Path::new("templates"));
	/// let content = manager.load_template_cached("header.html").unwrap();
	/// // Second call uses cache
	/// let content2 = manager.load_template_cached("header.html").unwrap();
	/// ```
	pub fn load_template_cached(&mut self, name: &str) -> Result<String> {
		if let Some(cached) = self.cache.get(name) {
			return Ok(cached.clone());
		}

		let content = self.load_template(name)?;
		self.cache.insert(name.to_string(), content.clone());
		Ok(content)
	}

	/// Clear the template cache
	pub fn clear_cache(&mut self) {
		self.cache.clear();
	}

	/// Check if a template exists
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_templates::TemplateIncludeManager;
	/// use std::path::Path;
	///
	/// let manager = TemplateIncludeManager::new(Path::new("templates"));
	/// if manager.template_exists("header.html") {
	///     println!("Template exists");
	/// }
	/// ```
	pub fn template_exists(&self, name: &str) -> bool {
		self.base_dir.join(name).exists()
	}

	/// Get the full path to a template
	pub fn template_path(&self, name: &str) -> PathBuf {
		self.base_dir.join(name)
	}
}

/// Include a template snippet
///
/// This is a helper function for including template snippets.
/// In practice, Tera's built-in `{% include %}` should be preferred.
///
/// # Examples
///
/// ```
/// use reinhardt_templates::include_template;
///
/// let snippet = "<div>Header</div>";
/// let result = include_template(snippet);
/// assert_eq!(result, "<div>Header</div>");
/// ```
pub fn include_template(content: &str) -> String {
	content.to_string()
}

/// Process includes in a template string
///
/// Replaces `{{ include "template.html" }}` markers with actual template content.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_templates::{process_includes, TemplateIncludeManager};
/// use std::path::Path;
///
/// let manager = TemplateIncludeManager::new(Path::new("templates"));
/// let template = r#"<header>{{ include "header.html" }}</header>"#;
/// let result = process_includes(template, &manager).unwrap();
/// ```
pub fn process_includes(template: &str, manager: &TemplateIncludeManager) -> Result<String> {
	let mut result = template.to_string();
	let include_pattern = regex::Regex::new(r#"\{\{\s*include\s+"([^"]+)"\s*\}\}"#).unwrap();

	for cap in include_pattern.captures_iter(template) {
		let full_match = &cap[0];
		let template_name = &cap[1];

		let included_content = manager.load_template(template_name)?;
		result = result.replace(full_match, &included_content);
	}

	Ok(result)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;
	use tempfile::TempDir;

	fn setup_test_templates() -> TempDir {
		let dir = TempDir::new().unwrap();
		let base_path = dir.path();

		fs::write(base_path.join("header.html"), "<header>Header</header>").unwrap();
		fs::write(base_path.join("footer.html"), "<footer>Footer</footer>").unwrap();
		fs::write(
			base_path.join("nav.html"),
			"<nav><a href='/'>Home</a></nav>",
		)
		.unwrap();

		dir
	}

	#[test]
	fn test_manager_new() {
		let dir = TempDir::new().unwrap();
		let manager = TemplateIncludeManager::new(dir.path());
		assert_eq!(manager.base_dir, dir.path());
	}

	#[test]
	fn test_load_template() {
		let dir = setup_test_templates();
		let manager = TemplateIncludeManager::new(dir.path());

		let content = manager.load_template("header.html").unwrap();
		assert_eq!(content, "<header>Header</header>");
	}

	#[test]
	fn test_load_template_not_found() {
		let dir = TempDir::new().unwrap();
		let manager = TemplateIncludeManager::new(dir.path());

		let result = manager.load_template("nonexistent.html");
		assert!(result.is_err());
	}

	#[test]
	fn test_load_template_cached() {
		let dir = setup_test_templates();
		let mut manager = TemplateIncludeManager::new(dir.path());

		let content1 = manager.load_template_cached("header.html").unwrap();
		let content2 = manager.load_template_cached("header.html").unwrap();

		assert_eq!(content1, content2);
		assert_eq!(content1, "<header>Header</header>");
	}

	#[test]
	fn test_clear_cache() {
		let dir = setup_test_templates();
		let mut manager = TemplateIncludeManager::new(dir.path());

		manager.load_template_cached("header.html").unwrap();
		assert!(!manager.cache.is_empty());

		manager.clear_cache();
		assert!(manager.cache.is_empty());
	}

	#[test]
	fn test_template_exists() {
		let dir = setup_test_templates();
		let manager = TemplateIncludeManager::new(dir.path());

		assert!(manager.template_exists("header.html"));
		assert!(manager.template_exists("footer.html"));
		assert!(!manager.template_exists("nonexistent.html"));
	}

	#[test]
	fn test_template_path() {
		let dir = TempDir::new().unwrap();
		let manager = TemplateIncludeManager::new(dir.path());

		let path = manager.template_path("test.html");
		assert_eq!(path, dir.path().join("test.html"));
	}

	#[test]
	fn test_include_template() {
		let snippet = "<div>Content</div>";
		let result = include_template(snippet);
		assert_eq!(result, "<div>Content</div>");
	}

	#[test]
	fn test_process_includes() {
		let dir = setup_test_templates();
		let manager = TemplateIncludeManager::new(dir.path());

		let template = r#"<html>{{ include "header.html" }}<main>Content</main>{{ include "footer.html" }}</html>"#;
		let result = process_includes(template, &manager).unwrap();

		assert!(result.contains("<header>Header</header>"));
		assert!(result.contains("<footer>Footer</footer>"));
		assert!(result.contains("<main>Content</main>"));
	}

	#[test]
	fn test_process_includes_not_found() {
		let dir = TempDir::new().unwrap();
		let manager = TemplateIncludeManager::new(dir.path());

		let template = r#"{{ include "nonexistent.html" }}"#;
		let result = process_includes(template, &manager);

		assert!(result.is_err());
	}
}
