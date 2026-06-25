use actix_web::{App, HttpResponse, HttpServer as ActixHttpServer, web};
use async_trait::async_trait;
use axum::{
	Json as AxumJson, Router as AxumRouter,
	extract::{Path as AxumPath, Query as AxumQuery},
	routing::{get as axum_get, post as axum_post},
};
use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
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
use reinhardt_server::server::HttpServer as ReinhardtHttpServer;
use reinhardt_urls::routers::ServerRouter;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
	hint::black_box,
	net::{SocketAddr, TcpListener as StdTcpListener},
	sync::Arc,
	time::Duration,
};
use tokio::{
	net::TcpListener as TokioTcpListener,
	sync::oneshot,
	task::JoinHandle,
	time::{sleep, timeout},
};

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

struct LoopbackServer {
	name: &'static str,
	base_url: String,
	shutdown_tx: Option<oneshot::Sender<()>>,
	actix_handle: Option<actix_web::dev::ServerHandle>,
	join_handle: Option<JoinHandle<()>>,
}

impl LoopbackServer {
	fn new(
		name: &'static str,
		addr: SocketAddr,
		shutdown_tx: Option<oneshot::Sender<()>>,
		actix_handle: Option<actix_web::dev::ServerHandle>,
		join_handle: JoinHandle<()>,
	) -> Self {
		Self {
			name,
			base_url: format!("http://{addr}"),
			shutdown_tx,
			actix_handle,
			join_handle: Some(join_handle),
		}
	}

	fn url(&self, path: &str) -> String {
		format!("{}{}", self.base_url, path)
	}

	fn target_urls(&self) -> TargetUrls {
		TargetUrls {
			name: self.name,
			hello: self.url("/hello"),
			echo: self.url("/echo"),
			path: self.url("/items/42/widget"),
			query: self.url("/search?q=bench&page=3"),
		}
	}

	async fn shutdown(mut self) {
		if let Some(handle) = self.actix_handle.take() {
			handle.stop(false).await;
		}
		if let Some(tx) = self.shutdown_tx.take() {
			let _ = tx.send(());
		}
		if let Some(mut join_handle) = self.join_handle.take() {
			tokio::select! {
				result = &mut join_handle => {
					if let Err(err) = result
						&& !err.is_cancelled()
					{
						eprintln!("{} benchmark server task failed: {err}", self.name);
					}
				}
				_ = sleep(Duration::from_millis(250)) => {
					join_handle.abort();
					let _ = join_handle.await;
				}
			}
		}
	}
}

struct TargetUrls {
	name: &'static str,
	hello: String,
	echo: String,
	path: String,
	query: String,
}

struct BenchServers {
	reinhardt: LoopbackServer,
	axum: LoopbackServer,
	actix: LoopbackServer,
	loco: LoopbackServer,
}

impl BenchServers {
	fn target_urls(&self) -> Vec<TargetUrls> {
		vec![
			self.reinhardt.target_urls(),
			self.axum.target_urls(),
			self.actix.target_urls(),
			self.loco.target_urls(),
		]
	}

	async fn shutdown(self) {
		self.reinhardt.shutdown().await;
		self.axum.shutdown().await;
		self.actix.shutdown().await;
		self.loco.shutdown().await;
	}
}

async fn spawn_reinhardt_server(client: &Client) -> LoopbackServer {
	let listener = TokioTcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
		.await
		.expect("Reinhardt listener should bind");
	let addr = listener
		.local_addr()
		.expect("Reinhardt listener should expose local address");
	let handler: Arc<dyn ReinhardtHandler> = Arc::new(reinhardt_router());
	let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

	let join_handle = tokio::spawn(async move {
		loop {
			tokio::select! {
				_ = &mut shutdown_rx => break,
				result = listener.accept() => {
					match result {
						Ok((stream, socket_addr)) => {
							let handler = handler.clone();
							tokio::spawn(async move {
								if let Err(err) =
									ReinhardtHttpServer::handle_connection(stream, socket_addr, handler, None).await
								{
									eprintln!("Reinhardt benchmark connection failed: {err}");
								}
							});
						}
						Err(err) => {
							eprintln!("Reinhardt benchmark accept failed: {err}");
							break;
						}
					}
				}
			}
		}
	});

	let server = LoopbackServer::new("reinhardt", addr, Some(shutdown_tx), None, join_handle);
	wait_until_ready(client, &server).await;
	server
}

async fn spawn_axum_server(client: &Client) -> LoopbackServer {
	spawn_axum_router_server("axum", axum_router(), client).await
}

async fn spawn_loco_server(client: &Client) -> LoopbackServer {
	spawn_axum_router_server("loco", loco_router().await, client).await
}

