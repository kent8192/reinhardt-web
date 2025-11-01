//! Namespace system for hierarchical URL routing
//!
//! This module provides a namespace-based URL reversal system inspired by Django's
//! URL namespace feature. It allows you to organize routes hierarchically and reference
//! them using colon-separated names (e.g., `"api:v1:users:detail"`).
//!
//! # Examples
//!
//! ```
//! use reinhardt_routers::namespace::{Namespace, NamespaceResolver};
//!
//! let mut resolver = NamespaceResolver::new();
//!
//! // Register routes with hierarchical namespaces
//! resolver.register("api:v1:users:list", "/api/v1/users/");
//! resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
//! resolver.register("api:v2:users:list", "/api/v2/users/");
//!
//! // Resolve URLs by namespace
//! let url = resolver.resolve("api:v1:users:detail", &[("id", "123")]).unwrap();
//! assert_eq!(url, "/api/v1/users/123/");
//!
//! // Query namespaces
//! let routes = resolver.list_routes_in_namespace("api:v1");
//! assert_eq!(routes.len(), 2);
//! ```

use crate::reverse::{ReverseError, ReverseResult};
use std::collections::HashMap;

/// Represents a hierarchical namespace for route organization
///
/// Namespaces are colon-separated strings that form a tree structure.
/// For example, `"api:v1:users"` represents:
/// - Root: "api"
/// - Child: "v1"
/// - Grandchild: "users"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Namespace {
	/// Colon-separated namespace string (e.g., "api:v1:users")
	full_path: String,

	/// Individual components of the namespace
	components: Vec<String>,
}

impl Namespace {
	/// Create a new namespace from a colon-separated string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let ns = Namespace::new("api:v1:users");
	/// assert_eq!(ns.full_path(), "api:v1:users");
	/// assert_eq!(ns.components(), &["api", "v1", "users"]);
	/// ```
	pub fn new(path: impl Into<String>) -> Self {
		let full_path = path.into();
		let components = full_path
			.split(':')
			.filter(|s| !s.is_empty())
			.map(|s| s.to_string())
			.collect();

		Self {
			full_path,
			components,
		}
	}

	/// Get the full namespace path
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let ns = Namespace::new("api:v1:users");
	/// assert_eq!(ns.full_path(), "api:v1:users");
	/// ```
	pub fn full_path(&self) -> &str {
		&self.full_path
	}

	/// Get the individual components of the namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let ns = Namespace::new("api:v1:users");
	/// assert_eq!(ns.components(), &["api", "v1", "users"]);
	/// ```
	pub fn components(&self) -> &[String] {
		&self.components
	}

	/// Get the root component of the namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let ns = Namespace::new("api:v1:users");
	/// assert_eq!(ns.root(), Some("api"));
	///
	/// let empty = Namespace::new("");
	/// assert_eq!(empty.root(), None);
	/// ```
	pub fn root(&self) -> Option<&str> {
		self.components.first().map(|s| s.as_str())
	}

	/// Get the parent namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let ns = Namespace::new("api:v1:users");
	/// let parent = ns.parent().unwrap();
	/// assert_eq!(parent.full_path(), "api:v1");
	///
	/// let root = Namespace::new("api");
	/// assert!(root.parent().is_none());
	/// ```
	pub fn parent(&self) -> Option<Namespace> {
		if self.components.len() <= 1 {
			return None;
		}

		let parent_components = &self.components[..self.components.len() - 1];
		Some(Namespace::new(parent_components.join(":")))
	}

	/// Get the leaf component (last component)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let ns = Namespace::new("api:v1:users");
	/// assert_eq!(ns.leaf(), Some("users"));
	/// ```
	pub fn leaf(&self) -> Option<&str> {
		self.components.last().map(|s| s.as_str())
	}

