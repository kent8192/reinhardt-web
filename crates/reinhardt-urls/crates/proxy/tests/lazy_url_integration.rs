//! Integration tests for lazy URL resolution and pattern matching
//!
//! Tests lazy URL reverse resolution, URL pattern compilation, parameterized URLs,
//! kwargs-based reverse resolution, and namespace resolution without database dependencies.

use reinhardt_proxy::{LazyUrl, UrlNamespace, UrlPattern, UrlResolver};
use std::collections::HashMap;
use std::sync::Arc;

/// Setup a basic URL resolver with common patterns
fn setup_basic_resolver() -> UrlResolver {
	let mut resolver = UrlResolver::new();

	// Simple patterns
	resolver.add_pattern(UrlPattern::new("home", "/", None));

	resolver.add_pattern(UrlPattern::new("about", "/about/", None));

	resolver.add_pattern(UrlPattern::new("contact", "/contact/", None));

	resolver
}

/// Setup a resolver with parameterized patterns
fn setup_parameterized_resolver() -> UrlResolver {
	let mut resolver = UrlResolver::new();

	// Patterns with parameters
	resolver.add_pattern(UrlPattern::new("user-detail", "/users/<id>/", None));

	resolver.add_pattern(UrlPattern::new("post-detail", "/posts/<slug>/", None));

	resolver.add_pattern(UrlPattern::new(
		"article-detail",
		"/articles/<year>/<month>/<slug>/",
		None,
	));

	resolver.add_pattern(UrlPattern::new(
		"category-posts",
		"/categories/<category>/posts/",
		None,
	));

	resolver
}

/// Setup a resolver with namespaces
fn setup_namespaced_resolver() -> UrlResolver {
	let mut resolver = UrlResolver::new();

	// Admin namespace
	let admin_ns = UrlNamespace::new("admin", "/admin/");
	resolver.add_namespace(admin_ns.clone());

	resolver.add_pattern(UrlPattern::new("admin:index", "/admin/", Some("admin")));

	resolver.add_pattern(UrlPattern::new(
		"admin:users",
		"/admin/users/",
		Some("admin"),
	));

	resolver.add_pattern(UrlPattern::new(
		"admin:user-edit",
		"/admin/users/<id>/edit/",
		Some("admin"),
	));

	// API namespace
	let api_ns = UrlNamespace::new("api", "/api/v1/");
	resolver.add_namespace(api_ns.clone());

	resolver.add_pattern(UrlPattern::new("api:users", "/api/v1/users/", Some("api")));

	resolver.add_pattern(UrlPattern::new(
		"api:user-detail",
		"/api/v1/users/<id>/",
		Some("api"),
	));

	resolver
}

#[test]
fn test_lazy_url_basic_resolution() {
	let resolver = Arc::new(setup_basic_resolver());

	let home_url = LazyUrl::new("home", resolver.clone());
	assert_eq!(home_url.resolve(), "/");

	let about_url = LazyUrl::new("about", resolver.clone());
	assert_eq!(about_url.resolve(), "/about/");

	let contact_url = LazyUrl::new("contact", resolver.clone());
	assert_eq!(contact_url.resolve(), "/contact/");
}

#[test]
fn test_lazy_url_deferred_resolution() {
	let resolver = Arc::new(setup_basic_resolver());

	// Create lazy URL before resolver is fully configured
	let lazy_url = LazyUrl::new("home", resolver.clone());

	// URL should not be resolved yet
	assert!(!lazy_url.is_resolved());

	// Trigger resolution
	let url = lazy_url.resolve();
	assert_eq!(url, "/");

	// Now it should be resolved
	assert!(lazy_url.is_resolved());
}

#[test]
fn test_lazy_url_with_single_parameter() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "123".to_string());

	let lazy_url = LazyUrl::with_kwargs("user-detail", kwargs, resolver.clone());
	assert_eq!(lazy_url.resolve(), "/users/123/");
}

#[test]
fn test_lazy_url_with_multiple_parameters() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("year".to_string(), "2025".to_string());
	kwargs.insert("month".to_string(), "01".to_string());
	kwargs.insert("slug".to_string(), "test-article".to_string());

	let lazy_url = LazyUrl::with_kwargs("article-detail", kwargs, resolver.clone());
	assert_eq!(lazy_url.resolve(), "/articles/2025/01/test-article/");
}

