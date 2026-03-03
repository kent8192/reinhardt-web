use reinhardt_urls::routers::namespace::{
	Namespace, NamespaceResolver, NamespacedRoute, extract_param_names,
};
use rstest::rstest;

// ============================================================================
// Namespace creation and basic properties
// ============================================================================

#[rstest]
fn test_namespace_new_single_component() {
	// Arrange + Act
	let ns = Namespace::new("api");

	// Assert
	assert_eq!(ns.full_path(), "api");
	assert_eq!(ns.components(), &["api"]);
}

#[rstest]
fn test_namespace_new_multi_component() {
	// Arrange + Act
	let ns = Namespace::new("api:v1:users");

	// Assert
	assert_eq!(ns.full_path(), "api:v1:users");
	assert_eq!(ns.components(), &["api", "v1", "users"]);
}

#[rstest]
fn test_namespace_new_empty() {
	// Arrange + Act
	let ns = Namespace::new("");

	// Assert
	assert_eq!(ns.full_path(), "");
	assert_eq!(ns.components(), &[] as &[String]);
}

#[rstest]
#[case("api", Some("api"))]
#[case("api:v1", Some("api"))]
#[case("api:v1:users", Some("api"))]
#[case("", None)]
fn test_namespace_root(#[case] path: &str, #[case] expected: Option<&str>) {
	// Arrange
	let ns = Namespace::new(path);

	// Act
	let root = ns.root();

	// Assert
	assert_eq!(root, expected);
}

#[rstest]
#[case("api:v1:users", Some("users"))]
#[case("api:v1", Some("v1"))]
#[case("api", Some("api"))]
#[case("", None)]
fn test_namespace_leaf(#[case] path: &str, #[case] expected: Option<&str>) {
	// Arrange
	let ns = Namespace::new(path);

	// Act
	let leaf = ns.leaf();

	// Assert
	assert_eq!(leaf, expected);
}

#[rstest]
#[case("api", 1)]
#[case("api:v1", 2)]
#[case("api:v1:users", 3)]
#[case("api:v1:users:detail", 4)]
fn test_namespace_depth(#[case] path: &str, #[case] expected: usize) {
	// Arrange
	let ns = Namespace::new(path);

	// Act
	let depth = ns.depth();

	// Assert
	assert_eq!(depth, expected);
}

#[rstest]
fn test_namespace_parent_from_deep() {
	// Arrange
	let ns = Namespace::new("api:v1:users");

	// Act
	let parent = ns.parent().unwrap();

	// Assert
	assert_eq!(parent.full_path(), "api:v1");
	assert_eq!(parent.components(), &["api", "v1"]);
}

#[rstest]
fn test_namespace_parent_from_root_returns_none() {
	// Arrange
	let ns = Namespace::new("api");

	// Act
	let parent = ns.parent();

	// Assert
	assert!(parent.is_none());
}

#[rstest]
fn test_namespace_parent_chain() {
	// Arrange
	let ns = Namespace::new("api:v1:users:detail");

	// Act
	let parent1 = ns.parent().unwrap();
	let parent2 = parent1.parent().unwrap();
	let parent3 = parent2.parent().unwrap();

	// Assert
	assert_eq!(parent1.full_path(), "api:v1:users");
	assert_eq!(parent2.full_path(), "api:v1");
	assert_eq!(parent3.full_path(), "api");
	assert!(parent3.parent().is_none());
}

#[rstest]
fn test_namespace_append() {
	// Arrange
	let ns = Namespace::new("api:v1");

	// Act
	let child = ns.append("users");

	// Assert
	assert_eq!(child.full_path(), "api:v1:users");
	assert_eq!(child.components(), &["api", "v1", "users"]);
}

#[rstest]
fn test_namespace_append_to_empty() {
	// Arrange
	let ns = Namespace::new("");

	// Act
	let child = ns.append("api");

	// Assert
	assert_eq!(child.full_path(), "api");
}

#[rstest]
fn test_namespace_is_parent_of_direct_child() {
	// Arrange
	let parent = Namespace::new("api:v1");
	let child = Namespace::new("api:v1:users");

	// Act + Assert
	assert!(parent.is_parent_of(&child));
}

#[rstest]
fn test_namespace_is_parent_of_deep_descendant() {
	// Arrange
	let ancestor = Namespace::new("api");
	let descendant = Namespace::new("api:v1:users:detail");

	// Act + Assert
	assert!(ancestor.is_parent_of(&descendant));
}

#[rstest]
fn test_namespace_is_not_parent_of_sibling() {
	// Arrange
	let ns1 = Namespace::new("api:v1");
	let ns2 = Namespace::new("api:v2");

	// Act + Assert
	assert!(!ns1.is_parent_of(&ns2));
}

#[rstest]
fn test_namespace_is_not_parent_of_itself() {
	// Arrange
	let ns = Namespace::new("api:v1");

	// Act + Assert
	assert!(!ns.is_parent_of(&ns));
}

#[rstest]
fn test_namespace_is_child_of() {
	// Arrange
	let parent = Namespace::new("api:v1");
	let child = Namespace::new("api:v1:users");

	// Act + Assert
	assert!(child.is_child_of(&parent));
	assert!(!parent.is_child_of(&child));
}