	/// Check if this namespace is a parent of another namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let parent = Namespace::new("api:v1");
	/// let child = Namespace::new("api:v1:users");
	/// let other = Namespace::new("api:v2");
	///
	/// assert!(parent.is_parent_of(&child));
	/// assert!(!parent.is_parent_of(&other));
	/// assert!(!child.is_parent_of(&parent));
	/// ```
	pub fn is_parent_of(&self, other: &Namespace) -> bool {
		if self.components.len() >= other.components.len() {
			return false;
		}

		self.components
			.iter()
			.zip(other.components.iter())
			.all(|(a, b)| a == b)
	}

	/// Check if this namespace is a child of another namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let parent = Namespace::new("api:v1");
	/// let child = Namespace::new("api:v1:users");
	///
	/// assert!(child.is_child_of(&parent));
	/// assert!(!parent.is_child_of(&child));
	/// ```
	pub fn is_child_of(&self, other: &Namespace) -> bool {
		other.is_parent_of(self)
	}

	/// Append a component to create a new namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// let parent = Namespace::new("api:v1");
	/// let child = parent.append("users");
	/// assert_eq!(child.full_path(), "api:v1:users");
	/// ```
	pub fn append(&self, component: impl Into<String>) -> Namespace {
		let component = component.into();
		let new_path = if self.full_path.is_empty() {
			component
		} else {
			format!("{}:{}", self.full_path, component)
		};
		Namespace::new(new_path)
	}

	/// Get the depth of this namespace (number of components)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::Namespace;
	///
	/// assert_eq!(Namespace::new("api").depth(), 1);
	/// assert_eq!(Namespace::new("api:v1").depth(), 2);
	/// assert_eq!(Namespace::new("api:v1:users").depth(), 3);
	/// ```
	pub fn depth(&self) -> usize {
		self.components.len()
	}
}

impl std::fmt::Display for Namespace {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.full_path)
	}
}

impl From<&str> for Namespace {
	fn from(s: &str) -> Self {
		Namespace::new(s)
	}
}

impl From<String> for Namespace {
	fn from(s: String) -> Self {
		Namespace::new(s)
	}
}

/// Route information with namespace metadata
#[derive(Debug, Clone)]
pub struct NamespacedRoute {
	/// Full route name including namespace (e.g., "api:v1:users:detail")
	pub full_name: String,

	/// Namespace component (e.g., "api:v1:users")
	pub namespace: Namespace,

	/// Route name component (e.g., "detail")
	pub route_name: String,

	/// URL pattern (e.g., "/api/v1/users/{id}/")
	pub pattern: String,

	/// Parameter names in the pattern
	pub param_names: Vec<String>,
}

impl NamespacedRoute {
	/// Create a new namespaced route
	///
	/// # Arguments
	///
	/// * `full_name` - Full route name including namespace (e.g., "api:v1:users:detail")
	/// * `pattern` - URL pattern (e.g., "/api/v1/users/{id}/")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespacedRoute;
	///
	/// let route = NamespacedRoute::new("api:v1:users:detail", "/api/v1/users/{id}/");
	/// assert_eq!(route.namespace.full_path(), "api:v1");
	/// assert_eq!(route.route_name, "users:detail");
	/// assert_eq!(route.pattern, "/api/v1/users/{id}/");
	/// ```
	pub fn new(full_name: impl Into<String>, pattern: impl Into<String>) -> Self {
		let full_name = full_name.into();
		let pattern = pattern.into();

		// Extract namespace and route name
		// Django-style: hierarchical namespaces
		// e.g., "api:v1:users:list" -> namespace = "api:v1", route_name = "users:list"
		let parts: Vec<&str> = full_name.split(':').collect();
		let (route_name, namespace) = if parts.len() >= 3 {
			// At least 3 parts: first N-2 are namespace, last 2 are route_name
			let namespace_end = parts.len() - 2;
			let ns = parts[..namespace_end].join(":");
			let rn = parts[namespace_end..].join(":");
			(rn, Namespace::new(ns))
		} else if parts.len() == 2 {
			// Exactly 2 parts: first is namespace, second is route_name
			(parts[1].to_string(), Namespace::new(parts[0]))
		} else {
			// Single part: no namespace, just route_name
			(full_name.clone(), Namespace::new(""))
		};

		// Extract parameter names from pattern
		let param_names = extract_param_names(&pattern);

		Self {
			full_name,
			namespace,
			route_name,
			pattern,
			param_names,
		}
	}

