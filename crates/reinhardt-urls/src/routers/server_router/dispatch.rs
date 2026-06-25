//! Request dispatch internals for [`ServerRouter`].
//!
//! Implements hierarchical route resolution, prefix stripping, and the
//! method/path matchit lookup used by the `Handler` implementation.

use super::ServerRouter;
use super::types::RouteMatch;
use hyper::Method;
use reinhardt_di::InjectionContext;
use reinhardt_middleware::Middleware;
use std::borrow::Cow;
use std::sync::{Arc, PoisonError};

impl ServerRouter {
	/// Strip `prefix` from `path` and ensure the result always has a leading `/`.
	///
	/// When a prefix ends with `/` (e.g., `/api/`), `str::strip_prefix` consumes
	/// the trailing slash, leaving the remainder without a leading `/`. This breaks
	/// child router matching because child prefixes expect paths starting with `/`.
	///
	/// To avoid false-positive matches, when `prefix` does not end with `/` the
	/// match also requires a segment boundary: `path` must either equal `prefix`
	/// exactly or continue with `/`. This prevents `/api` from being treated as
	/// a prefix of `/api2/...`.
	///
	/// Returns `None` if `path` does not start with `prefix` (subject to the
	/// boundary rule above).
	pub(crate) fn strip_prefix_normalized<'a>(prefix: &str, path: &'a str) -> Option<Cow<'a, str>> {
		if prefix.is_empty() {
			return Some(Cow::Borrowed(path));
		}
		let stripped = path.strip_prefix(prefix)?;
		// Enforce segment boundary when the prefix does not already end with `/`.
		// Without this, `prefix = "/api"` would match `path = "/api2/foo"` and
		// strip into `2/foo`, mis-routing requests.
		if !prefix.ends_with('/') && !stripped.is_empty() && !stripped.starts_with('/') {
			return None;
		}
		Some(if stripped.is_empty() {
			Cow::Borrowed("/")
		} else if stripped.starts_with('/') {
			Cow::Borrowed(stripped)
		} else {
			Cow::Owned(format!("/{stripped}"))
		})
	}

	/// Resolve a request path to a route match
	///
	/// This performs hierarchical route resolution:
	/// 1. Check prefix match
	/// 2. Try child routers first (depth-first search)
	/// 3. Try own routes
	pub(crate) fn resolve(&self, path: &str, method: &Method) -> Option<RouteMatch> {
		// 1. Check prefix and normalize remaining path (ensures leading `/`)
		let remaining_path = Self::strip_prefix_normalized(&self.prefix, path)?;

		// 2. Try child routers first
		let own_middleware = self.build_middleware_with_exclusions();
		for child in &self.children {
			if let Some(route_match) =
				child.resolve_internal(&remaining_path, method, &own_middleware, &self.di_context)
			{
				return Some(route_match);
			}
		}

		// 3. Try own routes
		self.match_own_routes_with_context(
			&remaining_path,
			method,
			own_middleware,
			self.di_context.clone(),
		)
	}

	/// Internal route resolution with middleware and DI context inheritance
	pub(crate) fn resolve_internal(
		&self,
		path: &str,
		method: &Method,
		parent_middleware: &[Arc<dyn Middleware>],
		parent_di: &Option<Arc<InjectionContext>>,
	) -> Option<RouteMatch> {
		// Check prefix and normalize remaining path (ensures leading `/`)
		let remaining_path = Self::strip_prefix_normalized(&self.prefix, path)?;

		// Build middleware stack (parent → child order)
		let mut middleware_stack = parent_middleware.to_vec();
		middleware_stack.extend(self.build_middleware_with_exclusions());

		// Inherit DI context
		let di_context = self.di_context.clone().or_else(|| parent_di.clone());

		// Try child routers
		for child in &self.children {
			if let Some(route_match) =
				child.resolve_internal(&remaining_path, method, &middleware_stack, &di_context)
			{
				return Some(route_match);
			}
		}

		// Try own routes
		self.match_own_routes_with_context(&remaining_path, method, middleware_stack, di_context)
	}

	/// Match routes in this router (without context)
	#[cfg(test)]
	pub(crate) fn match_own_routes(&self, path: &str, method: &Method) -> Option<RouteMatch> {
		self.match_own_routes_with_context(
			path,
			method,
			self.build_middleware_with_exclusions(),
			self.di_context.clone(),
		)
	}

	/// Match routes in this router with provided context
	///
	/// This method uses matchit for O(m) route matching where m = path length.
	/// Routes must be compiled before matching (automatically done on first match).
	pub(crate) fn match_own_routes_with_context(
		&self,
		path: &str,
		method: &Method,
		middleware_stack: Vec<Arc<dyn Middleware>>,
		di_context: Option<Arc<InjectionContext>>,
	) -> Option<RouteMatch> {
		// Compile routes on first use (lazy compilation with interior mutability)
		self.compile_routes();

		// Normalize path for matchit lookup - routes are registered with leading slash.
		// Borrow the common already-normalized path to avoid per-request allocation.
		let search_path: Cow<'_, str> = if path.starts_with('/') {
			Cow::Borrowed(path)
		} else {
			Cow::Owned(format!("/{path}"))
		};

		// Use matchit to find matching route - O(m) complexity
		let router_lock = match *method {
			Method::GET => &self.get_router,
			Method::POST => &self.post_router,
			Method::PUT => &self.put_router,
			Method::DELETE => &self.delete_router,
			Method::PATCH => &self.patch_router,
			Method::HEAD => &self.head_router,
			Method::OPTIONS => &self.options_router,
			_ => &self.get_router,
		};

		let router = router_lock.read().unwrap_or_else(PoisonError::into_inner);

		let build_route_match = |try_path: &str| {
			if let Ok(matched) = router.at(try_path) {
				let route_handler = matched.value;

				// Extract parameters from matchit. matchit's `Params` iterator
				// yields parameters in URL pattern declaration order, so we
				// collect into a `Vec` to preserve that ordering all the way
				// down to the tuple extractor (see issue #4013).
				let params: Vec<(String, String)> = matched
					.params
					.iter()
					.map(|(k, v)| (k.to_string(), v.to_string()))
					.collect();

				// Combine router-level and route-level middleware
				let mut combined_middleware = middleware_stack.clone();
				combined_middleware.extend(route_handler.middleware.iter().cloned());

				return Some(RouteMatch {
					handler: route_handler.handler.clone(),
					params,
					middleware_stack: combined_middleware,
					di_context: di_context.clone(),
				});
			}

			None
		};

		// Try matching with the original path first. If that fails, try with
		// trailing slash toggled (Django-style APPEND_SLASH behavior).
		if let Some(route_match) = build_route_match(search_path.as_ref()) {
			return Some(route_match);
		}

		if search_path.as_ref().ends_with('/') {
			let without_slash = search_path.trim_end_matches('/');
			let fallback_path = if without_slash.is_empty() {
				"/"
			} else {
				without_slash
			};

			if fallback_path != search_path.as_ref() {
				return build_route_match(fallback_path);
			}
		} else {
			let fallback_path = format!("{}/", search_path);
			return build_route_match(&fallback_path);
		}

		None
	}

	/// Check if a path exists in any HTTP method's router
	///
	/// This is used to determine whether to return 404 (path not found)
	/// or 405 (method not allowed) when a route doesn't match.
	pub(crate) fn path_exists_for_any_method(&self, path: &str) -> bool {
		self.compile_routes();

		// Apply prefix stripping logic (same as resolve method, ensures leading `/`)
		let search_path = match Self::strip_prefix_normalized(&self.prefix, path) {
			Some(p) => p,
			None => return false,
		};

		let method_routers = [
			&self.get_router,
			&self.post_router,
			&self.put_router,
			&self.delete_router,
			&self.patch_router,
			&self.head_router,
			&self.options_router,
		];

		let path_exists = |candidate_path: &str| {
			for router_lock in method_routers {
				let router = router_lock.read().unwrap_or_else(PoisonError::into_inner);
				if router.at(candidate_path).is_ok() {
					return true;
				}
			}

			// Also check children routers with remaining path.
			for child in &self.children {
				if child.path_exists_for_any_method(candidate_path) {
					return true;
				}
			}

			false
		};

		// Try matching with the original path first. If that fails, try with
		// trailing slash toggled (Django-style APPEND_SLASH behavior).
		if path_exists(search_path.as_ref()) {
			return true;
		}

		if search_path.as_ref().ends_with('/') {
			let without_slash = search_path.trim_end_matches('/');
			let fallback_path = if without_slash.is_empty() {
				"/"
			} else {
				without_slash
			};

			if fallback_path != search_path.as_ref() {
				return path_exists(fallback_path);
			}
		} else {
			let fallback_path = format!("{}/", search_path);
			return path_exists(&fallback_path);
		}

		false
	}
}
