//! Tests for Per-Route Middleware functionality

use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use reinhardt_routers::UnifiedRouter;
use std::sync::Arc;

/// Handler for testing
struct TestHandler {
	message: String,
}

#[async_trait]
impl Handler for TestHandler {
	async fn handle(&self, _req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(self.message.as_bytes()))
	}
}

/// Middleware for testing
struct TestMiddleware {
	prefix: String,
}

#[async_trait]
impl Middleware for TestMiddleware {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		// Process request
		let mut response = next.handle(request).await?;

		// Modify response
		let body = String::from_utf8_lossy(response.body()).to_string();
		let modified = format!("{}{}", self.prefix, body);
		response.set_body(modified.as_bytes());

		Ok(response)
	}
}

#[tokio::test]
async fn test_route_middleware() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let middleware = Arc::new(TestMiddleware {
		prefix: "middleware:".to_string(),
	});

	let router = UnifiedRouter::new()
		.function("/test", Method::GET, handler)
		.with_route_middleware(middleware);

	let req = Request::builder().method(Method::GET).uri("/test").build();

	let response = router.handle(req).await.unwrap();
	let body = String::from_utf8_lossy(response.body());

	assert_eq!(body, "middleware:test");
}

#[tokio::test]
async fn test_multiple_route_middleware() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let mw1 = Arc::new(TestMiddleware {
		prefix: "mw1:".to_string(),
	});
	let mw2 = Arc::new(TestMiddleware {
		prefix: "mw2:".to_string(),
	});

	let router = UnifiedRouter::new()
		.function("/test", Method::GET, handler)
		.with_route_middleware(mw1)
		.with_route_middleware(mw2);

	let req = Request::builder().method(Method::GET).uri("/test").build();

	let response = router.handle(req).await.unwrap();
	let body = String::from_utf8_lossy(response.body());

	// Middleware is applied in order: mw1 -> mw2
	assert_eq!(body, "mw1:mw2:test");
}

#[tokio::test]
async fn test_router_and_route_middleware() {
	async fn handler(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"test"))
	}

	let router_mw = Arc::new(TestMiddleware {
		prefix: "router:".to_string(),
	});
	let route_mw = Arc::new(TestMiddleware {
		prefix: "route:".to_string(),
	});

	let router = UnifiedRouter::new()
		.with_middleware(router_mw)
		.function("/test", Method::GET, handler)
		.with_route_middleware(route_mw);

	let req = Request::builder().method(Method::GET).uri("/test").build();

	let response = router.handle(req).await.unwrap();
	let body = String::from_utf8_lossy(response.body());

	// Router-level middleware is applied first
	assert_eq!(body, "router:route:test");
}

#[tokio::test]
async fn test_different_routes_different_middleware() {
	async fn handler1(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"route1"))
	}

	async fn handler2(_req: Request) -> Result<Response> {
		Ok(Response::ok().with_body(b"route2"))
	}

	let mw1 = Arc::new(TestMiddleware {
		prefix: "mw1:".to_string(),
	});
	let mw2 = Arc::new(TestMiddleware {
		prefix: "mw2:".to_string(),
	});

	let router = UnifiedRouter::new()
		.function("/route1", Method::GET, handler1)
		.with_route_middleware(mw1)
		.function("/route2", Method::GET, handler2)
		.with_route_middleware(mw2);

	// Test route1
	let req1 = Request::builder()
		.method(Method::GET)
		.uri("/route1")
		.build();
	let response1 = router.handle(req1).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response1.body()), "mw1:route1");

	// Test route2
	let req2 = Request::builder()
		.method(Method::GET)
		.uri("/route2")
		.build();
	let response2 = router.handle(req2).await.unwrap();
	assert_eq!(String::from_utf8_lossy(response2.body()), "mw2:route2");
}
