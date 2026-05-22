use matchit::Router as MatchitRouter;
use reinhardt_http::PathParams;

/// Error type for Radix Router operations
#[derive(Debug, thiserror::Error)]
pub enum RadixRouterError {
	/// The route pattern is syntactically invalid.
	#[error("Invalid pattern: {0}")]
	InvalidPattern(String),
	/// The route could not be inserted into the radix tree (e.g., conflict).
	#[error("Route insertion failed: {0}")]
	InsertionFailed(String),
}

/// Radix Tree-based router using matchit for O(m) matching
///
/// Provides significant performance improvements over linear O(n) matching,
/// especially for applications with large route counts (>100 routes).
///
/// # Performance Characteristics
///
/// - **Insertion**: O(m) where m is the pattern length
/// - **Matching**: O(m) where m is the path length (vs O(n×m) for linear)
/// - **Memory**: O(total pattern characters) for the radix tree structure
///
/// # Pattern Syntax
///
/// Uses matchit's pattern syntax, compatible with Django-style patterns:
/// - `/users/{id}` - Single parameter
/// - `/posts/{post_id}/comments/{comment_id}` - Multiple parameters
/// - `/files/{*path}` - Catch-all wildcard (matches remaining path including `/`)
///
/// # Examples
///
/// ```
/// use reinhardt_urls::routers::{RadixRouter, path};
///
/// let mut router = RadixRouter::new();
///
/// // Register routes
/// router.add_route(path!("/users/"), "users_list".to_string()).unwrap();
/// router.add_route(path!("/users/{id}/"), "users_detail".to_string()).unwrap();
///
/// // Match paths
/// let result = router.match_path("/users/123/");
/// assert!(result.is_some());
/// let (handler_id, params) = result.unwrap();
/// assert_eq!(handler_id, "users_detail");
/// assert_eq!(params.get("id"), Some(&"123".to_string()));
/// ```
pub struct RadixRouter {
	router: MatchitRouter<String>,
}

impl RadixRouter {
	/// Create a new RadixRouter
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RadixRouter;
	///
	/// let router = RadixRouter::new();
	/// ```
	pub fn new() -> Self {
		Self {
			router: MatchitRouter::new(),
		}
	}

	/// Add a route pattern to the router
	///
	/// Converts Django-style `{param}` patterns to matchit's `{param}` syntax.
	/// Both syntaxes are compatible, so no conversion is needed.
	///
	/// # Arguments
	///
	/// * `pattern` - URL pattern (e.g., `/users/{id}/`)
	/// * `handler_id` - Identifier for the route handler
	///
	/// # Errors
	///
	/// Returns `RadixRouterError::InsertionFailed` if:
	/// - Pattern conflicts with existing routes
	/// - Pattern syntax is invalid
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{RadixRouter, path};
	///
	/// let mut router = RadixRouter::new();
	/// router.add_route(path!("/users/{id}/"), "users_detail".to_string()).unwrap();
	///
	/// // Catch-all wildcard
	/// router.add_route(path!("/files/{file_path}"), "serve_file".to_string()).unwrap();
	/// ```
	pub fn add_route(&mut self, pattern: &str, handler_id: String) -> Result<(), RadixRouterError> {
		self.router
			.insert(pattern, handler_id)
			.map_err(|e| RadixRouterError::InsertionFailed(e.to_string()))
	}

	/// Match a path and return handler ID with extracted parameters
	///
	/// Performs O(m) matching where m is the path length.
	///
	/// # Arguments
	///
	/// * `path` - Request path to match
	///
	/// # Returns
	///
	/// `Some((handler_id, params))` if matched, `None` otherwise.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{RadixRouter, path};
	///
	/// let mut router = RadixRouter::new();
	/// router.add_route(path!("/users/{id}/posts/{post_id}/"), "post_detail".to_string()).unwrap();
	///
	/// let result = router.match_path("/users/123/posts/456/");
	/// assert!(result.is_some());
	///
	/// let (handler_id, params) = result.unwrap();
	/// assert_eq!(handler_id, "post_detail");
	/// assert_eq!(params.get("id"), Some(&"123".to_string()));
	/// assert_eq!(params.get("post_id"), Some(&"456".to_string()));
	/// ```
	pub fn match_path(&self, path: &str) -> Option<(String, PathParams)> {
		match self.router.at(path) {
			Ok(matched) => {
				let handler_id = matched.value.clone();
				// matchit's `Params` iterator yields parameters in URL pattern
				// declaration order; collect into `PathParams` to preserve it
				// (issue #4013).
				let params: PathParams = matched
					.params
					.iter()
					.map(|(k, v)| (k.to_string(), v.to_string()))
					.collect();

				Some((handler_id, params))
			}
			Err(_) => None,
		}
	}
}

impl Default for RadixRouter {
	fn default() -> Self {
		Self::new()
	}
}
