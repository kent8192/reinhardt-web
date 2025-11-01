//! Route introspection for debugging and analysis
//!
//! This module provides tools for inspecting registered routes at runtime.
//! It's useful for debugging, documentation generation, and route analysis.
//!
//! # Examples
//!
//! ```
//! use reinhardt_routers::introspection::{RouteInspector, RouteInfo};
//! use hyper::Method;
//!
//! let mut inspector = RouteInspector::new();
//!
//! // Register routes
//! inspector.add_route(
//!     "/api/v1/users/",
//!     vec![Method::GET, Method::POST],
//!     Some("api:v1:users:list"),
//!     None,
//! );
//!
//! // Query routes
//! let routes = inspector.all_routes();
//! assert_eq!(routes.len(), 1);
//!
//! // Find routes by pattern
//! let api_routes = inspector.find_by_path_prefix("/api/v1");
//! assert_eq!(api_routes.len(), 1);
//! ```

use crate::namespace::Namespace;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Metadata about a registered route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
	/// URL path pattern
	pub path: String,

	/// HTTP methods supported by this route (stored as strings for serialization)
	pub methods: Vec<String>,

	/// Full route name including namespace (e.g., "api:v1:users:detail")
	pub name: Option<String>,

	/// Namespace component
	pub namespace: Option<String>,

	/// Route name component (without namespace)
	pub route_name: Option<String>,

	/// Parameter names extracted from the path
	pub params: Vec<String>,

	/// Additional metadata
	pub metadata: HashMap<String, String>,
}

impl RouteInfo {
	/// Create a new RouteInfo
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInfo;
	/// use hyper::Method;
	///
	/// let info = RouteInfo::new(
	///     "/api/users/{id}/",
	///     vec![Method::GET, Method::PUT],
	///     Some("api:users:detail"),
	/// );
	///
	/// assert_eq!(info.path, "/api/users/{id}/");
	/// assert_eq!(info.params, vec!["id"]);
	/// ```
	pub fn new(
		path: impl Into<String>,
		methods: Vec<Method>,
		name: Option<impl Into<String>>,
	) -> Self {
		let path = path.into();
		let name = name.map(|n| n.into());

		// Convert Methods to strings for serialization
		let methods: Vec<String> = methods.iter().map(|m| m.as_str().to_string()).collect();

		// Extract parameters from path
		let params = crate::namespace::extract_param_names(&path);

		// Split name into namespace and route_name
		// Django-style: hierarchical namespaces
		// e.g., "api:v1:users:list" -> namespace = "api:v1", route_name = "users:list"
		let (namespace, route_name) = if let Some(ref n) = name {
			let parts: Vec<&str> = n.split(':').collect();
			if parts.len() >= 3 {
				// At least 3 parts: first N-2 are namespace, last 2 are route_name
				let namespace_end = parts.len() - 2;
				let ns = parts[..namespace_end].join(":");
				let rn = parts[namespace_end..].join(":");
				(Some(ns), Some(rn))
			} else if parts.len() == 2 {
				// Exactly 2 parts: first is namespace, second is route_name
				(Some(parts[0].to_string()), Some(parts[1].to_string()))
			} else {
				// Single part: no namespace, just route_name
				(None, Some(n.clone()))
			}
		} else {
			(None, None)
		};

		Self {
			path,
			methods,
			name,
			namespace,
			route_name,
			params,
			metadata: HashMap::new(),
		}
	}

	/// Add metadata to this route
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInfo;
	/// use hyper::Method;
	///
	/// let mut info = RouteInfo::new("/users/", vec![Method::GET], None::<String>);
	/// info.add_metadata("description", "List all users");
	/// info.add_metadata("tags", "users,api");
	///
	/// assert_eq!(info.metadata.get("description"), Some(&"List all users".to_string()));
	/// ```
	pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
		self.metadata.insert(key.into(), value.into());
	}

	/// Check if this route supports a given HTTP method
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInfo;
	/// use hyper::Method;
	///
	/// let info = RouteInfo::new("/users/", vec![Method::GET, Method::POST], None::<String>);
	///
	/// assert!(info.supports_method(&Method::GET));
	/// assert!(info.supports_method(&Method::POST));
	/// assert!(!info.supports_method(&Method::DELETE));
	/// ```
	pub fn supports_method(&self, method: &Method) -> bool {
		self.methods.contains(&method.as_str().to_string())
	}

	/// Get the namespace as a Namespace object
	pub fn namespace_object(&self) -> Option<Namespace> {
		self.namespace.as_ref().map(|ns| Namespace::new(ns))
	}
}

