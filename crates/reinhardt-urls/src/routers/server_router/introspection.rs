//! Introspection and URL reversal helpers for [`ServerRouter`].
//!
//! Exposes router metadata (prefix, namespace, middleware registry),
//! recursive route enumeration, and namespace-aware URL reversal.

use super::ServerRouter;
use super::types::{MiddlewareInfo, RouteInfo, join_path};
use hyper::Method;

impl ServerRouter {
	/// Get the prefix of this router
	pub fn prefix(&self) -> &str {
		&self.prefix
	}

	/// Get the namespace of this router
	pub fn namespace(&self) -> Option<&str> {
		self.namespace.as_deref()
	}

	/// Get the number of child routers
	pub fn children_count(&self) -> usize {
		self.children.len()
	}

	/// Get all registered middleware information
	///
	/// Returns a deduplicated list of middleware registered on this router
	/// and all child routers. The order reflects registration order.
	pub fn get_registered_middleware(&self) -> Vec<MiddlewareInfo> {
		let mut all = self.middleware_names.clone();

		// Collect from children recursively
		for child in &self.children {
			all.extend(child.get_registered_middleware());
		}

		// Deduplicate while preserving order
		let mut seen = std::collections::HashSet::new();
		all.retain(|info| seen.insert(info.type_name.clone()));
		all
	}

	/// Get all routes from this router and its children
	///
	/// Returns a vector of tuples containing (full_path, name, namespace, methods).
	/// This recursively collects routes from all child routers.
	///
	/// # Examples
	///
	/// ```ignore
	/// let router = ServerRouter::new()
	///     .with_prefix("/api/v1")
	///     .function("/users", Method::GET, handler);
	///
	/// let routes = router.get_all_routes();
	/// // Returns: [("/api/v1/users", None, None, vec![Method::GET])]
	/// ```
	pub fn get_all_routes(&self) -> RouteInfo {
		let mut routes = Vec::new();

		// Collect routes from this router
		for route in &self.routes {
			let full_path = if self.prefix.is_empty() {
				route.path.clone()
			} else {
				join_path(&self.prefix, &route.path)
			};

			routes.push((
				full_path,
				route.name.clone(),
				route.namespace.clone().or_else(|| self.namespace.clone()),
				vec![], // Method-agnostic handlers accept all HTTP methods (shown as "ALL" in showurls)
			));
		}

		// Collect function-based routes
		for func_route in &self.functions {
			let full_path = if self.prefix.is_empty() {
				func_route.path.clone()
			} else {
				join_path(&self.prefix, &func_route.path)
			};

			routes.push((
				full_path,
				None,                   // Function routes don't have names
				self.namespace.clone(), // Use router's namespace
				vec![func_route.method.clone()],
			));
		}

		// Collect view routes
		for view_route in &self.views {
			let full_path = if self.prefix.is_empty() {
				view_route.path.clone()
			} else {
				join_path(&self.prefix, &view_route.path)
			};

			routes.push((
				full_path,
				None,                   // View routes don't have names
				self.namespace.clone(), // Use router's namespace
				vec![], // Class-based views handle method dispatch internally (accepts all methods)
			));
		}

		// Collect ViewSet routes
		for prefix in self.viewsets.keys() {
			let base_path = if self.prefix.is_empty() {
				format!("/{}", prefix)
			} else {
				format!("{}/{}", self.prefix, prefix)
			};

			// ViewSets generate standard CRUD routes
			let viewset_routes = vec![
				(format!("{}/", base_path), vec![Method::GET, Method::POST]),
				(
					format!("{}/<id>/", base_path),
					vec![Method::GET, Method::PUT, Method::DELETE],
				),
			];

			for (path, methods) in viewset_routes {
				routes.push((
					path,
					None,                   // ViewSet routes don't have individual names
					self.namespace.clone(), // Use router's namespace
					methods,
				));
			}
		}

		// Recursively collect from child routers.
		//
		// `child.get_all_routes()` already prepends `child.prefix` to each route
		// it owns, but it has no knowledge of *this* router's prefix. We must
		// therefore prepend `self.prefix` here so nested routers report fully
		// qualified paths (e.g., parent `/api` + child `/users/` + route `/foo`
		// yields `/api/users/foo`, not `/users/foo`).
		for child in &self.children {
			for (path, name, namespace, methods) in child.get_all_routes() {
				let full_path = if self.prefix.is_empty() {
					path
				} else {
					join_path(&self.prefix, &path)
				};

				// Combine namespaces (parent:child)
				let combined_namespace = match (self.namespace.as_ref(), namespace.as_ref()) {
					(Some(parent), Some(child)) => Some(format!("{}:{}", parent, child)),
					(Some(parent), None) => Some(parent.clone()),
					(None, Some(child)) => Some(child.clone()),
					(None, None) => None,
				};

				routes.push((full_path, name, combined_namespace, methods));
			}
		}

		routes
	}

	/// Get the fully qualified namespace for this router
	///
	/// Returns the complete namespace chain from root to this router.
	/// For example, if this router has namespace "users" and its parent has "v1",
	/// this returns "v1:users".
	///
	/// # Arguments
	///
	/// * `parent_namespace` - The parent router's namespace (if any)
	///
	/// # Examples
	///
	/// ```ignore
	/// let router = ServerRouter::new().with_namespace("users");
	/// assert_eq!(router.get_full_namespace(Some("v1")), Some("v1:users".to_string()));
	/// assert_eq!(router.get_full_namespace(None), Some("users".to_string()));
	/// ```
	pub fn get_full_namespace(&self, parent_namespace: Option<&str>) -> Option<String> {
		match (parent_namespace, self.namespace.as_deref()) {
			(Some(parent), Some(child)) => Some(format!("{}:{}", parent, child)),
			(Some(parent), None) => Some(parent.to_string()),
			(None, Some(child)) => Some(child.to_string()),
			(None, None) => None,
		}
	}