	/// Resolve this route with parameters
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespacedRoute;
	///
	/// let route = NamespacedRoute::new("api:v1:users:detail", "/api/v1/users/{id}/");
	/// let url = route.resolve(&[("id", "123")]).unwrap();
	/// assert_eq!(url, "/api/v1/users/123/");
	/// ```
	pub fn resolve(&self, params: &[(&str, &str)]) -> ReverseResult<String> {
		let param_map: HashMap<&str, &str> = params.iter().copied().collect();
		let mut result = self.pattern.clone();

		for param_name in &self.param_names {
			let value = param_map.get(param_name.as_str()).ok_or_else(|| {
				ReverseError::Validation(format!("missing parameter: {}", param_name))
			})?;

			let placeholder = format!("{{{}}}", param_name);
			result = result.replace(&placeholder, value);
		}

		Ok(result)
	}
}

/// Extract parameter names from a URL pattern
///
/// # Examples
///
/// ```
/// use reinhardt_routers::namespace::extract_param_names;
///
/// let params = extract_param_names("/users/{id}/posts/{post_id}/");
/// assert_eq!(params, vec!["id", "post_id"]);
/// ```
pub fn extract_param_names(pattern: &str) -> Vec<String> {
	let mut params = Vec::new();
	let mut chars = pattern.chars().peekable();

	while let Some(ch) = chars.next() {
		if ch == '{' {
			let mut param_name = String::new();
			while let Some(&next_ch) = chars.peek() {
				if next_ch == '}' {
					chars.next(); // consume '}'
					params.push(param_name);
					break;
				}
				param_name.push(chars.next().unwrap());
			}
		}
	}

	params
}

/// Namespace-based URL resolver
///
/// This resolver provides hierarchical URL reversal using namespaces.
/// It's similar to Django's URL namespace system.
///
/// # Examples
///
/// ```
/// use reinhardt_routers::namespace::NamespaceResolver;
///
/// let mut resolver = NamespaceResolver::new();
///
/// // Register routes
/// resolver.register("api:v1:users:list", "/api/v1/users/");
/// resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
///
/// // Resolve URLs
/// let url = resolver.resolve("api:v1:users:detail", &[("id", "123")]).unwrap();
/// assert_eq!(url, "/api/v1/users/123/");
/// ```
#[derive(Debug, Clone)]
pub struct NamespaceResolver {
	/// Map of full route names to routes
	routes: HashMap<String, NamespacedRoute>,

	/// Map of namespaces to route names within that namespace
	namespace_index: HashMap<String, Vec<String>>,
}

impl NamespaceResolver {
	/// Create a new namespace resolver
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let resolver = NamespaceResolver::new();
	/// ```
	pub fn new() -> Self {
		Self {
			routes: HashMap::new(),
			namespace_index: HashMap::new(),
		}
	}

	/// Register a route with its namespace
	///
	/// # Arguments
	///
	/// * `full_name` - Full route name including namespace (e.g., "api:v1:users:detail")
	/// * `pattern` - URL pattern (e.g., "/api/v1/users/{id}/")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let mut resolver = NamespaceResolver::new();
	/// resolver.register("api:v1:users:list", "/api/v1/users/");
	/// ```
	pub fn register(&mut self, full_name: impl Into<String>, pattern: impl Into<String>) {
		let full_name = full_name.into();
		let route = NamespacedRoute::new(&full_name, pattern);

		// Index by namespace
		let ns_path = route.namespace.full_path().to_string();
		self.namespace_index
			.entry(ns_path)
			.or_default()
			.push(full_name.clone());

		// Store route
		self.routes.insert(full_name, route);
	}