/// Route inspector for analyzing registered routes
///
/// # Examples
///
/// ```
/// use reinhardt_routers::introspection::RouteInspector;
/// use hyper::Method;
///
/// let mut inspector = RouteInspector::new();
///
/// inspector.add_route("/api/users/", vec![Method::GET], Some("api:users:list"), None);
/// inspector.add_route("/api/users/{id}/", vec![Method::GET], Some("api:users:detail"), None);
///
/// assert_eq!(inspector.route_count(), 2);
/// ```
pub struct RouteInspector {
	routes: Vec<RouteInfo>,
	path_index: HashMap<String, usize>,
	name_index: HashMap<String, usize>,
}

impl RouteInspector {
	/// Create a new route inspector
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	///
	/// let inspector = RouteInspector::new();
	/// assert_eq!(inspector.route_count(), 0);
	/// ```
	pub fn new() -> Self {
		Self {
			routes: Vec::new(),
			path_index: HashMap::new(),
			name_index: HashMap::new(),
		}
	}

	/// Add a route to the inspector
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route(
	///     "/api/users/",
	///     vec![Method::GET, Method::POST],
	///     Some("api:users:list"),
	///     None,
	/// );
	///
	/// assert_eq!(inspector.route_count(), 1);
	/// ```
	pub fn add_route(
		&mut self,
		path: impl Into<String>,
		methods: Vec<Method>,
		name: Option<impl Into<String>>,
		metadata: Option<HashMap<String, String>>,
	) {
		let path = path.into();
		let mut route = RouteInfo::new(&path, methods, name);

		if let Some(meta) = metadata {
			route.metadata = meta;
		}

		let index = self.routes.len();

		// Index by path
		self.path_index.insert(path.clone(), index);

		// Index by name
		if let Some(ref name) = route.name {
			self.name_index.insert(name.clone(), index);
		}

		self.routes.push(route);
	}

	/// Get all registered routes
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], None::<String>, None);
	/// inspector.add_route("/posts/", vec![Method::GET], None::<String>, None);
	///
	/// assert_eq!(inspector.all_routes().len(), 2);
	/// ```
	pub fn all_routes(&self) -> &[RouteInfo] {
		&self.routes
	}

	/// Find a route by path
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], Some("users:list"), None);
	///
	/// let route = inspector.find_by_path("/users/").unwrap();
	/// assert_eq!(route.path, "/users/");
	/// ```
	pub fn find_by_path(&self, path: &str) -> Option<&RouteInfo> {
		self.path_index.get(path).map(|&idx| &self.routes[idx])
	}

	/// Find a route by name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], Some("users:list"), None);
	///
	/// let route = inspector.find_by_name("users:list").unwrap();
	/// assert_eq!(route.path, "/users/");
	/// ```
	pub fn find_by_name(&self, name: &str) -> Option<&RouteInfo> {
		self.name_index.get(name).map(|&idx| &self.routes[idx])
	}

	/// Find routes by path prefix
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/api/v1/users/", vec![Method::GET], None::<String>, None);
	/// inspector.add_route("/api/v1/posts/", vec![Method::GET], None::<String>, None);
	/// inspector.add_route("/api/v2/users/", vec![Method::GET], None::<String>, None);
	///
	/// let routes = inspector.find_by_path_prefix("/api/v1");
	/// assert_eq!(routes.len(), 2);
	/// ```
	pub fn find_by_path_prefix(&self, prefix: &str) -> Vec<&RouteInfo> {
		self.routes
			.iter()
			.filter(|route| route.path.starts_with(prefix))
			.collect()
	}

	/// Find routes by namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/api/users/", vec![Method::GET], Some("api:v1:users:list"), None);
	/// inspector.add_route("/api/posts/", vec![Method::GET], Some("api:v1:posts:list"), None);
	/// inspector.add_route("/api/users/", vec![Method::GET], Some("api:v2:users:list"), None);
	///
	/// let routes = inspector.find_by_namespace("api:v1");
	/// assert_eq!(routes.len(), 2);
	/// ```
	pub fn find_by_namespace(&self, namespace: &str) -> Vec<&RouteInfo> {
		self.routes
			.iter()
			.filter(|route| {
				route
					.namespace
					.as_ref()
					.map(|ns| ns == namespace)
					.unwrap_or(false)
			})
			.collect()
	}

	/// Find routes by HTTP method
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], None::<String>, None);
	/// inspector.add_route("/users/", vec![Method::POST], None::<String>, None);
	/// inspector.add_route("/posts/", vec![Method::GET], None::<String>, None);
	///
	/// let routes = inspector.find_by_method(&Method::GET);
	/// assert_eq!(routes.len(), 2);
	/// ```
	pub fn find_by_method(&self, method: &Method) -> Vec<&RouteInfo> {
		self.routes
			.iter()
			.filter(|route| route.supports_method(method))
			.collect()
	}

	/// Get all unique namespaces
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], Some("api:v1:users:list"), None);
	/// inspector.add_route("/posts/", vec![Method::GET], Some("api:v1:posts:list"), None);
	/// inspector.add_route("/users/", vec![Method::GET], Some("api:v2:users:list"), None);
	///
	/// let namespaces = inspector.all_namespaces();
	/// assert_eq!(namespaces.len(), 2);
	/// assert!(namespaces.contains(&"api:v1".to_string()));
	/// assert!(namespaces.contains(&"api:v2".to_string()));
	/// ```
	pub fn all_namespaces(&self) -> Vec<String> {
		let mut namespaces: HashSet<String> = self
			.routes
			.iter()
			.filter_map(|route| route.namespace.clone())
			.collect();

		let mut result: Vec<String> = namespaces.drain().collect();
		result.sort();
		result
	}

	/// Get all unique HTTP methods used
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET, Method::POST], None::<String>, None);
	/// inspector.add_route("/posts/", vec![Method::GET, Method::DELETE], None::<String>, None);
	///
	/// let methods = inspector.all_methods();
	/// assert!(methods.contains(&Method::GET));
	/// assert!(methods.contains(&Method::POST));
	/// assert!(methods.contains(&Method::DELETE));
	/// ```
	pub fn all_methods(&self) -> Vec<Method> {
		let mut methods: HashSet<String> = HashSet::new();
		for route in &self.routes {
			for method in &route.methods {
				methods.insert(method.clone());
			}
		}

		let mut result: Vec<Method> = methods.into_iter().filter_map(|m| m.parse().ok()).collect();
		result.sort_by(|a, b| a.as_str().cmp(b.as_str()));
		result
	}

	/// Get route statistics
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], Some("api:users:list"), None);
	/// inspector.add_route("/posts/", vec![Method::GET], Some("api:posts:list"), None);
	///
	/// let stats = inspector.statistics();
	/// assert_eq!(stats.total_routes, 2);
	/// assert_eq!(stats.total_namespaces, 1);
	/// ```
	pub fn statistics(&self) -> RouteStatistics {
		let total_routes = self.routes.len();
		let total_namespaces = self.all_namespaces().len();
		let total_methods = self.all_methods().len();

		let routes_with_params = self
			.routes
			.iter()
			.filter(|route| !route.params.is_empty())
			.count();

		let routes_with_names = self
			.routes
			.iter()
			.filter(|route| route.name.is_some())
			.count();

		RouteStatistics {
			total_routes,
			total_namespaces,
			total_methods,
			routes_with_params,
			routes_with_names,
		}
	}

	/// Get the number of registered routes
	pub fn route_count(&self) -> usize {
		self.routes.len()
	}

	/// Export routes as JSON
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], Some("users:list"), None);
	///
	/// let json = inspector.to_json().unwrap();
	/// assert!(json.contains("users:list"));
	/// ```
	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string_pretty(&self.routes)
	}

	/// Export routes as YAML
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_routers::introspection::RouteInspector;
	/// use hyper::Method;
	///
	/// let mut inspector = RouteInspector::new();
	/// inspector.add_route("/users/", vec![Method::GET], Some("users:list"), None);
	///
	/// let yaml = inspector.to_yaml().unwrap();
	/// assert!(yaml.contains("users:list"));
	/// ```
	pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
		serde_yaml::to_string(&self.routes)
	}
}

