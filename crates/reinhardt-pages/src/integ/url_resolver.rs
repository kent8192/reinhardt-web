//! Runtime support for url! macro.
//!
//! This module provides runtime URL resolution for routes using a URL mapping.
//! The mapping is typically configured during application startup and contains
//! mappings from route names to URL patterns.
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::integ::url_resolver;
//! use std::collections::HashMap;
//!
//! // Initialize with URL mapping (typically done in main.rs)
//! let mut routes = HashMap::new();
//! routes.insert("home".to_string(), "/".to_string());
//! routes.insert("user-profile".to_string(), "/users/{id}".to_string());
//! url_resolver::init_url_resolver(routes);
//!
//! // Resolve URLs (automatically called by url! macro)
//! let url = url_resolver::resolve_url("home");
//! assert_eq!(url, "/");
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global URL route mapping.
///
/// This mapping maps route names to URL patterns.
/// It should be initialized once at application startup using [`init_url_resolver`].
static URL_ROUTES: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Initializes the URL resolver with a route mapping.
///
/// This function should be called once at application startup, typically in main.rs.
/// Subsequent calls will panic.
///
/// # Panics
///
/// Panics if the URL resolver has already been initialized.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_pages::integ::url_resolver;
/// use std::collections::HashMap;
///
/// let mut routes = HashMap::new();
/// routes.insert("home".to_string(), "/".to_string());
/// routes.insert("user-profile".to_string(), "/users/{id}".to_string());
/// url_resolver::init_url_resolver(routes);
/// ```
pub fn init_url_resolver(routes: HashMap<String, String>) {
	URL_ROUTES
		.set(routes)
		.expect("URL resolver already initialized");
}

/// Resolves a route name to its URL pattern.
///
/// This function looks up the given route name in the URL mapping and returns
/// the corresponding URL pattern. If the route name is not found, it panics
/// (as this indicates a programming error - the route should exist).
///
/// # Panics
///
/// - Panics if the URL resolver has not been initialized with [`init_url_resolver`].
/// - Panics if the route name is not found in the mapping.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_pages::integ::url_resolver;
///
/// let url = url_resolver::resolve_url("home");
/// assert_eq!(url, "/");
///
/// let url = url_resolver::resolve_url("user-profile");
/// assert_eq!(url, "/users/{id}");
/// ```
///
/// # Note
///
/// Currently, this function returns the raw URL pattern without parameter substitution.
/// Future versions will support parameter substitution (e.g., `url!("user-profile", id = 123)`).
pub fn resolve_url(route_name: &str) -> String {
	let routes = URL_ROUTES
		.get()
		.expect("URL resolver not initialized. Call init_url_resolver() first.");

	// Look up route name
	routes.get(route_name).cloned().unwrap_or_else(|| {
		panic!(
			"Route '{}' not found. Available routes: {:?}",
			route_name,
			routes.keys().collect::<Vec<_>>()
		)
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serial_test::serial;

	#[rstest]
	#[serial(url_resolver)]
	fn test_resolve_basic_routes() {
		let mut routes = HashMap::new();
		routes.insert("home".to_string(), "/".to_string());
		routes.insert("about".to_string(), "/about".to_string());
		routes.insert("user-profile".to_string(), "/users/{id}".to_string());

		init_url_resolver(routes);

		assert_eq!(resolve_url("home"), "/");
		assert_eq!(resolve_url("about"), "/about");
		assert_eq!(resolve_url("user-profile"), "/users/{id}");
	}

	#[rstest]
	#[serial(url_resolver)]
	#[should_panic(expected = "Route 'nonexistent' not found")]
	fn test_resolve_nonexistent_route() {
		// Initialize with empty routes if not already initialized (OnceLock can only be set once)
		let _ = URL_ROUTES.set(HashMap::new());

		resolve_url("nonexistent");
	}

	#[rstest]
	#[should_panic(expected = "URL resolver not initialized")]
	fn test_resolve_before_init() {
		// Note: This test will fail if run after other tests that initialize the resolver
		// In practice, use a separate test binary or reset mechanism
		resolve_url("test");
	}
}
