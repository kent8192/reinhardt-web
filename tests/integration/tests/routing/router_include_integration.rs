//! Router include() API integration tests (Django-style alias for mount())

use reinhardt_routers::UnifiedRouter;

#[tokio::test]
async fn test_router_include_basic() {
	// Create child router
	let users_router = UnifiedRouter::new().with_namespace("users");

	// Use include() to mount child router
	let router = UnifiedRouter::new()
		.with_prefix("/api")
		.include("/users/", users_router);

	// Verify router structure using public API
	assert_eq!(router.prefix(), "/api");
	assert_eq!(router.children_count(), 1);
}

#[tokio::test]
async fn test_router_include_mut() {
	// Create parent router
	let mut router = UnifiedRouter::new().with_prefix("/api");

	// Create child routers
	let users_router = UnifiedRouter::new().with_namespace("users");
	let posts_router = UnifiedRouter::new().with_namespace("posts");

	// Use include_mut() to mount child routers
	router.include_mut("/users/", users_router);
	router.include_mut("/posts/", posts_router);

	// Verify router structure using public API
	assert_eq!(router.prefix(), "/api");
	assert_eq!(router.children_count(), 2);
}

#[tokio::test]
async fn test_router_include_vs_mount_equivalence() {
	// Create routers using mount()
	let users_router1 = UnifiedRouter::new().with_namespace("users");
	let router1 = UnifiedRouter::new()
		.with_prefix("/api")
		.mount("/users/", users_router1);

	// Create equivalent routers using include()
	let users_router2 = UnifiedRouter::new().with_namespace("users");
	let router2 = UnifiedRouter::new()
		.with_prefix("/api")
		.include("/users/", users_router2);

	// Verify both routers have identical structure using public API
	assert_eq!(router1.prefix(), router2.prefix());
	assert_eq!(router1.children_count(), router2.children_count());
}

#[tokio::test]
async fn test_router_include_nested() {
	// Create deeply nested router structure
	let detail_router = UnifiedRouter::new().with_namespace("user_detail");

	let users_router = UnifiedRouter::new()
		.with_namespace("users")
		.include("/{id}/", detail_router);

	let api_router = UnifiedRouter::new()
		.with_prefix("/api")
		.include("/users/", users_router);

	// Verify nested structure using public API
	assert_eq!(api_router.prefix(), "/api");
	assert_eq!(api_router.children_count(), 1);
}
