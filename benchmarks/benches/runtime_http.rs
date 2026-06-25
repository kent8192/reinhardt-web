use actix_web::{App, HttpResponse, test, web};
use async_trait::async_trait;
use axum::{
	Json as AxumJson, Router as AxumRouter,
	body::{Body, Bytes as AxumBytes, to_bytes},
	extract::{Path as AxumPath, Query as AxumQuery},
	http::{Method as AxumMethod, Request as AxumRequest, StatusCode as AxumStatusCode, header},
	routing::{get as axum_get, post as axum_post},
};
use bytes::Bytes;
use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use hyper::Method;
use loco_rs::{
	Result as LocoResult,
	prelude::{
		Json as LocoJson, Path as LocoPath, Query as LocoQuery, Response as LocoResponse,
		Routes as LocoRoutes, format, get as loco_get, post as loco_post,
	},
};
use reinhardt_core::endpoint::EndpointInfo;
use reinhardt_http::{
	Handler as ReinhardtHandler, Request as ReinhardtRequest, Response as ReinhardtResponse,
	Result as ReinhardtResult,
};
use reinhardt_urls::routers::ServerRouter;
use serde::{Deserialize, Serialize};
use std::{hint::black_box, time::Duration};
use tower::ServiceExt;

const JSON_BODY: &[u8] = br#"{"id":42,"message":"benchmark"}"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EchoPayload {
	id: u64,
	message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PathPayload {
	id: u64,
	slug: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SearchQuery {
	q: String,
	page: u32,
}

struct ReinhardtHello;

impl EndpointInfo for ReinhardtHello {
	fn path() -> &'static str {
		"/hello"
	}

	fn method() -> Method {
		Method::GET
	}

	fn name() -> &'static str {
		"runtime_hello_world"
	}
}

#[async_trait]
impl ReinhardtHandler for ReinhardtHello {
	async fn handle(&self, _req: ReinhardtRequest) -> ReinhardtResult<ReinhardtResponse> {
		Ok(ReinhardtResponse::ok().with_body("hello"))
	}
}

struct ReinhardtEcho;

impl EndpointInfo for ReinhardtEcho {
	fn path() -> &'static str {
		"/echo"
	}

	fn method() -> Method {
		Method::POST
	}

	fn name() -> &'static str {
		"runtime_json_echo"
	}
}

#[async_trait]
impl ReinhardtHandler for ReinhardtEcho {
	async fn handle(&self, req: ReinhardtRequest) -> ReinhardtResult<ReinhardtResponse> {
		let payload: EchoPayload = req.json()?;
		ReinhardtResponse::ok().with_json(&payload)
	}
}

struct ReinhardtPath;

impl EndpointInfo for ReinhardtPath {
	fn path() -> &'static str {
		"/items/{id}/{slug}"
	}

	fn method() -> Method {
		Method::GET
	}

	fn name() -> &'static str {
		"runtime_path_params"
	}
}

#[async_trait]
impl ReinhardtHandler for ReinhardtPath {
	async fn handle(&self, req: ReinhardtRequest) -> ReinhardtResult<ReinhardtResponse> {
		let id = req
			.path_params
			.get("id")
			.and_then(|value| value.parse::<u64>().ok())
			.expect("id path parameter should parse");
		let slug = req
			.path_params
			.get("slug")
			.cloned()
			.expect("slug path parameter should exist");
		ReinhardtResponse::ok().with_json(&PathPayload { id, slug })
	}
}

struct ReinhardtQuery;

impl EndpointInfo for ReinhardtQuery {
	fn path() -> &'static str {
		"/search"
	}

	fn method() -> Method {
		Method::GET
	}

	fn name() -> &'static str {
		"runtime_query_params"
	}
}

#[async_trait]
impl ReinhardtHandler for ReinhardtQuery {
	async fn handle(&self, req: ReinhardtRequest) -> ReinhardtResult<ReinhardtResponse> {
		let q = req
			.query_params
			.get("q")
			.cloned()
			.expect("q query parameter should exist");
		let page = req
			.query_params
			.get("page")
			.and_then(|value| value.parse::<u32>().ok())
			.expect("page query parameter should parse");
		ReinhardtResponse::ok().with_json(&SearchQuery { q, page })
	}
}

fn reinhardt_router() -> ServerRouter {
	ServerRouter::new()
		.endpoint(|| ReinhardtHello)
		.endpoint(|| ReinhardtEcho)
		.endpoint(|| ReinhardtPath)
		.endpoint(|| ReinhardtQuery)
}

