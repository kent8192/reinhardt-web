#![cfg(native)]

use bytes::Bytes;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use hyper::{Method, header};
use reinhardt_core::endpoint::EndpointInfo;
use reinhardt_http::{Handler, Request, Response, Result};
use reinhardt_pages::server_fn::{ServerFnError, ServerFnRouterExt, server_fn};
use reinhardt_urls::routers::ServerRouter;
use serde::{Deserialize, Serialize};
use std::hint::black_box;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoResponse {
	value: u32,
	label: String,
}

#[server_fn(endpoint = "/api/server_fn/bench_echo")]
async fn bench_echo(value: u32, label: String) -> std::result::Result<EchoResponse, ServerFnError> {
	Ok(EchoResponse { value, label })
}

struct PlainEndpoint;

impl EndpointInfo for PlainEndpoint {
	fn path() -> &'static str {
		"/api/plain"
	}

	fn method() -> Method {
		Method::GET
	}

	fn name() -> &'static str {
		"plain"
	}
}

#[async_trait::async_trait]
impl Handler for PlainEndpoint {
	async fn handle(&self, _req: Request) -> Result<Response> {
		Ok(Response::ok().with_body("ok"))
	}
}

struct PathParamEndpoint;

impl EndpointInfo for PathParamEndpoint {
	fn path() -> &'static str {
		"/api/items/{id}/"
	}

	fn method() -> Method {
		Method::GET
	}

	fn name() -> &'static str {
		"path_param"
	}
}

#[async_trait::async_trait]
impl Handler for PathParamEndpoint {
	async fn handle(&self, req: Request) -> Result<Response> {
		let value = req
			.path_params
			.get("id")
			.cloned()
			.unwrap_or_else(|| "missing".to_string());
		Ok(Response::ok().with_body(value))
	}
}

fn build_get_request(path: &str) -> Request {
	Request::builder()
		.method(Method::GET)
		.uri(path)
		.build()
		.expect("GET request should build")
}

fn build_server_fn_request() -> Request {
	Request::builder()
		.method(Method::POST)
		.uri("/api/server_fn/bench_echo")
		.header(header::CONTENT_TYPE, "application/json")
		.body(Bytes::from_static(br#"{"value":42,"label":"bench"}"#))
		.build()
		.expect("server_fn request should build")
}

fn warm_router(router: &ServerRouter, request: Request) {
	let rt = tokio::runtime::Runtime::new().expect("tokio runtime should build");
	let response = rt
		.block_on(router.handle(request))
		.expect("warmup request should succeed");
	assert!(response.status.is_success());
}

fn bench_http_endpoint(c: &mut Criterion) {
	let rt = tokio::runtime::Runtime::new().expect("tokio runtime should build");
	let router = ServerRouter::new().endpoint(|| PlainEndpoint);
	warm_router(&router, build_get_request("/api/plain"));

	c.bench_function("http_endpoint_plain_get", |b| {
		b.iter_batched(
			|| build_get_request("/api/plain"),
			|request| {
				let response = rt
					.block_on(router.handle(request))
					.expect("plain endpoint request should succeed");
				black_box(response)
			},
			BatchSize::SmallInput,
		)
	});
}

fn bench_http_path_param_endpoint(c: &mut Criterion) {
	let rt = tokio::runtime::Runtime::new().expect("tokio runtime should build");
	let router = ServerRouter::new().endpoint(|| PathParamEndpoint);
	warm_router(&router, build_get_request("/api/items/42/"));

	c.bench_function("http_endpoint_path_param_get", |b| {
		b.iter_batched(
			|| build_get_request("/api/items/42/"),
			|request| {
				let response = rt
					.block_on(router.handle(request))
					.expect("path endpoint request should succeed");
				black_box(response)
			},
			BatchSize::SmallInput,
		)
	});
}

fn bench_server_fn_endpoint(c: &mut Criterion) {
	let rt = tokio::runtime::Runtime::new().expect("tokio runtime should build");
	let router = ServerRouter::new().server_fn(bench_echo::marker);
	warm_router(&router, build_server_fn_request());

	c.bench_function("server_fn_json_post", |b| {
		b.iter_batched(
			build_server_fn_request,
			|request| {
				let response = rt
					.block_on(router.handle(request))
					.expect("server_fn request should succeed");
				black_box(response)
			},
			BatchSize::SmallInput,
		)
	});
}

criterion_group!(
	benches,
	bench_http_endpoint,
	bench_http_path_param_endpoint,
	bench_server_fn_endpoint
);
criterion_main!(benches);
