//! Tests for Route Group Middleware functionality

use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use reinhardt_routers::{RouteGroup, UnifiedRouter};
use std::sync::Arc;

/// Middleware for testing
struct TestMiddleware {
	prefix: String,
}

#[async_trait]
impl Middleware for TestMiddleware {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		let mut response = next.handle(request).await?;
		let body = String::from_utf8_lossy(response.body()).to_string();
		let modified = format!("{}{}", self.prefix, body);
		response.set_body(modified.as_bytes());
		Ok(response)
	}
}

#[tokio::test]
async fn test_route_group_basic() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let group = RouteGroup::new()
		.with_prefix("/api")
		.function("/test", Method::GET, handler);

	let router = group.build();

	let req = Request::builder()
		.method(Method::GET)
		.uri("/api/test")
		.build();

	let response = router.handle(req).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response.body()), "test");
}

#[tokio::test]
async fn test_route_group_with_middleware() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let middleware = Arc::new(TestMiddleware {
		prefix: "group:".to_string(),
	});

	let group = RouteGroup::new()
		.with_prefix("/api")
		.with_middleware(middleware)
		.function("/test", Method::GET, handler);

	let router = group.build();

	let req = Request::builder()
		.method(Method::GET)
		.uri("/api/test")
		.build();

	let response = router.handle(req).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response.body()), "group:test");
}

#[tokio::test]
async fn test_route_group_nested() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let parent_mw = Arc::new(TestMiddleware {
		prefix: "parent:".to_string(),
	});
	let child_mw = Arc::new(TestMiddleware {
		prefix: "child:".to_string(),
	});

	let child_group = RouteGroup::new()
		.with_prefix("/v1")
		.with_middleware(child_mw)
		.function("/test", Method::GET, handler);

	let parent_group = RouteGroup::new()
		.with_prefix("/api")
		.with_middleware(parent_mw)
		.nest(child_group);

	let router = parent_group.build();

	let req = Request::builder()
		.method(Method::GET)
		.uri("/api/v1/test")
		.build();

	let response = router.handle(req).await.unwrap();
	// Parent middleware is applied first
	assert_eq!(
		String::from_utf8_lossy(response.body()),
		"parent:child:test"
	);
}

#[tokio::test]
async fn test_route_group_with_namespace() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let group = RouteGroup::new()
		.with_prefix("/api")
		.with_namespace("v1")
		.function_named("/test", Method::GET, "test", handler);

	let mut router = group.build();
	router.register_all_routes();

	// Reverse URL lookup using namespace
	let url = router.reverse("v1:test", &[]).unwrap();
	assert_eq!(url, "/test");
}

#[tokio::test]
async fn test_route_group_multiple_routes() {
	async fn handler1(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"route1"))
	}

	async fn handler2(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"route2"))
	}

	let middleware = Arc::new(TestMiddleware {
		prefix: "group:".to_string(),
	});

	let group = RouteGroup::new()
		.with_prefix("/api")
		.with_middleware(middleware)
		.function("/route1", Method::GET, handler1)
		.function("/route2", Method::GET, handler2);

	let router = group.build();

	// Test route1
	let req1 = Request::builder()
		.method(Method::GET)
		.uri("/api/route1")
		.build();
	let response1 = router.handle(req1).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response1.body()), "group:route1");

	// Test route2
	let req2 = Request::builder()
		.method(Method::GET)
		.uri("/api/route2")
		.build();
	let response2 = router.handle(req2).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response2.body()), "group:route2");
}

#[tokio::test]
async fn test_route_group_multiple_middleware() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let mw1 = Arc::new(TestMiddleware {
		prefix: "mw1:".to_string(),
	});
	let mw2 = Arc::new(TestMiddleware {
		prefix: "mw2:".to_string(),
	});

	let group = RouteGroup::new()
		.with_prefix("/api")
		.with_middleware(mw1)
		.with_middleware(mw2)
		.function("/test", Method::GET, handler);

	let router = group.build();

	let req = Request::builder()
		.method(Method::GET)
		.uri("/api/test")
		.build();

	let response = router.handle(req).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response.body()), "mw1:mw2:test");
}

#[tokio::test]
async fn test_route_group_deeply_nested() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let mw1 = Arc::new(TestMiddleware {
		prefix: "l1:".to_string(),
	});
	let mw2 = Arc::new(TestMiddleware {
		prefix: "l2:".to_string(),
	});
	let mw3 = Arc::new(TestMiddleware {
		prefix: "l3:".to_string(),
	});

	let level3 = RouteGroup::new()
		.with_prefix("/resource")
		.with_middleware(mw3)
		.function("/test", Method::GET, handler);

	let level2 = RouteGroup::new()
		.with_prefix("/v1")
		.with_middleware(mw2)
		.nest(level3);

	let level1 = RouteGroup::new()
		.with_prefix("/api")
		.with_middleware(mw1)
		.nest(level2);

	let router = level1.build();

	let req = Request::builder()
		.method(Method::GET)
		.uri("/api/v1/resource/test")
		.build();

	let response = router.handle(req).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response.body()), "l1:l2:l3:test");
}