fn reinhardt_get(uri: &str) -> ReinhardtRequest {
	ReinhardtRequest::builder()
		.method(Method::GET)
		.uri(uri)
		.build()
		.expect("GET request should build")
}

fn reinhardt_json_post(uri: &str) -> ReinhardtRequest {
	ReinhardtRequest::builder()
		.method(Method::POST)
		.uri(uri)
		.header(hyper::header::CONTENT_TYPE, "application/json")
		.body(Bytes::from_static(JSON_BODY))
		.build()
		.expect("JSON request should build")
}

async fn reinhardt_call(router: &ServerRouter, request: ReinhardtRequest) -> Bytes {
	let response = router
		.handle(request)
		.await
		.expect("Reinhardt request should succeed");
	assert!(
		response.status.is_success(),
		"Reinhardt returned {}",
		response.status
	);
	response.body
}

async fn axum_hello() -> &'static str {
	"hello"
}

async fn axum_echo(AxumJson(payload): AxumJson<EchoPayload>) -> AxumJson<EchoPayload> {
	AxumJson(payload)
}

async fn axum_path(AxumPath((id, slug)): AxumPath<(u64, String)>) -> AxumJson<PathPayload> {
	AxumJson(PathPayload { id, slug })
}

async fn axum_query(AxumQuery(query): AxumQuery<SearchQuery>) -> AxumJson<SearchQuery> {
	AxumJson(query)
}

fn axum_router() -> AxumRouter {
	AxumRouter::new()
		.route("/hello", axum_get(axum_hello))
		.route("/echo", axum_post(axum_echo))
		.route("/items/{id}/{slug}", axum_get(axum_path))
		.route("/search", axum_get(axum_query))
}

fn axum_get_request(uri: &str) -> AxumRequest<Body> {
	AxumRequest::builder()
		.method(AxumMethod::GET)
		.uri(uri)
		.body(Body::empty())
		.expect("Axum GET request should build")
}

fn axum_json_post_request(uri: &str) -> AxumRequest<Body> {
	AxumRequest::builder()
		.method(AxumMethod::POST)
		.uri(uri)
		.header(header::CONTENT_TYPE, "application/json")
		.body(Body::from(JSON_BODY))
		.expect("Axum JSON request should build")
}

async fn axum_call(router: AxumRouter, request: AxumRequest<Body>) -> AxumBytes {
	let response = router
		.oneshot(request)
		.await
		.expect("Axum request should succeed");
	assert_eq!(response.status(), AxumStatusCode::OK);
	to_bytes(response.into_body(), usize::MAX)
		.await
		.expect("Axum response body should read")
}

async fn actix_hello() -> HttpResponse {
	HttpResponse::Ok().body("hello")
}

async fn actix_echo(payload: web::Json<EchoPayload>) -> HttpResponse {
	HttpResponse::Ok().json(payload.into_inner())
}

async fn actix_path(path: web::Path<(u64, String)>) -> HttpResponse {
	let (id, slug) = path.into_inner();
	HttpResponse::Ok().json(PathPayload { id, slug })
}

async fn actix_query(query: web::Query<SearchQuery>) -> HttpResponse {
	HttpResponse::Ok().json(query.into_inner())
}

async fn loco_hello() -> LocoResult<LocoResponse> {
	format::text("hello")
}

async fn loco_echo(LocoJson(payload): LocoJson<EchoPayload>) -> LocoResult<LocoResponse> {
	format::json(payload)
}

async fn loco_path(LocoPath((id, slug)): LocoPath<(u64, String)>) -> LocoResult<LocoResponse> {
	format::json(PathPayload { id, slug })
}

async fn loco_query(LocoQuery(query): LocoQuery<SearchQuery>) -> LocoResult<LocoResponse> {
	format::json(query)
}

async fn loco_router() -> AxumRouter {
	let routes = LocoRoutes::new()
		.add("/hello", loco_get(loco_hello))
		.add("/echo", loco_post(loco_echo))
		.add("/items/{id}/{slug}", loco_get(loco_path))
		.add("/search", loco_get(loco_query));
	let mut router = AxumRouter::new();
	for handler in routes.handlers {
		router = router.route(&handler.uri, handler.method);
	}
	router.with_state(loco_rs::tests_cfg::app::get_app_context().await)
}