	/// Resolve a route name to a URL
	///
	/// # Arguments
	///
	/// * `name` - Full route name including namespace (e.g., "api:v1:users:detail")
	/// * `params` - URL parameters as key-value pairs
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let mut resolver = NamespaceResolver::new();
	/// resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
	///
	/// let url = resolver.resolve("api:v1:users:detail", &[("id", "123")]).unwrap();
	/// assert_eq!(url, "/api/v1/users/123/");
	/// ```
	pub fn resolve(&self, name: &str, params: &[(&str, &str)]) -> ReverseResult<String> {
		let route = self
			.routes
			.get(name)
			.ok_or_else(|| ReverseError::NotFound(name.to_string()))?;

		route.resolve(params)
	}

	/// List all routes in a namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let mut resolver = NamespaceResolver::new();
	/// resolver.register("api:v1:users:list", "/api/v1/users/");
	/// resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
	/// resolver.register("api:v2:users:list", "/api/v2/users/");
	///
	/// let routes = resolver.list_routes_in_namespace("api:v1");
	/// assert_eq!(routes.len(), 2);
	/// ```
	pub fn list_routes_in_namespace(&self, namespace: &str) -> Vec<&NamespacedRoute> {
		let ns = Namespace::new(namespace);
		self.routes
			.values()
			.filter(|route| route.namespace == ns)
			.collect()
	}

	/// List all child namespaces of a given namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let mut resolver = NamespaceResolver::new();
	/// resolver.register("api:v1:users:list", "/api/v1/users/");
	/// resolver.register("api:v1:posts:list", "/api/v1/posts/");
	/// resolver.register("api:v2:users:list", "/api/v2/users/");
	///
	/// let children = resolver.list_child_namespaces("api");
	/// assert!(children.contains(&"v1".to_string()));
	/// assert!(children.contains(&"v2".to_string()));
	/// ```
	pub fn list_child_namespaces(&self, namespace: &str) -> Vec<String> {
		let parent_ns = Namespace::new(namespace);
		let parent_depth = parent_ns.depth();

		let mut children = std::collections::HashSet::new();

		for route in self.routes.values() {
			if route.namespace.is_child_of(&parent_ns) {
				// Get the immediate child component
				if let Some(child_component) = route.namespace.components().get(parent_depth) {
					children.insert(child_component.clone());
				}
			}
		}

		let mut result: Vec<String> = children.into_iter().collect();
		result.sort();
		result
	}

	/// Get all namespaces
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let mut resolver = NamespaceResolver::new();
	/// resolver.register("api:v1:users:list", "/api/v1/users/");
	/// resolver.register("api:v2:posts:detail", "/api/v2/posts/{id}/");
	///
	/// let namespaces = resolver.list_all_namespaces();
	/// assert!(namespaces.contains(&"api:v1".to_string()));
	/// assert!(namespaces.contains(&"api:v2".to_string()));
	/// ```
	pub fn list_all_namespaces(&self) -> Vec<String> {
		let mut namespaces: Vec<String> = self.namespace_index.keys().cloned().collect();
		namespaces.sort();
		namespaces
	}

	/// Get all routes
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let mut resolver = NamespaceResolver::new();
	/// resolver.register("api:v1:users:list", "/api/v1/users/");
	/// resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
	///
	/// let routes = resolver.all_routes();
	/// assert_eq!(routes.len(), 2);
	/// ```
	pub fn all_routes(&self) -> Vec<&NamespacedRoute> {
		self.routes.values().collect()
	}

	/// Check if a route exists
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::namespace::NamespaceResolver;
	///
	/// let mut resolver = NamespaceResolver::new();
	/// resolver.register("api:v1:users:list", "/api/v1/users/");
	///
	/// assert!(resolver.has_route("api:v1:users:list"));
	/// assert!(!resolver.has_route("api:v2:users:list"));
	/// ```
	pub fn has_route(&self, name: &str) -> bool {
		self.routes.contains_key(name)
	}

	/// Get the number of registered routes
	pub fn route_count(&self) -> usize {
		self.routes.len()
	}

	/// Get the number of registered namespaces
	pub fn namespace_count(&self) -> usize {
		self.namespace_index.len()
	}
}

impl Default for NamespaceResolver {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_namespace_creation() {
		let ns = Namespace::new("api:v1:users");
		assert_eq!(ns.full_path(), "api:v1:users");
		assert_eq!(ns.components(), &["api", "v1", "users"]);
	}

