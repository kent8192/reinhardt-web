//! Integration tests for UnifiedRouter with hierarchical routing and namespace support

use async_trait::async_trait;
use hyper::Method;
use reinhardt_http::{Request, Response, Result};
use reinhardt_routers::UnifiedRouter;
use reinhardt_types::Handler;
use reinhardt_viewsets::{Action, ActionType, ViewSet};

// Mock ViewSet for testing
#[derive(Clone)]
struct UserViewSet;

#[async_trait]
impl ViewSet for UserViewSet {
	fn get_basename(&self) -> &str {
		"users"
	}

	async fn dispatch(&self, _req: Request, action: Action) -> Result<Response> {
		match action.action_type {
			ActionType::List => Ok(Response::ok().with_body(b"User list".to_vec())),
			ActionType::Retrieve => Ok(Response::ok().with_body(b"User detail".to_vec())),
			ActionType::Create => Ok(Response::ok().with_body(b"User created".to_vec())),
			ActionType::Update => Ok(Response::ok().with_body(b"User updated".to_vec())),
			ActionType::Destroy => Ok(Response::ok().with_body(b"User deleted".to_vec())),
			_ => Ok(Response::not_found()),
		}
	}
}

// Mock function handler
async fn health_handler(_req: Request) -> Result<Response> {
	Ok(Response::ok().with_body(b"OK".to_vec()))
}

// Mock view handler
#[derive(Clone)]
struct AboutView;

#[async_trait]
impl Handler for AboutView {
	async fn handle(&self, _req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"About page".to_vec()))
	}
}

#[tokio::test]
async fn test_unified_router_basic_structure() {
	let router = UnifiedRouter::new()
		.with_prefix("/api")
		.with_namespace("api");

	assert_eq!(router.prefix(), "/api");
	assert_eq!(router.namespace(), Some("api"));
}

#[tokio::test]
async fn test_unified_router_mount_child() {
	let child = UnifiedRouter::new().with_namespace("users");

	let router = UnifiedRouter::new()
		.with_prefix("/api")
		.with_namespace("api")
		.mount("/users/", child);

	assert_eq!(router.children_count(), 1);
}

#[tokio::test]
async fn test_unified_router_with_viewset() {
	let router = UnifiedRouter::new()
		.with_prefix("/api")
		.viewset("users", UserViewSet);

	// Check that routes are generated
	let routes = router.get_all_routes();
	assert!(!routes.is_empty());
}

#[tokio::test]
async fn test_unified_router_hierarchical_namespace() {
	let users = UnifiedRouter::new()
		.with_namespace("users")
		.viewset("users", UserViewSet);

	let mut api = UnifiedRouter::new()
		.with_namespace("v1")
		.with_prefix("/api/v1")
		.mount("/users/", users);

	// Register all routes
	api.register_all_routes();

	// Check namespace resolution
	assert_eq!(api.namespace(), Some("v1"));
}

#[tokio::test]
async fn test_unified_router_url_reversal() {
	let mut router = UnifiedRouter::new().with_namespace("api").function_named(
		"/health",
		Method::GET,
		"health",
		health_handler,
	);

	router.register_all_routes();

	// Reverse URL with namespace
	let url = router.reverse("api:health", &[]);
	assert!(url.is_some());
	assert_eq!(url.unwrap(), "/health");
}

#[tokio::test]
async fn test_unified_router_nested_namespace_reversal() {
	let users = UnifiedRouter::new().with_namespace("users").function_named(
		"/list",
		Method::GET,
		"list",
		health_handler,
	);

	let v1 = UnifiedRouter::new()
		.with_namespace("v1")
		.mount("/users/", users);

	let mut api = UnifiedRouter::new().with_namespace("api").mount("/v1/", v1);

	api.register_all_routes();

	// Reverse with full namespace chain
	let url = api.reverse("api:v1:users:list", &[]);
	assert!(url.is_some());
}

#[tokio::test]
async fn test_unified_router_multiple_children() {
	let users = UnifiedRouter::new()
		.with_namespace("users")
		.viewset("users", UserViewSet);

	let posts = UnifiedRouter::new().with_namespace("posts").function_named(
		"/list",
		Method::GET,
		"list",
		health_handler,
	);

	let router = UnifiedRouter::new()
		.with_prefix("/api")
		.mount("/users/", users)
		.mount("/posts/", posts);

	assert_eq!(router.children_count(), 2);

	let routes = router.get_all_routes();
	assert!(!routes.is_empty());
}

#[tokio::test]
async fn test_unified_router_mixed_api_styles() {
	let router = UnifiedRouter::new()
		.with_prefix("/api")
		.function("/health", Method::GET, health_handler)
		.viewset("users", UserViewSet)
		.view("/about", AboutView);

	let routes = router.get_all_routes();
	// Should have routes from function, ViewSet, and view
	assert!(routes.len() >= 3);
}

#[tokio::test]
async fn test_unified_router_deep_nesting() {
	let resource = UnifiedRouter::new()
		.with_namespace("resource")
		.function_named("/action", Method::POST, "action", health_handler);

	let v2 = UnifiedRouter::new()
		.with_namespace("v2")
		.mount("/resource/", resource);

	let v1 = UnifiedRouter::new().with_namespace("v1").mount("/v2/", v2);

	let mut api = UnifiedRouter::new()
		.with_namespace("api")
		.with_prefix("/api")
		.mount("/v1/", v1);

	api.register_all_routes();

	// Test deep namespace resolution
	let url = api.reverse("api:v1:v2:resource:action", &[]);
	assert!(url.is_some());
}

#[tokio::test]
async fn test_unified_router_get_all_routes() {
	let users = UnifiedRouter::new().with_namespace("users").function_named(
		"/export",
		Method::GET,
		"export",
		health_handler,
	);

	let router = UnifiedRouter::new()
		.with_prefix("/api")
		.with_namespace("api")
		.function_named("/health", Method::GET, "health", health_handler)
		.mount("/users/", users);

	let routes = router.get_all_routes();

	// Should have routes from both parent and child
	assert!(routes.len() >= 2);

	// Check namespace combination in routes
	let has_combined_namespace = routes
		.iter()
		.any(|(_, _, ns, _)| ns.as_ref().is_some_and(|n| n.contains(':')));
	assert!(has_combined_namespace);
}

#[tokio::test]
async fn test_unified_router_viewset_url_reversal() {
	let mut router = UnifiedRouter::new()
		.with_namespace("api")
		.with_prefix("/api")
		.viewset("users", UserViewSet);

	router.register_all_routes();

	// ViewSets should register standard action names
	let list_url = router.reverse("api:users-list", &[]);
	assert!(list_url.is_some());

	let detail_url = router.reverse("api:users-detail", &[("id", "123")]);
	assert!(detail_url.is_some());
}

#[tokio::test]
async fn test_unified_router_namespace_inheritance() {
	let child =
		UnifiedRouter::new().function_named("/action", Method::POST, "action", health_handler);

	let mut parent = UnifiedRouter::new()
		.with_namespace("parent")
		.mount("/child/", child);

	parent.register_all_routes();

	// Child route should inherit parent namespace
	let url = parent.reverse("parent:action", &[]);
	assert!(url.is_some());
}