#[test]
fn test_lazy_url_parameter_ordering() {
	let resolver = Arc::new(setup_parameterized_resolver());

	// Parameters provided in different order
	let mut kwargs = HashMap::new();
	kwargs.insert("slug".to_string(), "my-post".to_string());
	kwargs.insert("month".to_string(), "12".to_string());
	kwargs.insert("year".to_string(), "2024".to_string());

	let lazy_url = LazyUrl::with_kwargs("article-detail", kwargs, resolver.clone());
	assert_eq!(lazy_url.resolve(), "/articles/2024/12/my-post/");
}

#[test]
fn test_lazy_url_missing_parameter() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("year".to_string(), "2025".to_string());
	// Missing 'month' and 'slug'

	let lazy_url = LazyUrl::with_kwargs("article-detail", kwargs, resolver.clone());
	let result = std::panic::catch_unwind(|| lazy_url.resolve());

	assert!(result.is_err());
}

#[test]
fn test_lazy_url_extra_parameters_ignored() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "456".to_string());
	kwargs.insert("extra".to_string(), "ignored".to_string());

	let lazy_url = LazyUrl::with_kwargs("user-detail", kwargs, resolver.clone());
	assert_eq!(lazy_url.resolve(), "/users/456/");
}

#[test]
fn test_lazy_url_namespace_simple() {
	let resolver = Arc::new(setup_namespaced_resolver());

	let admin_index = LazyUrl::new("admin:index", resolver.clone());
	assert_eq!(admin_index.resolve(), "/admin/");

	let admin_users = LazyUrl::new("admin:users", resolver.clone());
	assert_eq!(admin_users.resolve(), "/admin/users/");
}

#[test]
fn test_lazy_url_namespace_with_parameters() {
	let resolver = Arc::new(setup_namespaced_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "789".to_string());

	let admin_edit = LazyUrl::with_kwargs("admin:user-edit", kwargs.clone(), resolver.clone());
	assert_eq!(admin_edit.resolve(), "/admin/users/789/edit/");

	let api_detail = LazyUrl::with_kwargs("api:user-detail", kwargs, resolver.clone());
	assert_eq!(api_detail.resolve(), "/api/v1/users/789/");
}

#[test]
fn test_lazy_url_namespace_resolution() {
	let resolver = Arc::new(setup_namespaced_resolver());

	// API namespace URLs
	let api_users = LazyUrl::new("api:users", resolver.clone());
	assert_eq!(api_users.resolve(), "/api/v1/users/");

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "100".to_string());

	let api_detail = LazyUrl::with_kwargs("api:user-detail", kwargs, resolver.clone());
	assert_eq!(api_detail.resolve(), "/api/v1/users/100/");
}

#[test]
fn test_url_pattern_compilation() {
	let pattern = UrlPattern::new("test", "/items/<id>/", None);

	assert_eq!(pattern.name(), "test");
	assert_eq!(pattern.template(), "/items/<id>/");
	assert!(pattern.namespace().is_none());
}

#[test]
fn test_url_pattern_parameter_extraction() {
	let pattern = UrlPattern::new("test", "/items/<id>/details/<action>/", None);

	let params = pattern.extract_parameters();
	assert_eq!(params.len(), 2);
	assert!(params.contains(&"id".to_string()));
	assert!(params.contains(&"action".to_string()));
}

#[test]
fn test_url_pattern_no_parameters() {
	let pattern = UrlPattern::new("test", "/static/path/", None);

	let params = pattern.extract_parameters();
	assert_eq!(params.len(), 0);
}

#[test]
fn test_url_pattern_multiple_parameters_in_sequence() {
	let pattern = UrlPattern::new("test", "/path/<year>/<month>/<day>/<slug>/", None);

	let params = pattern.extract_parameters();
	assert_eq!(params.len(), 4);
	assert!(params.contains(&"year".to_string()));
	assert!(params.contains(&"month".to_string()));
	assert!(params.contains(&"day".to_string()));
	assert!(params.contains(&"slug".to_string()));
}

#[test]
fn test_url_pattern_with_namespace() {
	let pattern = UrlPattern::new("admin:users", "/admin/users/", Some("admin"));

	assert_eq!(pattern.name(), "admin:users");
	assert_eq!(pattern.namespace(), Some("admin"));
}

#[test]
fn test_url_reverse_simple() {
	let resolver = Arc::new(setup_basic_resolver());

	let url = resolver.reverse("home", HashMap::new()).unwrap();
	assert_eq!(url, "/");

	let url = resolver.reverse("about", HashMap::new()).unwrap();
	assert_eq!(url, "/about/");
}

#[test]
fn test_url_reverse_with_kwargs() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "42".to_string());

	let url = resolver.reverse("user-detail", kwargs).unwrap();
	assert_eq!(url, "/users/42/");
}

