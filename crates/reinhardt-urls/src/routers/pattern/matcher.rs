use super::path_pattern::PathPattern;
use super::radix::{RadixRouter, RadixRouterError};
use super::validation::validate_path_param;
use reinhardt_http::PathParams;

/// Matching mode for PathMatcher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchingMode {
	/// Linear O(n) matching through all patterns (default)
	Linear,
	/// Radix Tree O(m) matching using matchit (recommended for >100 routes)
	RadixTree,
}

/// Path matcher - uses composition to match paths
///
/// Supports two matching modes:
/// - **Linear** (default): O(n) search through patterns, suitable for <100 routes
/// - **RadixTree**: O(m) matching using radix tree, recommended for >100 routes
pub struct PathMatcher {
	patterns: Vec<(PathPattern, String)>, // (pattern, handler_id)
	radix_router: Option<RadixRouter>,
	mode: MatchingMode,
}

impl PathMatcher {
	/// Create a new PathMatcher with linear matching (default)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::PathMatcher;
	///
	/// let matcher = PathMatcher::new();
	/// assert_eq!(matcher.match_path("/users/"), None);
	/// ```
	pub fn new() -> Self {
		Self {
			patterns: Vec::new(),
			radix_router: None,
			mode: MatchingMode::Linear,
		}
	}

	/// Create a new PathMatcher with specified matching mode
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathMatcher, MatchingMode};
	///
	/// let matcher = PathMatcher::with_mode(MatchingMode::RadixTree);
	/// ```
	pub fn with_mode(mode: MatchingMode) -> Self {
		Self {
			patterns: Vec::new(),
			radix_router: if mode == MatchingMode::RadixTree {
				Some(RadixRouter::new())
			} else {
				None
			},
			mode,
		}
	}

	/// Enable radix tree matching mode
	///
	/// Rebuilds the radix router from existing patterns.
	/// Recommended when route count exceeds 100.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathMatcher, PathPattern, path};
	///
	/// let mut matcher = PathMatcher::new();
	/// let pattern = PathPattern::new(path!("/users/")).unwrap();
	/// matcher.add_pattern(pattern, "users_list".to_string()).unwrap();
	///
	/// // Enable radix tree mode
	/// matcher.enable_radix_tree().unwrap();
	///
	/// let result = matcher.match_path("/users/");
	/// assert!(result.is_some());
	/// ```
	///
	/// # Errors
	///
	/// Returns `RadixRouterError` if any of the existing patterns conflicts with
	/// or is rejected by the underlying radix router. When this happens the
	/// matcher is left in `Linear` mode (the original state) so callers can
	/// recover or surface the error.
	pub fn enable_radix_tree(&mut self) -> Result<(), RadixRouterError> {
		if self.mode == MatchingMode::RadixTree {
			return Ok(()); // Already enabled
		}

		let mut radix_router = RadixRouter::new();

		// Rebuild radix router from existing patterns. Propagate failures so
		// Linear and RadixTree modes cannot silently diverge.
		for (pattern, handler_id) in &self.patterns {
			radix_router.add_route(&pattern.to_matchit_pattern(), handler_id.clone())?;
		}

		self.mode = MatchingMode::RadixTree;
		self.radix_router = Some(radix_router);
		Ok(())
	}

	/// Get current matching mode
	pub fn mode(&self) -> MatchingMode {
		self.mode
	}
	/// Add a pattern to the matcher
	///
	/// If radix tree mode is enabled, also adds to the radix router.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathMatcher, PathPattern, path};
	///
	/// let mut matcher = PathMatcher::new();
	/// let pattern = PathPattern::new(path!("/users/")).unwrap();
	/// matcher.add_pattern(pattern, "users_list".to_string()).unwrap();
	///
	/// let result = matcher.match_path("/users/");
	/// assert!(result.is_some());
	/// ```
	///
	/// # Errors
	///
	/// Returns `RadixRouterError` when radix tree mode is active and the
	/// underlying `RadixRouter::add_route` rejects the pattern (e.g., conflict
	/// or invalid syntax). When this happens the pattern is **not** appended
	/// to the linear list either, so `Linear` and `RadixTree` modes remain in
	/// sync.
	pub fn add_pattern(
		&mut self,
		pattern: PathPattern,
		handler_id: String,
	) -> Result<(), RadixRouterError> {
		let matchit_pattern = pattern.to_matchit_pattern();

		// If radix tree mode is enabled, insert into the radix router first
		// so a failure does not leave the linear list and the radix tree out
		// of sync.
		if let Some(ref mut radix_router) = self.radix_router {
			radix_router.add_route(&matchit_pattern, handler_id.clone())?;
		}

		self.patterns.push((pattern, handler_id));
		Ok(())
	}
	/// Match a path and extract parameters
	///
	/// Uses the configured matching mode:
	/// - **Linear**: O(n) search through patterns (default)
	/// - **RadixTree**: O(m) radix tree matching where m = path length
	///
	/// # Performance Notes
	///
	/// - **Linear mode**: O(n×m) where n = route count, m = path length
	///   - Suitable for <100 routes
	///   - Benefits from RouteCache for O(1) on cache hits
	/// - **RadixTree mode**: O(m) where m = path length
	///   - Recommended for >100 routes
	///   - 3-5x faster for large route sets
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathMatcher, PathPattern, path};
	///
	/// let mut matcher = PathMatcher::new();
	/// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
	/// matcher.add_pattern(pattern, "users_detail".to_string());
	///
	/// let result = matcher.match_path("/users/123/");
	/// assert!(result.is_some());
	/// let (handler_id, params) = result.unwrap();
	/// assert_eq!(handler_id, "users_detail");
	/// assert_eq!(params.get("id"), Some(&"123".to_string()));
	/// ```
	pub fn match_path(&self, path: &str) -> Option<(String, PathParams)> {
		match self.mode {
			MatchingMode::RadixTree => {
				// Use radix tree for O(m) matching
				if let Some(ref radix_router) = self.radix_router {
					let (handler_id, params) = radix_router.match_path(path)?;

					// Validate path-type parameters against directory traversal
					if let Some((pattern, _)) =
						self.patterns.iter().find(|(_, id)| *id == handler_id)
					{
						for (name, value) in params.iter() {
							if pattern.path_type_params.contains(name)
								&& !validate_path_param(value)
							{
								return None;
							}
						}
					}

					Some((handler_id, params))
				} else {
					// Fallback to linear if radix router not initialized
					self.match_path_linear(path)
				}
			}
			MatchingMode::Linear => {
				// Use linear O(n) matching
				self.match_path_linear(path)
			}
		}
	}

	/// Linear pattern matching (O(n))
	fn match_path_linear(&self, path: &str) -> Option<(String, PathParams)> {
		'outer: for (pattern, handler_id) in &self.patterns {
			if let Some(captures) = pattern.regex.captures(path) {
				// Use ordered `PathParams` so tuple extractors see params in
				// URL pattern declaration order (issue #4013). `param_names()`
				// already yields names in the order they appear in the pattern.
				let mut params = PathParams::new();

				for name in pattern.param_names() {
					if let Some(value) = captures.name(name) {
						let val = value.as_str();
						// Validate path-type parameters against directory traversal
						if pattern.path_type_params.contains(name) && !validate_path_param(val) {
							continue 'outer;
						}
						params.insert(name.clone(), val.to_string());
					}
				}

				return Some((handler_id.clone(), params));
			}
		}

		None
	}
}

impl Default for PathMatcher {
	fn default() -> Self {
		Self::new()
	}
}