	/// Register all routes with the URL reverser
	///
	/// This recursively registers all routes from this router and its children
	/// with their fully qualified names (namespace:name format).
	///
	/// # Examples
	///
	/// ```ignore
	/// let mut router = ServerRouter::new()
	///     .with_namespace("v1");
	///
	/// // After registering routes, you can reverse them:
	/// router.register_all_routes();
	/// let url = router.reverse("v1:users:detail", &[("id", "123")]);
	/// ```
	#[must_use]
	pub fn register_all_routes(&mut self) -> Vec<String> {
		let registrations = self.collect_routes_recursive(None, "");
		let mut errors = Vec::new();
		for (name, path) in registrations {
			if let Err(e) = self.reverser.register_path(&name, &path) {
				errors.push(e);
			}
		}
		errors
	}

	/// Register an alias for a route name in this router's reverser.
	///
	/// See `UrlReverser::add_name_alias` for details.
	pub fn add_name_alias(&mut self, alias: &str, canonical: &str) {
		self.reverser.add_name_alias(alias, canonical);
	}

	/// Recursively collect all routes with accumulated prefixes and namespaces.
	///
	/// Returns a list of `(qualified_name, full_path)` pairs to be registered
	/// in the root router's reverser.
	pub(crate) fn collect_routes_recursive(
		&self,
		parent_namespace: Option<&str>,
		parent_prefix: &str,
	) -> Vec<(String, String)> {
		let full_namespace = self.get_full_namespace(parent_namespace);
		let current_prefix =
			crate::routers::path_utils::join_prefix_path(parent_prefix, &self.prefix);
		let mut registrations = Vec::new();

		// Collect routes from this router
		for route in &self.routes {
			if let Some(name) = &route.name {
				let qualified_name = if let Some(ref ns) = full_namespace {
					format!("{}:{}", ns, name)
				} else {
					name.clone()
				};

				let full_path =
					crate::routers::path_utils::join_prefix_path(&current_prefix, &route.path);
				registrations.push((qualified_name, full_path));
			}
		}

		// Collect function routes
		for func_route in &self.functions {
			if let Some(ref name) = func_route.name {
				let qualified_name = if let Some(ref ns) = full_namespace {
					format!("{}:{}", ns, name)
				} else {
					name.clone()
				};

				let full_path =
					crate::routers::path_utils::join_prefix_path(&current_prefix, &func_route.path);
				registrations.push((qualified_name, full_path));
			}
		}

		// Collect view routes
		for view_route in &self.views {
			if let Some(ref name) = view_route.name {
				let qualified_name = if let Some(ref ns) = full_namespace {
					format!("{}:{}", ns, name)
				} else {
					name.clone()
				};

				let full_path =
					crate::routers::path_utils::join_prefix_path(&current_prefix, &view_route.path);
				registrations.push((qualified_name, full_path));
			}
		}

		// Collect ViewSet routes with standard names (Django convention: basename, not prefix)
		for (prefix, viewset) in &self.viewsets {
			let base_path = if current_prefix.is_empty() {
				format!("/{}", prefix)
			} else {
				format!("{}/{}", current_prefix, prefix)
			};

			let basename = viewset.get_basename();
			let lookup_field = viewset.get_lookup_field();
			let viewset_routes = [
				(format!("{}-list", basename), format!("{}/", base_path)),
				(
					format!("{}-detail", basename),
					format!("{}/<{}>/", base_path, lookup_field),
				),
			];

			for (name, path) in viewset_routes {
				let qualified_name = if let Some(ref ns) = full_namespace {
					format!("{}:{}", ns, name)
				} else {
					name
				};

				registrations.push((qualified_name, path));
			}
		}

		// Recursively collect child routes
		for child in &self.children {
			registrations
				.extend(child.collect_routes_recursive(full_namespace.as_deref(), &current_prefix));
		}

		registrations
	}

	/// Reverse a URL by route name
	///
	/// Supports hierarchical namespace notation (e.g., "v1:users:detail").
	///
	/// # Arguments
	///
	/// * `name` - The route name, optionally with namespace (e.g., "users-detail" or "v1:users-detail")
	/// * `params` - URL parameters as key-value pairs
	///
	/// # Examples
	///
	/// ```ignore
	/// let router = ServerRouter::new()
	///     .with_namespace("v1");
	///
	/// // Reverse with namespace
	/// let url = router.reverse("v1:users:detail", &[("id", "123")]).unwrap();
	/// assert_eq!(url, "/users/123/");
	///
	/// // Reverse without namespace (searches all routes)
	/// let url = router.reverse("users-detail", &[("id", "123")]).unwrap();
	/// ```
	pub fn reverse(&self, name: &str, params: &[(&str, &str)]) -> Option<String> {
		// Try own reverser first
		if let Ok(url) = self.reverser.reverse_with(name, params) {
			return Some(url);
		}

		// Try child routers
		for child in &self.children {
			if let Some(url) = child.reverse(name, params) {
				return Some(url);
			}
		}

		None
	}
}