#[test]
fn test_url_reverse_complex_kwargs() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("year".to_string(), "2025".to_string());
	kwargs.insert("month".to_string(), "06".to_string());
	kwargs.insert("slug".to_string(), "summer-release".to_string());

	let url = resolver.reverse("article-detail", kwargs).unwrap();
	assert_eq!(url, "/articles/2025/06/summer-release/");
}

#[test]
fn test_url_reverse_not_found() {
	let resolver = Arc::new(setup_basic_resolver());

	let result = resolver.reverse("nonexistent", HashMap::new());
	assert!(result.is_err());
}

#[test]
fn test_url_reverse_namespace() {
	let resolver = Arc::new(setup_namespaced_resolver());

	let url = resolver.reverse("admin:index", HashMap::new()).unwrap();
	assert_eq!(url, "/admin/");

	let url = resolver.reverse("api:users", HashMap::new()).unwrap();
	assert_eq!(url, "/api/v1/users/");
}

#[test]
fn test_url_reverse_namespace_with_kwargs() {
	let resolver = Arc::new(setup_namespaced_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "999".to_string());

	let url = resolver.reverse("admin:user-edit", kwargs.clone()).unwrap();
	assert_eq!(url, "/admin/users/999/edit/");

	let url = resolver.reverse("api:user-detail", kwargs).unwrap();
	assert_eq!(url, "/api/v1/users/999/");
}

#[test]
fn test_lazy_url_caching() {
	let resolver = Arc::new(setup_basic_resolver());

	let lazy_url = LazyUrl::new("home", resolver.clone());

	// First resolution
	let url1 = lazy_url.resolve();
	assert_eq!(url1, "/");
	assert!(lazy_url.is_resolved());

	// Second resolution should use cached value
	let url2 = lazy_url.resolve();
	assert_eq!(url2, "/");
	assert_eq!(url1, url2);
}

#[test]
fn test_lazy_url_with_special_characters_in_slug() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("slug".to_string(), "hello-world-2025".to_string());

	let lazy_url = LazyUrl::with_kwargs("post-detail", kwargs, resolver.clone());
	assert_eq!(lazy_url.resolve(), "/posts/hello-world-2025/");
}

#[test]
fn test_lazy_url_numeric_id_parameter() {
	let resolver = Arc::new(setup_parameterized_resolver());

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "12345".to_string());

	let lazy_url = LazyUrl::with_kwargs("user-detail", kwargs, resolver.clone());
	assert_eq!(lazy_url.resolve(), "/users/12345/");
}

#[test]
fn test_url_pattern_matching_simple() {
	let pattern = UrlPattern::new("test", "/users/", None);

	assert!(pattern.matches("/users/"));
	assert!(!pattern.matches("/posts/"));
	assert!(!pattern.matches("/users"));
}

#[test]
fn test_url_pattern_matching_with_parameter() {
	let pattern = UrlPattern::new("test", "/users/<id>/", None);

	assert!(pattern.matches("/users/123/"));
	assert!(pattern.matches("/users/abc/"));
	assert!(!pattern.matches("/users/"));
	assert!(!pattern.matches("/users/123/edit/"));
}

#[test]
fn test_url_pattern_matching_multiple_parameters() {
	let pattern = UrlPattern::new("test", "/articles/<year>/<month>/<slug>/", None);

	assert!(pattern.matches("/articles/2025/01/test/"));
	assert!(pattern.matches("/articles/2024/12/year-end/"));
	assert!(!pattern.matches("/articles/2025/01/"));
	assert!(!pattern.matches("/articles/2025/"));
}

#[test]
fn test_url_namespace_prefix() {
	let ns = UrlNamespace::new("admin", "/admin/");

	assert_eq!(ns.name(), "admin");
	assert_eq!(ns.prefix(), "/admin/");
}

#[test]
fn test_url_namespace_nested() {
	let ns = UrlNamespace::new("api:v1", "/api/v1/");

	assert_eq!(ns.name(), "api:v1");
	assert_eq!(ns.prefix(), "/api/v1/");
}

#[test]
fn test_lazy_url_clone() {
	let resolver = Arc::new(setup_basic_resolver());

	let lazy_url1 = LazyUrl::new("home", resolver.clone());
	let lazy_url2 = lazy_url1.clone();

	assert_eq!(lazy_url1.resolve(), lazy_url2.resolve());
}

