//! Runtime [`UrlReverser`] for name-to-URL lookup, plus the top-level
//! [`reverse`] convenience function.

use super::super::Route;
use super::super::pattern::PathPattern;
use super::runtime::ReverseResult;
use reinhardt_core::exception::Error;
use std::collections::HashMap;

/// URL reverser for resolving names back to URLs
/// Similar to Django's URLResolver reverse functionality
pub struct UrlReverser {
	/// Map of route names (including namespace) to routes
	routes: HashMap<String, Route>,
	/// Alias map: alias name → canonical name.
	/// Used for backward compatibility when route names change format.
	aliases: HashMap<String, String>,
}

impl UrlReverser {
	/// Create a new empty URL reverser.
	pub fn new() -> Self {
		Self {
			routes: HashMap::new(),
			aliases: HashMap::new(),
		}
	}

	/// Register a route for reverse lookup.
	///
	/// Returns `Err` with a descriptive message if a route with the same
	/// fully-qualified name has already been registered.
	pub fn register(&mut self, route: Route) -> std::result::Result<(), String> {
		if let Some(full_name) = route.full_name() {
			use std::collections::hash_map::Entry;
			match self.routes.entry(full_name.clone()) {
				Entry::Occupied(existing) => Err(format!(
					"Duplicate route name '{}': path '{}' conflicts with existing path '{}'",
					full_name,
					route.path,
					existing.get().path
				)),
				Entry::Vacant(entry) => {
					entry.insert(route);
					Ok(())
				}
			}
		} else {
			Ok(())
		}
	}

	/// Register a route by name and path (without handler)
	///
	/// This is used for hierarchical routers where we only need the name-to-path mapping
	/// for URL reversal, not the actual handler.
	///
	/// # Arguments
	///
	/// * `name` - The fully qualified route name (e.g., "v1:users:detail")
	/// * `path` - The URL path pattern (e.g., "/users/{id}/")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::UrlReverser;
	///
	/// let mut reverser = UrlReverser::new();
	/// reverser.register_path("v1:users:detail", "/api/v1/users/{id}/").unwrap();
	///
	/// let url = reverser.reverse_with("v1:users:detail", &[("id", "123")]).unwrap();
	/// assert_eq!(url, "/api/v1/users/123/");
	/// ```
	pub fn register_path(&mut self, name: &str, path: &str) -> std::result::Result<(), String> {
		// Create a dummy handler for the route
		// The handler is never used for URL reversal
		use reinhardt_http::Handler;
		use std::sync::Arc;

		#[derive(Clone)]
		struct DummyHandler;

		#[async_trait::async_trait]
		impl Handler for DummyHandler {
			async fn handle(
				&self,
				_req: reinhardt_http::Request,
			) -> reinhardt_core::exception::Result<reinhardt_http::Response> {
				unreachable!("DummyHandler should never be called")
			}
		}

		// Parse the name to extract namespace (if any)
		let parts: Vec<&str> = name.rsplitn(2, ':').collect();
		let (route_name, namespace) = if parts.len() == 2 {
			(parts[0].to_string(), Some(parts[1].to_string()))
		} else {
			(name.to_string(), None)
		};

		let route = Route::new(path, Arc::new(DummyHandler)).with_name(&route_name);

		let route = if let Some(ns) = namespace {
			route.with_namespace(&ns)
		} else {
			route
		};

		use std::collections::hash_map::Entry;
		match self.routes.entry(name.to_string()) {
			Entry::Occupied(existing) => Err(format!(
				"Duplicate route name '{}': path '{}' conflicts with existing path '{}'",
				name,
				path,
				existing.get().path
			)),
			Entry::Vacant(entry) => {
				entry.insert(route);
				Ok(())
			}
		}
	}