impl Default for RouteInspector {
	fn default() -> Self {
		Self::new()
	}
}

/// Statistics about registered routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStatistics {
	/// Total number of routes
	pub total_routes: usize,

	/// Total number of unique namespaces
	pub total_namespaces: usize,

	/// Total number of unique HTTP methods
	pub total_methods: usize,

	/// Number of routes with parameters
	pub routes_with_params: usize,

	/// Number of routes with names
	pub routes_with_names: usize,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_route_info_creation() {
		let info = RouteInfo::new("/users/{id}/", vec![Method::GET], Some("users:detail"));

		assert_eq!(info.path, "/users/{id}/");
		assert_eq!(info.params, vec!["id"]);
		assert_eq!(info.name, Some("users:detail".to_string()));
		assert_eq!(info.namespace, Some("users".to_string()));
		assert_eq!(info.route_name, Some("detail".to_string()));
	}

	#[test]
	fn test_route_info_supports_method() {
		let info = RouteInfo::new("/users/", vec![Method::GET, Method::POST], None::<String>);

		assert!(info.supports_method(&Method::GET));
		assert!(info.supports_method(&Method::POST));
		assert!(!info.supports_method(&Method::DELETE));
	}

	#[test]
	fn test_route_info_metadata() {
		let mut info = RouteInfo::new("/users/", vec![Method::GET], None::<String>);
		info.add_metadata("description", "List users");

		assert_eq!(
			info.metadata.get("description"),
			Some(&"List users".to_string())
		);
	}

	#[test]
	fn test_route_inspector_add_and_count() {
		let mut inspector = RouteInspector::new();
		inspector.add_route(
			"/users/",
			vec![Method::GET],
			None::<String>,
			None::<std::collections::HashMap<String, String>>,
		);
		inspector.add_route(
			"/posts/",
			vec![Method::GET],
			None::<String>,
			None::<std::collections::HashMap<String, String>>,
		);

		assert_eq!(inspector.route_count(), 2);
	}

	#[test]
	fn test_route_inspector_find_by_path() {
		let mut inspector = RouteInspector::new();
		inspector.add_route("/users/", vec![Method::GET], Some("users:list"), None);

		let route = inspector.find_by_path("/users/").unwrap();
		assert_eq!(route.name, Some("users:list".to_string()));
	}

	#[test]
	fn test_route_inspector_find_by_name() {
		let mut inspector = RouteInspector::new();
		inspector.add_route("/users/", vec![Method::GET], Some("users:list"), None);

		let route = inspector.find_by_name("users:list").unwrap();
		assert_eq!(route.path, "/users/");
	}

	#[test]
	fn test_route_inspector_find_by_prefix() {
		let mut inspector = RouteInspector::new();
		inspector.add_route(
			"/api/v1/users/",
			vec![Method::GET],
			None::<String>,
			None::<std::collections::HashMap<String, String>>,
		);
		inspector.add_route(
			"/api/v1/posts/",
			vec![Method::GET],
			None::<String>,
			None::<std::collections::HashMap<String, String>>,
		);
		inspector.add_route(
			"/api/v2/users/",
			vec![Method::GET],
			None::<String>,
			None::<std::collections::HashMap<String, String>>,
		);

		let routes = inspector.find_by_path_prefix("/api/v1");
		assert_eq!(routes.len(), 2);
	}

	#[test]
	fn test_route_inspector_find_by_namespace() {
		let mut inspector = RouteInspector::new();
		inspector.add_route(
			"/users/",
			vec![Method::GET],
			Some("api:v1:users:list"),
			None,
		);
		inspector.add_route(
			"/posts/",
			vec![Method::GET],
			Some("api:v1:posts:list"),
			None,
		);
		inspector.add_route(
			"/users/",
			vec![Method::GET],
			Some("api:v2:users:list"),
			None,
		);

		let routes = inspector.find_by_namespace("api:v1");
		assert_eq!(routes.len(), 2);
	}

	#[test]
	fn test_route_inspector_all_namespaces() {
		let mut inspector = RouteInspector::new();
		inspector.add_route(
			"/users/",
			vec![Method::GET],
			Some("api:v1:users:list"),
			None,
		);
		inspector.add_route(
			"/posts/",
			vec![Method::GET],
			Some("api:v2:posts:list"),
			None,
		);

		let namespaces = inspector.all_namespaces();
		assert_eq!(namespaces.len(), 2);
		assert!(namespaces.contains(&"api:v1".to_string()));
		assert!(namespaces.contains(&"api:v2".to_string()));
	}

	#[test]
	fn test_route_inspector_statistics() {
		let mut inspector = RouteInspector::new();
		inspector.add_route("/users/", vec![Method::GET], Some("api:users:list"), None);
		inspector.add_route(
			"/users/{id}/",
			vec![Method::GET],
			Some("api:users:detail"),
			None,
		);

		let stats = inspector.statistics();
		assert_eq!(stats.total_routes, 2);
		assert_eq!(stats.routes_with_params, 1);
		assert_eq!(stats.routes_with_names, 2);
	}

	#[test]
	fn test_route_inspector_to_json() {
		let mut inspector = RouteInspector::new();
		inspector.add_route("/users/", vec![Method::GET], Some("users:list"), None);

		let json = inspector.to_json().unwrap();
		assert!(json.contains("users:list"));
	}
}