async fn spawn_axum_router_server(
	name: &'static str,
	router: AxumRouter,
	client: &Client,
) -> LoopbackServer {
	let listener = TokioTcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
		.await
		.expect("Axum-compatible listener should bind");
	let addr = listener
		.local_addr()
		.expect("Axum-compatible listener should expose local address");
	let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

	let join_handle = tokio::spawn(async move {
		let server = axum::serve(listener, router).with_graceful_shutdown(async {
			let _ = shutdown_rx.await;
		});
		if let Err(err) = server.await {
			eprintln!("{name} benchmark server failed: {err}");
		}
	});

	let server = LoopbackServer::new(name, addr, Some(shutdown_tx), None, join_handle);
	wait_until_ready(client, &server).await;
	server
}

async fn spawn_actix_server(client: &Client) -> LoopbackServer {
	let listener = StdTcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
		.expect("Actix listener should bind");
	let addr = listener
		.local_addr()
		.expect("Actix listener should expose local address");
	let server = ActixHttpServer::new(|| {
		App::new()
			.route("/hello", web::get().to(actix_hello))
			.route("/echo", web::post().to(actix_echo))
			.route("/items/{id}/{slug}", web::get().to(actix_path))
			.route("/search", web::get().to(actix_query))
	})
	.workers(1)
	.listen(listener)
	.expect("Actix server should listen")
	.run();
	let actix_handle = server.handle();
	let join_handle = tokio::spawn(async move {
		if let Err(err) = server.await {
			eprintln!("Actix benchmark server failed: {err}");
		}
	});

	let server = LoopbackServer::new("actix-web", addr, None, Some(actix_handle), join_handle);
	wait_until_ready(client, &server).await;
	server
}

async fn spawn_bench_servers(client: &Client) -> BenchServers {
	BenchServers {
		reinhardt: spawn_reinhardt_server(client).await,
		axum: spawn_axum_server(client).await,
		actix: spawn_actix_server(client).await,
		loco: spawn_loco_server(client).await,
	}
}

async fn wait_until_ready(client: &Client, server: &LoopbackServer) {
	let hello_url = server.url("/hello");
	let result = timeout(Duration::from_secs(5), async {
		loop {
			if let Ok(response) = client.get(&hello_url).send().await
				&& response.status() == StatusCode::OK
			{
				let _ = response.bytes().await;
				break;
			}
			sleep(Duration::from_millis(10)).await;
		}
	})
	.await;
	assert!(
		result.is_ok(),
		"{} benchmark server did not become ready",
		server.name
	);
}

async fn http_get(client: &Client, url: &str) -> Bytes {
	read_success_response(
		client
			.get(url)
			.send()
			.await
			.expect("GET request should succeed"),
	)
	.await
}

async fn http_json_post(client: &Client, url: &str) -> Bytes {
	read_success_response(
		client
			.post(url)
			.header(reqwest::header::CONTENT_TYPE, "application/json")
			.body(Bytes::from_static(JSON_BODY))
			.send()
			.await
			.expect("JSON POST request should succeed"),
	)
	.await
}

async fn read_success_response(response: reqwest::Response) -> Bytes {
	assert_eq!(response.status(), StatusCode::OK);
	response
		.bytes()
		.await
		.expect("response body should be readable")
}

fn bench_http_target(
	group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
	rt: &tokio::runtime::Runtime,
	client: &Client,
	target: &TargetUrls,
) {
	group.bench_function(BenchmarkId::new("hello_world", target.name), |b| {
		b.iter(|| black_box(rt.block_on(http_get(client, &target.hello))))
	});
	group.bench_function(BenchmarkId::new("json_echo", target.name), |b| {
		b.iter(|| black_box(rt.block_on(http_json_post(client, &target.echo))))
	});
	group.bench_function(BenchmarkId::new("path_params", target.name), |b| {
		b.iter(|| black_box(rt.block_on(http_get(client, &target.path))))
	});
	group.bench_function(BenchmarkId::new("query_params", target.name), |b| {
		b.iter(|| black_box(rt.block_on(http_get(client, &target.query))))
	});
}

fn runtime_http_benchmarks(c: &mut Criterion) {
	let rt = tokio::runtime::Runtime::new().expect("Tokio runtime should build");
	let client = Client::builder()
		.pool_max_idle_per_host(8)
		.build()
		.expect("HTTP client should build");
	let servers = rt.block_on(spawn_bench_servers(&client));
	let targets = servers.target_urls();

	let mut group = c.benchmark_group("runtime_http_loopback");
	group.sample_size(10);
	group.warm_up_time(Duration::from_millis(200));
	group.measurement_time(Duration::from_secs(1));

	for target in &targets {
		bench_http_target(&mut group, &rt, &client, target);
	}

	group.finish();
	drop(client);
	rt.block_on(servers.shutdown());
}

criterion_group!(benches, runtime_http_benchmarks);
criterion_main!(benches);