#[test]
fn test_lazy_url_multiple_resolvers() {
	let resolver1 = Arc::new(setup_basic_resolver());
	let resolver2 = Arc::new(setup_parameterized_resolver());

	let url1 = LazyUrl::new("home", resolver1);
	assert_eq!(url1.resolve(), "/");

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "42".to_string());

	let url2 = LazyUrl::with_kwargs("user-detail", kwargs, resolver2);
	assert_eq!(url2.resolve(), "/users/42/");
}

#[test]
fn test_url_pattern_parameter_substitution() {
	let pattern = UrlPattern::new("test", "/items/<id>/details/", None);

	let mut kwargs = HashMap::new();
	kwargs.insert("id".to_string(), "456".to_string());

	let url = pattern.build_url(&kwargs).unwrap();
	assert_eq!(url, "/items/456/details/");
}

#[test]
fn test_url_pattern_multiple_parameter_substitution() {
	let pattern = UrlPattern::new("test", "/<category>/<year>/<slug>/", None);

	let mut kwargs = HashMap::new();
	kwargs.insert("category".to_string(), "tech".to_string());
	kwargs.insert("year".to_string(), "2025".to_string());
	kwargs.insert("slug".to_string(), "new-post".to_string());

	let url = pattern.build_url(&kwargs).unwrap();
	assert_eq!(url, "/tech/2025/new-post/");
}

#[test]
fn test_lazy_url_resolution_performance() {
	let resolver = Arc::new(setup_basic_resolver());

	// Create many lazy URLs
	let urls: Vec<LazyUrl> = (0..100)
		.map(|_| LazyUrl::new("home", resolver.clone()))
		.collect();

	// Resolve all
	for url in &urls {
		assert_eq!(url.resolve(), "/");
	}

	// All should be resolved
	for url in &urls {
		assert!(url.is_resolved());
	}
}

#[test]
fn test_url_resolver_pattern_count() {
	let resolver = setup_basic_resolver();
	assert_eq!(resolver.pattern_count(), 3);

	let resolver = setup_parameterized_resolver();
	assert_eq!(resolver.pattern_count(), 4);

	let resolver = setup_namespaced_resolver();
	assert!(resolver.pattern_count() >= 5);
}

#[test]
fn test_url_resolver_has_pattern() {
	let resolver = setup_basic_resolver();

	assert!(resolver.has_pattern("home"));
	assert!(resolver.has_pattern("about"));
	assert!(!resolver.has_pattern("nonexistent"));
}

#[test]
fn test_lazy_url_with_query_parameters() {
	let resolver = Arc::new(setup_basic_resolver());

	let lazy_url = LazyUrl::new("home", resolver.clone());
	let base_url = lazy_url.resolve();

	// Query parameters should be added separately
	let url_with_query = format!("{}?page=1&sort=desc", base_url);
	assert_eq!(url_with_query, "/?page=1&sort=desc");
}

#[test]
fn test_url_pattern_trailing_slash_consistency() {
	let pattern1 = UrlPattern::new("test1", "/path/", None);
	let pattern2 = UrlPattern::new("test2", "/path", None);

	assert!(pattern1.matches("/path/"));
	assert!(!pattern1.matches("/path"));

	assert!(pattern2.matches("/path"));
	assert!(!pattern2.matches("/path/"));
}

#[test]
fn test_lazy_url_empty_kwargs() {
	let resolver = Arc::new(setup_basic_resolver());

	let lazy_url = LazyUrl::with_kwargs("home", HashMap::new(), resolver.clone());
	assert_eq!(lazy_url.resolve(), "/");
}

#[test]
fn test_url_namespace_hierarchy() {
	let mut resolver = UrlResolver::new();

	// Parent namespace
	let admin_ns = UrlNamespace::new("admin", "/admin/");
	resolver.add_namespace(admin_ns);

	// Child namespace
	let users_ns = UrlNamespace::new("admin:users", "/admin/users/");
	resolver.add_namespace(users_ns);

	resolver.add_pattern(UrlPattern::new(
		"admin:users:list",
		"/admin/users/",
		Some("admin:users"),
	));

	let resolver = Arc::new(resolver);

	let url = LazyUrl::new("admin:users:list", resolver);
	assert_eq!(url.resolve(), "/admin/users/");
}

#[test]
fn test_lazy_url_concurrent_resolution() {
	use std::thread;

	let resolver = Arc::new(setup_basic_resolver());

	let handles: Vec<_> = (0..10)
		.map(|_| {
			let resolver_clone = resolver.clone();
			thread::spawn(move || {
				let lazy_url = LazyUrl::new("home", resolver_clone);
				lazy_url.resolve()
			})
		})
		.collect();

	for handle in handles {
		let url = handle.join().unwrap();
		assert_eq!(url, "/");
	}
}