#[rstest]
fn test_namespace_display() {
	// Arrange
	let ns = Namespace::new("api:v1:users");

	// Act
	let displayed = format!("{}", ns);

	// Assert
	assert_eq!(displayed, "api:v1:users");
}

#[rstest]
fn test_namespace_from_str() {
	// Arrange + Act
	let ns: Namespace = "api:v1:users".into();

	// Assert
	assert_eq!(ns.full_path(), "api:v1:users");
}

#[rstest]
fn test_namespace_from_string() {
	// Arrange + Act
	let ns: Namespace = String::from("api:v1:users").into();

	// Assert
	assert_eq!(ns.full_path(), "api:v1:users");
}

#[rstest]
fn test_namespace_equality() {
	// Arrange
	let ns1 = Namespace::new("api:v1:users");
	let ns2 = Namespace::new("api:v1:users");
	let ns3 = Namespace::new("api:v2:users");

	// Act + Assert
	assert_eq!(ns1, ns2);
	assert_ne!(ns1, ns3);
}

// ============================================================================
// extract_param_names
// ============================================================================

#[rstest]
#[case("/users/", vec![])]
#[case("/users/{id}/", vec!["id"])]
#[case("/users/{id}/posts/{post_id}/", vec!["id", "post_id"])]
#[case("/{a}{b}/", vec!["a", "b"])]
fn test_extract_param_names(#[case] pattern: &str, #[case] expected: Vec<&str>) {
	// Arrange + Act
	let params = extract_param_names(pattern);

	// Assert
	let expected_owned: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
	assert_eq!(params, expected_owned);
}

// ============================================================================
// NamespacedRoute creation and resolution
// ============================================================================

#[rstest]
fn test_namespaced_route_with_namespace_and_route_name() {
	// Arrange + Act
	let route = NamespacedRoute::new("api:v1:users:detail", "/api/v1/users/{id}/");

	// Assert
	assert_eq!(route.full_name, "api:v1:users:detail");
	assert_eq!(route.namespace.full_path(), "api:v1:users");
	assert_eq!(route.route_name, "detail");
	assert_eq!(route.pattern, "/api/v1/users/{id}/");
	assert_eq!(route.param_names, vec!["id"]);
}

#[rstest]
fn test_namespaced_route_single_component_no_namespace() {
	// Arrange + Act
	let route = NamespacedRoute::new("list", "/users/");

	// Assert
	assert_eq!(route.full_name, "list");
	assert_eq!(route.namespace.full_path(), "");
	assert_eq!(route.route_name, "list");
	assert_eq!(route.pattern, "/users/");
}

#[rstest]
fn test_namespaced_route_resolve_single_param() {
	// Arrange
	let route = NamespacedRoute::new("api:users:detail", "/users/{id}/");

	// Act
	let url = route.resolve(&[("id", "42")]).unwrap();

	// Assert
	assert_eq!(url, "/users/42/");
}

#[rstest]
fn test_namespaced_route_resolve_multiple_params() {
	// Arrange
	let route = NamespacedRoute::new("api:posts:detail", "/users/{user_id}/posts/{post_id}/");

	// Act
	let url = route
		.resolve(&[("user_id", "1"), ("post_id", "99")])
		.unwrap();

	// Assert
	assert_eq!(url, "/users/1/posts/99/");
}

#[rstest]
fn test_namespaced_route_resolve_no_params() {
	// Arrange
	let route = NamespacedRoute::new("api:users:list", "/api/users/");

	// Act
	let url = route.resolve(&[]).unwrap();

	// Assert
	assert_eq!(url, "/api/users/");
}

#[rstest]
fn test_namespaced_route_resolve_missing_param_returns_error() {
	// Arrange
	let route = NamespacedRoute::new("api:users:detail", "/users/{id}/");

	// Act
	let result = route.resolve(&[]);

	// Assert
	assert!(result.is_err());
}

// ============================================================================
// NamespaceResolver: register and resolve
// ============================================================================

#[rstest]
fn test_resolver_new_is_empty() {
	// Arrange + Act
	let resolver = NamespaceResolver::new();

	// Assert
	assert_eq!(resolver.route_count(), 0);
	assert_eq!(resolver.namespace_count(), 0);
}

#[rstest]
fn test_resolver_default_is_empty() {
	// Arrange + Act
	let resolver = NamespaceResolver::default();

	// Assert
	assert_eq!(resolver.route_count(), 0);
}

#[rstest]
fn test_resolver_register_and_resolve_with_param() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");

	// Act
	let url = resolver
		.resolve("api:v1:users:detail", &[("id", "123")])
		.unwrap();

	// Assert
	assert_eq!(url, "/api/v1/users/123/");
}

#[rstest]
fn test_resolver_register_and_resolve_no_params() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");

	// Act
	let url = resolver.resolve("api:v1:users:list", &[]).unwrap();

	// Assert
	assert_eq!(url, "/api/v1/users/");
}

