//! Middleware integration functionality tests
//!
//! Tests integration of global, group, and route-level middleware.

use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use reinhardt_routers::{RouteGroup, UnifiedRouter};
use std::sync::Arc;

/// Test middleware
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
async fn test_global_group_route_middleware() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let global_mw = Arc::new(TestMiddleware {
		prefix: "global:".to_string(),
	});
	let group_mw = Arc::new(TestMiddleware {
		prefix: "group:".to_string(),
	});
	let route_mw = Arc::new(TestMiddleware {
		prefix: "route:".to_string(),
	});

	let group = RouteGroup::new()
		.with_prefix("/api")
		.with_middleware(group_mw)
		.function("/test", Method::GET, handler)
		.with_route_middleware(route_mw);

	let router = UnifiedRouter::new()
		.with_middleware(global_mw)
		.mount("/api", group.build());

	let req = Request::builder()
		.method(Method::GET)
		.uri("/api/test")
		.build();

	let response = router.handle(req).await.unwrap();
	// Execution order: global -> group -> route -> handler
	assert_eq!(
		String::from_utf8_lossy(response.body()),
		"global:group:route:test"
	);
}

#[tokio::test]
async fn test_conditional_middleware_application() {
	async fn handler1(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"route1"))
	}

	async fn handler2(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"route2"))
	}

	let global_mw = Arc::new(TestMiddleware {
		prefix: "global:".to_string(),
	});
	let route1_mw = Arc::new(TestMiddleware {
		prefix: "r1:".to_string(),
	});

	let router = UnifiedRouter::new()
		.with_middleware(global_mw)
		.function("/route1", Method::GET, handler1)
		.with_route_middleware(route1_mw)
		.function("/route2", Method::GET, handler2); // route2 has no route middleware

	// route1 has global + route middleware
	let req1 = Request::builder()
		.method(Method::GET)
		.uri("/route1")
		.build();
	let response1 = router.handle(req1).await.unwrap();
	assert_eq!(
		String::from_utf8_lossy(response1.body()),
		"global:r1:route1"
	);

	// route2 has global middleware only
	let req2 = Request::builder()
		.method(Method::GET)
		.uri("/route2")
		.build();
	let response2 = router.handle(req2).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response2.body()), "global:route2");
}

#[tokio::test]
async fn test_middleware_execution_order() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let mw1 = Arc::new(TestMiddleware {
		prefix: "1:".to_string(),
	});
	let mw2 = Arc::new(TestMiddleware {
		prefix: "2:".to_string(),
	});
	let mw3 = Arc::new(TestMiddleware {
		prefix: "3:".to_string(),
	});

	let router = UnifiedRouter::new()
		.with_middleware(mw1.clone())
		.with_middleware(mw2.clone())
		.function("/test", Method::GET, handler)
		.with_route_middleware(mw3.clone());

	let req = Request::builder().method(Method::GET).uri("/test").build();

	let response = router.handle(req).await.unwrap();
	// Middleware is executed in registration order
	assert_eq!(String::from_utf8_lossy(response.body()), "1:2:3:test");
}

#[tokio::test]
async fn test_nested_groups_with_middleware() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let level1_mw = Arc::new(TestMiddleware {
		prefix: "l1:".to_string(),
	});
	let level2_mw = Arc::new(TestMiddleware {
		prefix: "l2:".to_string(),
	});
	let level3_mw = Arc::new(TestMiddleware {
		prefix: "l3:".to_string(),
	});
	let route_mw = Arc::new(TestMiddleware {
		prefix: "route:".to_string(),
	});

	let level3 = RouteGroup::new()
		.with_prefix("/v1")
		.with_middleware(level3_mw)
		.function("/test", Method::GET, handler)
		.with_route_middleware(route_mw);

	let level2 = RouteGroup::new()
		.with_prefix("/api")
		.with_middleware(level2_mw)
		.nest(level3);

	let level1 = UnifiedRouter::new()
		.with_prefix("/service")
		.with_middleware(level1_mw)
		.mount("/api", level2.build());

	let req = Request::builder()
		.method(Method::GET)
		.uri("/service/api/v1/test")
		.build();

	let response = level1.handle(req).await.unwrap();
	assert_eq!(
		String::from_utf8_lossy(response.body()),
		"l1:l2:l3:route:test"
	);
}

#[tokio::test]
async fn test_multiple_routes_different_middleware_stacks() {
	async fn handler1(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"1"))
	}

	async fn handler2(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"2"))
	}

	async fn handler3(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"3"))
	}

	let global_mw = Arc::new(TestMiddleware {
		prefix: "g:".to_string(),
	});
	let mw_a = Arc::new(TestMiddleware {
		prefix: "a:".to_string(),
	});
	let mw_b = Arc::new(TestMiddleware {
		prefix: "b:".to_string(),
	});

	let router = UnifiedRouter::new()
		.with_middleware(global_mw)
		.function("/route1", Method::GET, handler1)
		.with_route_middleware(mw_a.clone())
		.function("/route2", Method::GET, handler2)
		.with_route_middleware(mw_b.clone())
		.function("/route3", Method::GET, handler3); // No middleware

	// route1: global + a
	let req1 = Request::builder()
		.method(Method::GET)
		.uri("/route1")
		.build();
	let response1 = router.handle(req1).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response1.body()), "g:a:1");

	// route2: global + b
	let req2 = Request::builder()
		.method(Method::GET)
		.uri("/route2")
		.build();
	let response2 = router.handle(req2).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response2.body()), "g:b:2");

	// route3: global only
	let req3 = Request::builder()
		.method(Method::GET)
		.uri("/route3")
		.build();
	let response3 = router.handle(req3).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response3.body()), "g:3");
}

#[tokio::test]
async fn test_group_isolation() {
	async fn handler1(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"g1"))
	}

	async fn handler2(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"g2"))
	}

	let group1_mw = Arc::new(TestMiddleware {
		prefix: "g1:".to_string(),
	});
	let group2_mw = Arc::new(TestMiddleware {
		prefix: "g2:".to_string(),
	});

	let group1 = RouteGroup::new()
		.with_prefix("/group1")
		.with_middleware(group1_mw)
		.function("/test", Method::GET, handler1);

	let group2 = RouteGroup::new()
		.with_prefix("/group2")
		.with_middleware(group2_mw)
		.function("/test", Method::GET, handler2);

	let router = UnifiedRouter::new()
		.mount("/group1", group1.build())
		.mount("/group2", group2.build());

	// group1 middleware is only applied to group1 routes
	let req1 = Request::builder()
		.method(Method::GET)
		.uri("/group1/test")
		.build();
	let response1 = router.handle(req1).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response1.body()), "g1:g1");

	// group2 middleware is only applied to group2 routes
	let req2 = Request::builder()
		.method(Method::GET)
		.uri("/group2/test")
		.build();
	let response2 = router.handle(req2).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response2.body()), "g2:g2");
}
