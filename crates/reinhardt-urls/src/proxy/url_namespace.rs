//! URL namespace for organizing related URL patterns
//!
//! This module provides Django-style URL namespaces for grouping related
//! URL patterns under a common prefix.

/// A namespace for grouping related URL patterns
#[derive(Debug, Clone)]
pub struct UrlNamespace {
	/// Namespace name (e.g., "admin", "api")
	name: String,
	/// URL prefix for all patterns in this namespace (e.g., "/admin/", "/api/v1/")
	prefix: String,
}

impl UrlNamespace {
	/// Creates a new URL namespace
	///
	/// # Arguments
	///
	/// * `name` - Namespace name for reverse resolution
	/// * `prefix` - URL prefix to prepend to all patterns in this namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::UrlNamespace;
	///
	/// let admin_ns = UrlNamespace::new("admin", "/admin/");
	/// assert_eq!(admin_ns.name(), "admin");
	/// assert_eq!(admin_ns.prefix(), "/admin/");
	/// ```
	pub fn new(name: impl Into<String>, prefix: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			prefix: prefix.into(),
		}
	}

	/// Returns the namespace name
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Returns the URL prefix
	pub fn prefix(&self) -> &str {
		&self.prefix
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_namespace_creation() {
		let ns = UrlNamespace::new("admin", "/admin/");
		assert_eq!(ns.name(), "admin");
		assert_eq!(ns.prefix(), "/admin/");
	}

	#[rstest]
	fn test_namespace_with_nested_prefix() {
		let ns = UrlNamespace::new("api", "/api/v1/");
		assert_eq!(ns.name(), "api");
		assert_eq!(ns.prefix(), "/api/v1/");
	}

	#[rstest]
	fn test_namespace_clone() {
		let ns = UrlNamespace::new("admin", "/admin/");
		let cloned = ns.clone();
		assert_eq!(cloned.name(), ns.name());
		assert_eq!(cloned.prefix(), ns.prefix());
	}
}