fn bench_reinhardt(
	group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
	rt: &tokio::runtime::Runtime,
) {
	let router = reinhardt_router();

	group.bench_function(BenchmarkId::new("hello_world", "reinhardt"), |b| {
		b.iter_batched(
			|| reinhardt_get("/hello"),
			|request| black_box(rt.block_on(reinhardt_call(&router, request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("json_echo", "reinhardt"), |b| {
		b.iter_batched(
			|| reinhardt_json_post("/echo"),
			|request| black_box(rt.block_on(reinhardt_call(&router, request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("path_params", "reinhardt"), |b| {
		b.iter_batched(
			|| reinhardt_get("/items/42/widget"),
			|request| black_box(rt.block_on(reinhardt_call(&router, request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("query_params", "reinhardt"), |b| {
		b.iter_batched(
			|| reinhardt_get("/search?q=bench&page=3"),
			|request| black_box(rt.block_on(reinhardt_call(&router, request))),
			BatchSize::SmallInput,
		)
	});
}

fn bench_axum(
	group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
	rt: &tokio::runtime::Runtime,
) {
	let router = axum_router();

	group.bench_function(BenchmarkId::new("hello_world", "axum"), |b| {
		b.iter_batched(
			|| axum_get_request("/hello"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("json_echo", "axum"), |b| {
		b.iter_batched(
			|| axum_json_post_request("/echo"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("path_params", "axum"), |b| {
		b.iter_batched(
			|| axum_get_request("/items/42/widget"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("query_params", "axum"), |b| {
		b.iter_batched(
			|| axum_get_request("/search?q=bench&page=3"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
}

fn bench_actix(
	group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
	rt: &tokio::runtime::Runtime,
) {
	let app = rt.block_on(test::init_service(
		App::new()
			.route("/hello", web::get().to(actix_hello))
			.route("/echo", web::post().to(actix_echo))
			.route("/items/{id}/{slug}", web::get().to(actix_path))
			.route("/search", web::get().to(actix_query)),
	));

	group.bench_function(BenchmarkId::new("hello_world", "actix-web"), |b| {
		b.iter(|| {
			black_box(rt.block_on(async {
				let request = test::TestRequest::get().uri("/hello").to_request();
				let response = test::call_service(&app, request).await;
				assert!(response.status().is_success());
				test::read_body(response).await
			}))
		})
	});
	group.bench_function(BenchmarkId::new("json_echo", "actix-web"), |b| {
		b.iter(|| {
			black_box(rt.block_on(async {
				let request = test::TestRequest::post()
					.uri("/echo")
					.insert_header(("content-type", "application/json"))
					.set_payload(JSON_BODY)
					.to_request();
				let response = test::call_service(&app, request).await;
				assert!(response.status().is_success());
				test::read_body(response).await
			}))
		})
	});
	group.bench_function(BenchmarkId::new("path_params", "actix-web"), |b| {
		b.iter(|| {
			black_box(rt.block_on(async {
				let request = test::TestRequest::get()
					.uri("/items/42/widget")
					.to_request();
				let response = test::call_service(&app, request).await;
				assert!(response.status().is_success());
				test::read_body(response).await
			}))
		})
	});
	group.bench_function(BenchmarkId::new("query_params", "actix-web"), |b| {
		b.iter(|| {
			black_box(rt.block_on(async {
				let request = test::TestRequest::get()
					.uri("/search?q=bench&page=3")
					.to_request();
				let response = test::call_service(&app, request).await;
				assert!(response.status().is_success());
				test::read_body(response).await
			}))
		})
	});
}

fn bench_loco(
	group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
	rt: &tokio::runtime::Runtime,
) {
	let router = rt.block_on(loco_router());

	group.bench_function(BenchmarkId::new("hello_world", "loco"), |b| {
		b.iter_batched(
			|| axum_get_request("/hello"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("json_echo", "loco"), |b| {
		b.iter_batched(
			|| axum_json_post_request("/echo"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("path_params", "loco"), |b| {
		b.iter_batched(
			|| axum_get_request("/items/42/widget"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
	group.bench_function(BenchmarkId::new("query_params", "loco"), |b| {
		b.iter_batched(
			|| axum_get_request("/search?q=bench&page=3"),
			|request| black_box(rt.block_on(axum_call(router.clone(), request))),
			BatchSize::SmallInput,
		)
	});
}

fn runtime_http_benchmarks(c: &mut Criterion) {
	let rt = tokio::runtime::Runtime::new().expect("Tokio runtime should build");
	let mut group = c.benchmark_group("runtime_http");
	group.sample_size(10);
	group.warm_up_time(Duration::from_millis(200));
	group.measurement_time(Duration::from_secs(1));

	bench_reinhardt(&mut group, &rt);
	bench_axum(&mut group, &rt);
	bench_actix(&mut group, &rt);
	bench_loco(&mut group, &rt);

	group.finish();
}

criterion_group!(benches, runtime_http_benchmarks);
criterion_main!(benches);
