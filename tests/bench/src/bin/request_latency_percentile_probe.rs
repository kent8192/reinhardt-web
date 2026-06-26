use async_trait::async_trait;
use bytes::Bytes;
use hyper::Method;
use reinhardt_http::{Handler, Request, Response, Result};
use reinhardt_middleware::Middleware;
use reinhardt_urls::routers::ServerRouter;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

const ITERATIONS: usize = 50_000;
const WARMUP_ITERATIONS: usize = 1_000;

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

async fn sample<F, Fut>(label: &str, mut f: F)
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = ()>,
{
	for _ in 0..WARMUP_ITERATIONS {
		f().await;
	}

	let mut nanos = Vec::with_capacity(ITERATIONS);
	for _ in 0..ITERATIONS {
		let start = Instant::now();
		f().await;
		nanos.push(start.elapsed().as_nanos() as u64);
	}
	nanos.sort_unstable();
	let total = nanos.iter().map(|value| *value as u128).sum::<u128>();
	let mean = total as f64 / ITERATIONS as f64;
	println!(
		"{label}: samples={ITERATIONS}, mean_ns={mean:.2}, p50_ns={}, p95_ns={}, p99_ns={}, max_ns={}",
		percentile(&nanos, 0.50),
		percentile(&nanos, 0.95),
		percentile(&nanos, 0.99),
		nanos.last().copied().unwrap_or_default()
	);
}

fn percentile(sorted: &[u64], percentile: f64) -> u64 {
	let index = ((sorted.len() as f64 * percentile).ceil() as usize).saturating_sub(1);
	sorted[index.min(sorted.len().saturating_sub(1))]
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
	let handler: Arc<dyn Handler> = Arc::new(EmptyHandler);
	sample("direct_handler_build_plus_handle", || {
		let handler = handler.clone();
		async move {
			let response = handler.handle(request("/health")).await.unwrap();
			black_box(response);
		}
	})
	.await;

	let router = Arc::new(ServerRouter::new().handler("/health", EmptyHandler));
	router.handle(request("/health")).await.unwrap();
	sample("server_router_static_build_plus_handle", || {
		let router = router.clone();
		async move {
			let response = router.handle(request("/health")).await.unwrap();
			black_box(response);
		}
	})
	.await;

	let router_param =
		Arc::new(ServerRouter::new().handler("/users/{id}/posts/{post_id}/", EmptyHandler));
	router_param
		.handle(request("/users/123/posts/456/"))
		.await
		.unwrap();
	sample("server_router_two_params_build_plus_handle", || {
		let router = router_param.clone();
		async move {
			let response = router
				.handle(request("/users/123/posts/456/"))
				.await
				.unwrap();
			black_box(response);
		}
	})
	.await;

	let router_mw = Arc::new(
		ServerRouter::new()
			.with_middleware(TinyMiddleware)
			.handler("/health", EmptyHandler),
	);
	router_mw.handle(request("/health")).await.unwrap();
	sample("server_router_one_middleware_build_plus_handle", || {
		let router = router_mw.clone();
		async move {
			let response = router.handle(request("/health")).await.unwrap();
			black_box(response);
		}
	})
	.await;
}
