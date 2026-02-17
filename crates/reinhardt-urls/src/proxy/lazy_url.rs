//! Lazy URL resolution for deferred URL generation
//!
//! This module provides Django-style lazy URL objects that defer URL resolution
//! until the actual URL string is needed.

use crate::proxy::url_resolver::UrlResolver;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A lazy URL that defers resolution until needed
#[derive(Clone)]
pub struct LazyUrl {
	/// Pattern name to resolve
	name: String,
	/// Parameters for URL generation
	kwargs: HashMap<String, String>,
	/// Shared resolver instance
	resolver: Arc<UrlResolver>,
	/// Cached resolved URL
	cached_url: Arc<RwLock<Option<String>>>,
}

impl LazyUrl {
	/// Creates a new lazy URL without parameters
	///
	/// # Arguments
	///
	/// * `name` - Pattern name to resolve (e.g., "home", "admin:users")
	/// * `resolver` - Shared resolver instance
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{LazyUrl, UrlResolver, UrlPattern};
	/// use std::sync::Arc;
	///
	/// let mut resolver = UrlResolver::new();
	/// resolver.add_pattern(UrlPattern::new("home", "/", None));
	/// let resolver = Arc::new(resolver);
	///
	/// let home_url = LazyUrl::new("home", resolver.clone());
	/// assert_eq!(home_url.resolve(), "/");
	/// ```
	pub fn new(name: impl Into<String>, resolver: Arc<UrlResolver>) -> Self {
		Self {
			name: name.into(),
			kwargs: HashMap::new(),
			resolver,
			cached_url: Arc::new(RwLock::new(None)),
		}
	}

	/// Creates a new lazy URL with parameters
	///
	/// # Arguments
	///
	/// * `name` - Pattern name to resolve
	/// * `kwargs` - Parameters for URL generation
	/// * `resolver` - Shared resolver instance
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{LazyUrl, UrlResolver, UrlPattern};
	/// use std::sync::Arc;
	/// use std::collections::HashMap;
	///
	/// let mut resolver = UrlResolver::new();
	/// resolver.add_pattern(UrlPattern::new("user-detail", "/users/<id>/", None));
	/// let resolver = Arc::new(resolver);
	///
	/// let mut kwargs = HashMap::new();
	/// kwargs.insert("id".to_string(), "123".to_string());
	///
	/// let user_url = LazyUrl::with_kwargs("user-detail", kwargs, resolver.clone());
	/// assert_eq!(user_url.resolve(), "/users/123/");
	/// ```
	pub fn with_kwargs(
		name: impl Into<String>,
		kwargs: HashMap<String, String>,
		resolver: Arc<UrlResolver>,
	) -> Self {
		Self {
			name: name.into(),
			kwargs,
			resolver,
			cached_url: Arc::new(RwLock::new(None)),
		}
	}

	/// Checks if the URL has been resolved
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{LazyUrl, UrlResolver, UrlPattern};
	/// use std::sync::Arc;
	///
	/// let mut resolver = UrlResolver::new();
	/// resolver.add_pattern(UrlPattern::new("home", "/", None));
	/// let resolver = Arc::new(resolver);
	///
	/// let home_url = LazyUrl::new("home", resolver.clone());
	/// assert!(!home_url.is_resolved());
	///
	/// home_url.resolve();
	/// assert!(home_url.is_resolved());
	/// ```
	pub fn is_resolved(&self) -> bool {
		self.cached_url.read().unwrap().is_some()
	}

	/// Resolves the URL to its string representation
	///
	/// This method will cache the result for subsequent calls.
	///
	/// # Panics
	///
	/// Panics if:
	/// - Pattern name is not found
	/// - Required parameters are missing
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::proxy::{LazyUrl, UrlResolver, UrlPattern};
	/// use std::sync::Arc;
	///
	/// let mut resolver = UrlResolver::new();
	/// resolver.add_pattern(UrlPattern::new("home", "/", None));
	/// let resolver = Arc::new(resolver);
	///
	/// let home_url = LazyUrl::new("home", resolver.clone());
	/// assert_eq!(home_url.resolve(), "/");
	/// ```
	pub fn resolve(&self) -> String {
		// Check if already cached
		{
			let cached = self.cached_url.read().unwrap();
			if let Some(url) = cached.as_ref() {
				return url.clone();
			}
		}

		// Resolve the URL
		let url = self
			.resolver
			.reverse(&self.name, self.kwargs.clone())
			.unwrap_or_else(|e| panic!("Failed to resolve URL '{}': {}", self.name, e));

		// Cache the result
		{
			let mut cached = self.cached_url.write().unwrap();
			*cached = Some(url.clone());
		}

		url
	}
}

impl std::fmt::Display for LazyUrl {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.resolve())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::proxy::url_pattern::UrlPattern;
	use rstest::rstest;

	fn setup_resolver() -> Arc<UrlResolver> {
		let mut resolver = UrlResolver::new();
		resolver.add_pattern(UrlPattern::new("home", "/", None));
		resolver.add_pattern(UrlPattern::new("about", "/about/", None));
		resolver.add_pattern(UrlPattern::new("user-detail", "/users/<id>/", None));
		Arc::new(resolver)
	}

	#[rstest]
	fn test_lazy_url_creation() {
		let resolver = setup_resolver();
		let home_url = LazyUrl::new("home", resolver.clone());
		assert!(!home_url.is_resolved());
	}

	#[rstest]
	fn test_lazy_url_resolve() {
		let resolver = setup_resolver();
		let home_url = LazyUrl::new("home", resolver.clone());
		assert_eq!(home_url.resolve(), "/");
		assert!(home_url.is_resolved());
	}

	#[rstest]
	fn test_lazy_url_with_kwargs() {
		let resolver = setup_resolver();
		let mut kwargs = HashMap::new();
		kwargs.insert("id".to_string(), "123".to_string());

		let user_url = LazyUrl::with_kwargs("user-detail", kwargs, resolver.clone());
		assert_eq!(user_url.resolve(), "/users/123/");
	}

	#[rstest]
	fn test_lazy_url_caching() {
		let resolver = setup_resolver();
		let home_url = LazyUrl::new("home", resolver.clone());

		// First resolution
		let url1 = home_url.resolve();
		assert!(home_url.is_resolved());

		// Second resolution (should use cache)
		let url2 = home_url.resolve();
		assert_eq!(url1, url2);
	}

	#[rstest]
	fn test_lazy_url_display() {
		let resolver = setup_resolver();
		let home_url = LazyUrl::new("home", resolver.clone());
		assert_eq!(format!("{}", home_url), "/");
	}

	#[rstest]
	#[should_panic(expected = "Failed to resolve URL")]
	fn test_lazy_url_pattern_not_found() {
		let resolver = setup_resolver();
		let invalid_url = LazyUrl::new("nonexistent", resolver.clone());
		invalid_url.resolve();
	}

	#[rstest]
	#[should_panic(expected = "Failed to resolve URL")]
	fn test_lazy_url_missing_parameter() {
		let resolver = setup_resolver();
		let user_url = LazyUrl::new("user-detail", resolver.clone());
		user_url.resolve();
	}
}