	#[test]
	fn test_namespace_root() {
		let ns = Namespace::new("api:v1:users");
		assert_eq!(ns.root(), Some("api"));

		let empty = Namespace::new("");
		assert_eq!(empty.root(), None);
	}

	#[test]
	fn test_namespace_parent() {
		let ns = Namespace::new("api:v1:users");
		let parent = ns.parent().unwrap();
		assert_eq!(parent.full_path(), "api:v1");

		let root = Namespace::new("api");
		assert!(root.parent().is_none());
	}

	#[test]
	fn test_namespace_leaf() {
		let ns = Namespace::new("api:v1:users");
		assert_eq!(ns.leaf(), Some("users"));
	}

	#[test]
	fn test_namespace_is_parent_of() {
		let parent = Namespace::new("api:v1");
		let child = Namespace::new("api:v1:users");
		let other = Namespace::new("api:v2");

		assert!(parent.is_parent_of(&child));
		assert!(!parent.is_parent_of(&other));
		assert!(!child.is_parent_of(&parent));
	}

	#[test]
	fn test_namespace_append() {
		let parent = Namespace::new("api:v1");
		let child = parent.append("users");
		assert_eq!(child.full_path(), "api:v1:users");
	}

	#[test]
	fn test_namespace_depth() {
		assert_eq!(Namespace::new("api").depth(), 1);
		assert_eq!(Namespace::new("api:v1").depth(), 2);
		assert_eq!(Namespace::new("api:v1:users").depth(), 3);
	}

	#[test]
	fn test_extract_param_names() {
		let params = extract_param_names("/users/{id}/posts/{post_id}/");
		assert_eq!(params, vec!["id", "post_id"]);

		let no_params = extract_param_names("/users/");
		assert!(no_params.is_empty());
	}

	#[test]
	fn test_namespaced_route_creation() {
		let route = NamespacedRoute::new("api:v1:users:detail", "/api/v1/users/{id}/");
		assert_eq!(route.namespace.full_path(), "api:v1:users");
		assert_eq!(route.route_name, "detail");
		assert_eq!(route.pattern, "/api/v1/users/{id}/");
		assert_eq!(route.param_names, vec!["id"]);
	}

	#[test]
	fn test_namespaced_route_resolve() {
		let route = NamespacedRoute::new("api:v1:users:detail", "/api/v1/users/{id}/");
		let url = route.resolve(&[("id", "123")]).unwrap();
		assert_eq!(url, "/api/v1/users/123/");
	}

	#[test]
	fn test_namespace_resolver_register_and_resolve() {
		let mut resolver = NamespaceResolver::new();
		resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");

		let url = resolver
			.resolve("api:v1:users:detail", &[("id", "123")])
			.unwrap();
		assert_eq!(url, "/api/v1/users/123/");
	}

	#[test]
	fn test_namespace_resolver_list_routes() {
		let mut resolver = NamespaceResolver::new();
		resolver.register("api:v1:users:list", "/api/v1/users/");
		resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
		resolver.register("api:v2:users:list", "/api/v2/users/");

		let routes = resolver.list_routes_in_namespace("api:v1:users");
		assert_eq!(routes.len(), 2);
	}

	#[test]
	fn test_namespace_resolver_list_child_namespaces() {
		let mut resolver = NamespaceResolver::new();
		resolver.register("api:v1:users:list", "/api/v1/users/");
		resolver.register("api:v1:posts:list", "/api/v1/posts/");
		resolver.register("api:v2:users:list", "/api/v2/users/");

		let children = resolver.list_child_namespaces("api:v1");
		assert_eq!(children.len(), 2);
		assert!(children.contains(&"users".to_string()));
		assert!(children.contains(&"posts".to_string()));
	}

	#[test]
	fn test_namespace_resolver_has_route() {
		let mut resolver = NamespaceResolver::new();
		resolver.register("api:v1:users:list", "/api/v1/users/");

		assert!(resolver.has_route("api:v1:users:list"));
		assert!(!resolver.has_route("api:v2:users:list"));
	}
}