	/// Reverse a URL name to a path with parameters
	/// Similar to Django's reverse() function
	///
	/// # Arguments
	///
	/// * `name` - The route name, optionally with namespace (e.g., "users:detail")
	/// * `params` - Map of parameter names to values
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{UrlReverser, Route};
	/// use reinhardt_http::Handler;
	/// use std::sync::Arc;
	/// use std::collections::HashMap;
	///
	/// # use async_trait::async_trait;
	/// # use reinhardt_http::{Request, Response, Result};
	/// # struct DummyHandler;
	/// # #[async_trait]
	/// # impl Handler for DummyHandler {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	/// let handler = Arc::new(DummyHandler);
	/// let mut reverser = UrlReverser::new();
	/// let route = Route::new("/users/{id}/", handler)
	///     .with_name("detail")
	///     .with_namespace("users");
	/// reverser.register(route).unwrap();
	///
	/// let mut params = HashMap::new();
	/// params.insert("id".to_string(), "123".to_string());
	///
	/// let url = reverser.reverse("users:detail", &params).unwrap();
	/// assert_eq!(url, "/users/123/");
	/// ```
	pub fn reverse(&self, name: &str, params: &HashMap<String, String>) -> ReverseResult<String> {
		// Prefer canonical route name; fall back to alias resolution only
		// when the direct lookup misses. This prevents an alias entry from
		// shadowing a real route with the same key.
		let route = if let Some(r) = self.routes.get(name) {
			r
		} else {
			let resolved_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
			self.routes
				.get(resolved_name)
				.ok_or_else(|| Error::NotFound(name.to_string()))?
		};

		// Parse the path pattern and delegate substitution to PathPattern::reverse.
		// PathPattern normalizes typed placeholders (e.g., "{<path:filepath>}" -> "{filepath}")
		// and substitutes against the normalized form, so reversal works correctly even when
		// the raw route.path contains typed placeholder syntax.
		let pattern = PathPattern::new(&route.path)
			.map_err(|e| Error::Validation(format!("pattern: {}", e)))?;

		pattern.reverse(params).map_err(Error::Validation)
	}

	/// Reverse a URL name to a path with positional parameters
	/// Convenience method that takes a slice of key-value pairs
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{UrlReverser, Route};
	/// use reinhardt_http::Handler;
	/// use std::sync::Arc;
	///
	/// # use async_trait::async_trait;
	/// # use reinhardt_http::{Request, Response, Result};
	/// # struct DummyHandler;
	/// # #[async_trait]
	/// # impl Handler for DummyHandler {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	/// let handler = Arc::new(DummyHandler);
	/// let mut reverser = UrlReverser::new();
	/// let route = Route::new("/users/{id}/", handler)
	///     .with_name("detail");
	/// reverser.register(route).unwrap();
	///
	/// let url = reverser.reverse_with("detail", &[("id", "123")]).unwrap();
	/// assert_eq!(url, "/users/123/");
	/// ```
	pub fn reverse_with<S: AsRef<str>>(
		&self,
		name: &str,
		params: &[(S, S)],
	) -> ReverseResult<String> {
		let params_map: HashMap<String, String> = params
			.iter()
			.map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
			.collect();

		self.reverse(name, &params_map)
	}

	/// Check if a route name is registered
	pub fn has_route(&self, name: &str) -> bool {
		self.routes.contains_key(name)
	}

	/// Get all registered route names
	pub fn route_names(&self) -> Vec<String> {
		self.routes.keys().cloned().collect()
	}

	/// Register an alias for a route name.
	///
	/// `reverse(alias)` will resolve to the same URL as `reverse(canonical)`.
	/// If the alias already exists, it is overwritten (last-write-wins).
	///
	/// The canonical target is resolved lazily — if the canonical route does
	/// not exist at the time of `reverse()`, the lookup returns `NotFound`.
	pub fn add_name_alias(&mut self, alias: &str, canonical: &str) {
		self.aliases
			.insert(alias.to_string(), canonical.to_string());
	}
}

impl Default for UrlReverser {
	fn default() -> Self {
		Self::new()
	}
}

/// Standalone reverse function for convenience
/// Similar to Django's reverse() function
///
/// This requires routes to be registered with a global reverser.
/// For more control, use UrlReverser directly.
pub fn reverse(
	name: &str,
	params: &HashMap<String, String>,
	reverser: &UrlReverser,
) -> ReverseResult<String> {
	reverser.reverse(name, params)
}
