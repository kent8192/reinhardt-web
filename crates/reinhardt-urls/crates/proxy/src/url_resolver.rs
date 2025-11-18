//! URL resolver for reverse URL resolution
//!
//! This module provides Django-style reverse URL resolution, allowing you to
//! generate URLs from pattern names and parameters.

use crate::url_namespace::UrlNamespace;
use crate::url_pattern::UrlPattern;
use std::collections::HashMap;

/// A resolver for reverse URL resolution
#[derive(Debug)]
pub struct UrlResolver {
	/// Registered URL patterns
	patterns: Vec<UrlPattern>,
	/// Registered namespaces
	namespaces: Vec<UrlNamespace>,
}

impl UrlResolver {
	/// Creates a new URL resolver
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::UrlResolver;
	///
	/// let resolver = UrlResolver::new();
	/// ```
	pub fn new() -> Self {
		Self {
			patterns: Vec::new(),
			namespaces: Vec::new(),
		}
	}

	/// Adds a URL pattern to the resolver
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::{UrlResolver, UrlPattern};
	///
	/// let mut resolver = UrlResolver::new();
	/// resolver.add_pattern(UrlPattern::new("home", "/", None));
	/// ```
	pub fn add_pattern(&mut self, pattern: UrlPattern) {
		self.patterns.push(pattern);
	}

	/// Adds a namespace to the resolver
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::{UrlResolver, UrlNamespace};
	///
	/// let mut resolver = UrlResolver::new();
	/// let admin_ns = UrlNamespace::new("admin", "/admin/");
	/// resolver.add_namespace(admin_ns);
	/// ```
	pub fn add_namespace(&mut self, namespace: UrlNamespace) {
		self.namespaces.push(namespace);
	}

	/// Reverses a URL pattern name to its URL
	///
	/// # Arguments
	///
	/// * `name` - Pattern name (e.g., "home", "admin:users")
	/// * `kwargs` - Parameters to fill in the URL template
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Pattern name not found
	/// - Required parameters are missing
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::{UrlResolver, UrlPattern};
	/// use std::collections::HashMap;
	///
	/// let mut resolver = UrlResolver::new();
	/// resolver.add_pattern(UrlPattern::new("home", "/", None));
	///
	/// let url = resolver.reverse("home", HashMap::new()).unwrap();
	/// assert_eq!(url, "/");
	/// ```
	pub fn reverse(&self, name: &str, kwargs: HashMap<String, String>) -> Result<String, String> {
		// Find pattern by name
		let pattern = self
			.patterns
			.iter()
			.find(|p| p.name() == name)
			.ok_or_else(|| format!("URL pattern '{}' not found", name))?;

		// Build URL using pattern
		pattern.build_url(&kwargs)
	}

	/// Returns the number of registered patterns
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::{UrlResolver, UrlPattern};
	///
	/// let mut resolver = UrlResolver::new();
	/// assert_eq!(resolver.pattern_count(), 0);
	///
	/// resolver.add_pattern(UrlPattern::new("home", "/", None));
	/// assert_eq!(resolver.pattern_count(), 1);
	/// ```
	pub fn pattern_count(&self) -> usize {
		self.patterns.len()
	}

	/// Checks if a pattern with the given name is registered
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_proxy::{UrlResolver, UrlPattern};
	///
	/// let mut resolver = UrlResolver::new();
	/// resolver.add_pattern(UrlPattern::new("home", "/", None));
	///
	/// assert!(resolver.has_pattern("home"));
	/// assert!(!resolver.has_pattern("nonexistent"));
	/// ```
	pub fn has_pattern(&self, name: &str) -> bool {
		self.patterns.iter().any(|p| p.name() == name)
	}
}

impl Default for UrlResolver {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_resolver_creation() {
		let resolver = UrlResolver::new();
		assert_eq!(resolver.patterns.len(), 0);
		assert_eq!(resolver.namespaces.len(), 0);
	}

	#[test]
	fn test_add_pattern() {
		let mut resolver = UrlResolver::new();
		resolver.add_pattern(UrlPattern::new("home", "/", None));
		assert_eq!(resolver.patterns.len(), 1);
	}

	#[test]
	fn test_add_namespace() {
		let mut resolver = UrlResolver::new();
		let ns = UrlNamespace::new("admin", "/admin/");
		resolver.add_namespace(ns);
		assert_eq!(resolver.namespaces.len(), 1);
	}

	#[test]
	fn test_reverse_simple() {
		let mut resolver = UrlResolver::new();
		resolver.add_pattern(UrlPattern::new("home", "/", None));

		let url = resolver.reverse("home", HashMap::new()).unwrap();
		assert_eq!(url, "/");
	}

	#[test]
	fn test_reverse_with_kwargs() {
		let mut resolver = UrlResolver::new();
		resolver.add_pattern(UrlPattern::new("user-detail", "/users/<id>/", None));

		let mut kwargs = HashMap::new();
		kwargs.insert("id".to_string(), "123".to_string());

		let url = resolver.reverse("user-detail", kwargs).unwrap();
		assert_eq!(url, "/users/123/");
	}

	#[test]
	fn test_reverse_pattern_not_found() {
		let resolver = UrlResolver::new();
		let result = resolver.reverse("nonexistent", HashMap::new());
		assert!(result.is_err());
		assert_eq!(result.unwrap_err(), "URL pattern 'nonexistent' not found");
	}

	#[test]
	fn test_reverse_missing_parameter() {
		let mut resolver = UrlResolver::new();
		resolver.add_pattern(UrlPattern::new("user-detail", "/users/<id>/", None));

		let result = resolver.reverse("user-detail", HashMap::new());
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Missing required parameter"));
	}
}
