use async_trait::async_trait;
use bytes::Bytes;
use hyper::Method;
use reinhardt_http::{Handler, Request, Response, Result};
use reinhardt_middleware::Middleware;
use reinhardt_urls::routers::ServerRouter;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
		unsafe { System.alloc(layout) }
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		unsafe { System.dealloc(ptr, layout) }
	}
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn reset_allocs() {
	ALLOCATIONS.store(0, Ordering::Relaxed);
}

fn allocs() -> usize {
	ALLOCATIONS.load(Ordering::Relaxed)
}

struct EmptyHandler;

#[async_trait]
impl Handler for EmptyHandler {
	async fn handle(&self, _request: Request) -> Result<Response> {
		Ok(Response::ok())
	}
}

struct TinyMiddleware;

#[async_trait]
impl Middleware for TinyMiddleware {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		next.handle(request).await
	}
}

fn request(path: &str) -> Request {
	Request::builder()
		.method(Method::GET)
		.uri(path)
		.body(Bytes::new())
		.build()
		.unwrap()
}

async fn measure<F, Fut>(label: &str, iterations: usize, mut f: F)
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = ()>,
{
	for _ in 0..128 {
		f().await;
	}

	reset_allocs();
	for _ in 0..iterations {
		f().await;
	}
	let total = allocs();
	println!(
		"{label}: total={total}, per_req={:.2}",
		total as f64 / iterations as f64
	);
}

async fn measure_counted<F, Fut>(label: &str, iterations: usize, mut f: F)
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = usize>,
{
	for _ in 0..128 {
		let _ = f().await;
	}

	let mut total = 0usize;
	for _ in 0..iterations {
		total += f().await;
	}
	println!(
		"{label}: total={total}, per_req={:.2}",
		total as f64 / iterations as f64
	);
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
	let iterations = 10_000;

	measure("request_build_empty_path", iterations, || async {
		let _ = request("/health");
	})
	.await;

	measure("request_build_two_query_params", iterations, || async {
		let _ = request("/health?a=1&b=two");
	})
	.await;

	let handler: Arc<dyn Handler> = Arc::new(EmptyHandler);
	measure("direct_handler_build_plus_handle", iterations, || {
		let handler = handler.clone();
		async move {
			let _ = handler.handle(request("/health")).await.unwrap();
		}
	})
	.await;

	measure_counted("direct_handler_handle_only", iterations, || {
		let handler = handler.clone();
		async move {
			let req = request("/health");
			reset_allocs();
			let _ = handler.handle(req).await.unwrap();
			allocs()
		}
	})
	.await;

	measure_counted("clone_for_di_empty_path", iterations, || async {
		let req = request("/health");
		reset_allocs();
		let _ = req.clone_for_di();
		allocs()
	})
	.await;

	measure_counted("clone_for_di_two_query_params", iterations, || async {
		let req = request("/health?a=1&b=two");
		reset_allocs();
		let _ = req.clone_for_di();
		allocs()
	})
	.await;

	let router = Arc::new(ServerRouter::new().handler("/health", EmptyHandler));
	router.handle(request("/health")).await.unwrap();
	measure("server_router_static_build_plus_handle", iterations, || {
		let router = router.clone();
		async move {
			let _ = router.handle(request("/health")).await.unwrap();
		}
	})
	.await;

	let router_param =
		Arc::new(ServerRouter::new().handler("/users/{id}/posts/{post_id}/", EmptyHandler));
	router_param
		.handle(request("/users/123/posts/456/"))
		.await
		.unwrap();
	measure(
		"server_router_two_params_build_plus_handle",
		iterations,
		|| {
			let router = router_param.clone();
			async move {
				let _ = router
					.handle(request("/users/123/posts/456/"))
					.await
					.unwrap();
			}
		},
	)
	.await;

	let router_mw = Arc::new(
		ServerRouter::new()
			.with_middleware(TinyMiddleware)
			.handler("/health", EmptyHandler),
	);
	router_mw.handle(request("/health")).await.unwrap();
	measure(
		"server_router_one_middleware_build_plus_handle",
		iterations,
		|| {
			let router = router_mw.clone();
			async move {
				let _ = router.handle(request("/health")).await.unwrap();
			}
		},
	)
	.await;
}