#[rstest]
fn test_resolver_resolve_unknown_name_returns_error() {
	// Arrange
	let resolver = NamespaceResolver::new();

	// Act
	let result = resolver.resolve("nonexistent:route", &[]);

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_resolver_resolve_missing_param_returns_error() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:users:detail", "/users/{id}/");

	// Act
	let result = resolver.resolve("api:users:detail", &[]);

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_resolver_has_route_registered() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:users:list", "/api/users/");

	// Act + Assert
	assert!(resolver.has_route("api:users:list"));
	assert!(!resolver.has_route("api:users:detail"));
}

#[rstest]
fn test_resolver_route_count() {
	// Arrange
	let mut resolver = NamespaceResolver::new();

	// Act
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
	resolver.register("api:v2:users:list", "/api/v2/users/");

	// Assert
	assert_eq!(resolver.route_count(), 3);
}

// ============================================================================
// NamespaceResolver: list_routes_in_namespace
// ============================================================================

#[rstest]
fn test_resolver_list_routes_in_namespace_exact_match() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
	resolver.register("api:v2:users:list", "/api/v2/users/");

	// Act
	let routes = resolver.list_routes_in_namespace("api:v1:users");

	// Assert
	assert_eq!(routes.len(), 2);
}

#[rstest]
fn test_resolver_list_routes_in_namespace_no_match() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");

	// Act
	// "api:v1" is a parent, not the exact namespace; routes live under "api:v1:users"
	let routes = resolver.list_routes_in_namespace("api:v1");

	// Assert
	assert_eq!(routes.len(), 0);
}

#[rstest]
fn test_resolver_list_routes_in_namespace_empty_resolver() {
	// Arrange
	let resolver = NamespaceResolver::new();

	// Act
	let routes = resolver.list_routes_in_namespace("api:v1:users");

	// Assert
	assert!(routes.is_empty());
}

// ============================================================================
// NamespaceResolver: list_child_namespaces
// ============================================================================

#[rstest]
fn test_resolver_list_child_namespaces() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("api:v1:posts:list", "/api/v1/posts/");
	resolver.register("api:v2:users:list", "/api/v2/users/");

	// Act
	let children = resolver.list_child_namespaces("api:v1");

	// Assert
	assert_eq!(children.len(), 2);
	assert!(children.contains(&"users".to_string()));
	assert!(children.contains(&"posts".to_string()));
}

#[rstest]
fn test_resolver_list_child_namespaces_returns_sorted() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:zebra:list", "/api/v1/zebra/");
	resolver.register("api:v1:alpha:list", "/api/v1/alpha/");
	resolver.register("api:v1:middle:list", "/api/v1/middle/");

	// Act
	let children = resolver.list_child_namespaces("api:v1");

	// Assert
	assert_eq!(children, vec!["alpha", "middle", "zebra"]);
}

#[rstest]
fn test_resolver_list_child_namespaces_top_level() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("admin:dashboard:list", "/admin/dashboard/");

	// Act
	let children = resolver.list_child_namespaces("api");

	// Assert
	assert_eq!(children, vec!["v1"]);
}

// ============================================================================
// NamespaceResolver: list_all_namespaces and all_routes
// ============================================================================

#[rstest]
fn test_resolver_list_all_namespaces() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("api:v2:posts:detail", "/api/v2/posts/{id}/");

	// Act
	let namespaces = resolver.list_all_namespaces();

	// Assert
	assert!(namespaces.contains(&"api:v1:users".to_string()));
	assert!(namespaces.contains(&"api:v2:posts".to_string()));
}

#[rstest]
fn test_resolver_all_routes() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");

	// Act
	let routes = resolver.all_routes();

	// Assert
	assert_eq!(routes.len(), 2);
}

// ============================================================================
// Nested namespace scenarios (3+ levels deep)
// ============================================================================

#[rstest]
fn test_resolver_nested_namespace_three_levels() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("api:v1:users:detail", "/api/v1/users/{id}/");
	resolver.register("api:v1:posts:list", "/api/v1/posts/");
	resolver.register("api:v2:users:list", "/api/v2/users/");

	// Act: resolve deeply nested route
	let url = resolver
		.resolve("api:v1:users:detail", &[("id", "7")])
		.unwrap();

	// Assert
	assert_eq!(url, "/api/v1/users/7/");
}

#[rstest]
fn test_resolver_nested_namespace_child_list_isolated() {
	// Arrange
	let mut resolver = NamespaceResolver::new();
	resolver.register("api:v1:users:list", "/api/v1/users/");
	resolver.register("api:v1:posts:list", "/api/v1/posts/");
	resolver.register("api:v2:users:list", "/api/v2/users/");

	// Act: routes in "api:v1:users" namespace should not include "api:v1:posts"
	let user_routes = resolver.list_routes_in_namespace("api:v1:users");
	let post_routes = resolver.list_routes_in_namespace("api:v1:posts");

	// Assert
	assert_eq!(user_routes.len(), 1);
	assert_eq!(post_routes.len(), 1);
	assert_eq!(user_routes[0].pattern, "/api/v1/users/");
	assert_eq!(post_routes[0].pattern, "/api/v1/posts/");
}
