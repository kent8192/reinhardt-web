#![cfg(all(
	not(all(target_family = "wasm", target_os = "unknown")),
	feature = "msw"
))]

use std::sync::Arc;
use std::time::Duration;

use reinhardt_test::msw::{MockResponse, MockServiceWorker, UnhandledPolicy, rest};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn endpoint(worker: &MockServiceWorker, path: &str) -> String {
	format!("{}{}", worker.url(), path)
}

#[tokio::test]
async fn native_worker_serves_registered_get_response() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::get("/api/test").respond(MockResponse::json(json!({"hello": "world"}))));
	worker.start().await;

	let response = reqwest::get(endpoint(&worker, "/api/test"))
		.await
		.expect("mock request should succeed");

	assert_eq!(response.status().as_u16(), 200);
	assert_eq!(
		response
			.headers()
			.get("content-type")
			.expect("json content type should be set"),
		"application/json"
	);
	assert_eq!(
		response.text().await.expect("response body should decode"),
		r#"{"hello":"world"}"#
	);
	worker.calls_to("/api/test").assert_count(1);
}

#[tokio::test]
async fn native_worker_passes_request_body_to_dynamic_handler() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::post("/api/echo").respond_with(|req| {
		MockResponse::text(req.body.clone().expect("request body should be recorded"))
	}));
	worker.start().await;

	let response = reqwest::Client::new()
		.post(endpoint(&worker, "/api/echo"))
		.body("native-body")
		.send()
		.await
		.expect("mock request should succeed");

	assert_eq!(response.status().as_u16(), 200);
	assert_eq!(
		response.text().await.expect("response body should decode"),
		"native-body"
	);
	let recorded = worker
		.calls_to("/api/echo")
		.last()
		.expect("call should be recorded");
	assert_eq!(recorded.method, "POST");
	assert_eq!(recorded.body.as_deref(), Some("native-body"));
}

#[tokio::test]
async fn native_worker_matches_parameterized_paths_and_query_strings() {
	let worker = MockServiceWorker::new();
	worker.handle(
		rest::get("/api/users/:id")
			.respond_with(|req| MockResponse::json(json!({ "url": req.url }))),
	);
	worker.start().await;

	let response = reqwest::get(endpoint(&worker, "/api/users/42?active=true"))
		.await
		.expect("mock request should succeed");

	assert_eq!(response.status().as_u16(), 200);
	let body = response.text().await.expect("response body should decode");
	assert_eq!(body, r#"{"url":"/api/users/42?active=true"}"#);
	worker.calls_to("/api/users/:id").assert_count(1);
}

#[tokio::test]
async fn native_worker_consumes_once_handlers() {
	let worker = MockServiceWorker::new();
	worker.handle(
		rest::get("/api/once")
			.once()
			.respond(MockResponse::text("first")),
	);
	worker.start().await;

	let first = reqwest::get(endpoint(&worker, "/api/once"))
		.await
		.expect("first mock request should succeed");
	assert_eq!(first.status().as_u16(), 200);
	assert_eq!(
		first.text().await.expect("first body should decode"),
		"first"
	);

	let second = reqwest::get(endpoint(&worker, "/api/once"))
		.await
		.expect("second mock request should receive diagnostic response");
	assert_eq!(second.status().as_u16(), 500);
	assert_eq!(
		second.text().await.expect("second body should decode"),
		"MSW: No handler for GET /api/once"
	);
	worker.calls_to("/api/once").assert_count(2);
}

#[tokio::test]
async fn native_worker_reset_clears_handlers_and_recorded_calls() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::get("/api/reset").respond(MockResponse::text("ok")));
	worker.start().await;

	let response = reqwest::get(endpoint(&worker, "/api/reset"))
		.await
		.expect("mock request should succeed");
	assert_eq!(response.status().as_u16(), 200);
	worker.calls_to("/api/reset").assert_count(1);

	worker.reset();

	assert_eq!(worker.all_calls().len(), 0);
	let after_reset = reqwest::get(endpoint(&worker, "/api/reset"))
		.await
		.expect("request after reset should receive diagnostic response");
	assert_eq!(after_reset.status().as_u16(), 500);
	assert_eq!(
		after_reset
			.text()
			.await
			.expect("reset diagnostic body should decode"),
		"MSW: No handler for GET /api/reset"
	);
}

#[tokio::test]
async fn native_worker_reset_handlers_preserves_recorded_calls() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::get("/api/reset-handlers").respond(MockResponse::text("ok")));
	worker.start().await;

	let response = reqwest::get(endpoint(&worker, "/api/reset-handlers"))
		.await
		.expect("mock request should succeed");
	assert_eq!(response.status().as_u16(), 200);

	worker.reset_handlers();

	worker.calls_to("/api/reset-handlers").assert_count(1);
	let after_reset = reqwest::get(endpoint(&worker, "/api/reset-handlers"))
		.await
		.expect("request after handler reset should receive diagnostic response");
	assert_eq!(after_reset.status().as_u16(), 500);
	assert_eq!(
		after_reset
			.text()
			.await
			.expect("reset handlers diagnostic body should decode"),
		"MSW: No handler for GET /api/reset-handlers"
	);
	worker.calls_to("/api/reset-handlers").assert_count(2);
}

#[tokio::test]
async fn native_worker_applies_handler_delay() {
	let worker = MockServiceWorker::new();
	worker.handle(
		rest::get("/api/slow")
			.delay(Duration::from_millis(50))
			.respond(MockResponse::text("slow")),
	);
	worker.start().await;

	let started = tokio::time::Instant::now();
	let response = reqwest::get(endpoint(&worker, "/api/slow"))
		.await
		.expect("mock request should succeed");

	assert_eq!(response.status().as_u16(), 200);
	assert_eq!(
		response.text().await.expect("slow body should decode"),
		"slow"
	);
	assert!(
		started.elapsed() >= Duration::from_millis(50),
		"handler delay should be applied"
	);
}

#[tokio::test]
async fn native_worker_returns_diagnostic_response_for_unhandled_requests() {
	let worker = MockServiceWorker::new();
	worker.start().await;

	let response = reqwest::get(endpoint(&worker, "/api/missing"))
		.await
		.expect("unhandled request should return deterministic response");

	assert_eq!(response.status().as_u16(), 500);
	assert_eq!(
		response
			.text()
			.await
			.expect("diagnostic body should decode"),
		"MSW: No handler for GET /api/missing"
	);
	worker.calls_to("/api/missing").assert_count(1);
}

#[tokio::test]
async fn native_worker_rejects_passthrough_policy_at_startup() {
	let worker = MockServiceWorker::with_policy(UnhandledPolicy::Passthrough);

	let error = worker
		.try_start()
		.await
		.expect_err("native passthrough should be rejected");

	assert_eq!(
		error.to_string(),
		"UnhandledPolicy::Passthrough is not supported on native MSW"
	);
}

#[tokio::test]
async fn native_worker_rejects_concurrent_startup() {
	let worker = MockServiceWorker::new();

	let (first, second) = tokio::join!(worker.try_start(), worker.try_start());
	let successes = [first.is_ok(), second.is_ok()]
		.into_iter()
		.filter(|ok| *ok)
		.count();
	let failures = [first, second]
		.into_iter()
		.filter_map(Result::err)
		.map(|error| error.to_string())
		.collect::<Vec<_>>();

	assert_eq!(successes, 1);
	assert_eq!(
		failures,
		vec!["MockServiceWorker is already started".to_string()]
	);

	let response = reqwest::get(endpoint(&worker, "/api/concurrent-start"))
		.await
		.expect("started worker should serve diagnostic responses");
	assert_eq!(response.status().as_u16(), 500);
	assert_eq!(
		response
			.text()
			.await
			.expect("diagnostic body should decode"),
		"MSW: No handler for GET /api/concurrent-start"
	);
	worker.stop().await;
}

#[tokio::test]
async fn native_worker_remains_restartable_after_concurrent_start_and_stop() {
	let worker = Arc::new(MockServiceWorker::new());
	worker.handle(rest::get("/api/restart").respond(MockResponse::text("ok")));

	let start_worker = Arc::clone(&worker);
	let stop_worker = Arc::clone(&worker);
	let (start_result, stop_result) = tokio::join!(
		tokio::spawn(async move { start_worker.try_start().await }),
		tokio::spawn(async move { stop_worker.stop().await })
	);

	start_result
		.expect("startup task should complete")
		.expect("concurrent startup should not fail");
	stop_result.expect("stop task should complete");

	worker.stop().await;
	worker
		.try_start()
		.await
		.expect("worker should restart after concurrent lifecycle calls");
	let response = reqwest::get(endpoint(&worker, "/api/restart"))
		.await
		.expect("restarted worker should serve requests");
	assert_eq!(response.status().as_u16(), 200);
	assert_eq!(
		response.text().await.expect("response body should decode"),
		"ok"
	);
	worker.stop().await;
}

#[tokio::test]
async fn native_worker_network_error_closes_request_without_http_response() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::get("/api/network-error").network_error());
	worker.start().await;

	let result = reqwest::get(endpoint(&worker, "/api/network-error")).await;

	assert!(
		result.is_err(),
		"network_error handler should surface as a client transport error"
	);
	worker.calls_to("/api/network-error").assert_count(1);
}

#[tokio::test]
async fn native_worker_network_error_once_handler_is_consumed() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::get("/api/network-error-once").once().network_error());
	worker.start().await;

	let first = reqwest::get(endpoint(&worker, "/api/network-error-once")).await;
	assert!(
		first.is_err(),
		"network_error handler should surface as a client transport error"
	);

	let second = reqwest::get(endpoint(&worker, "/api/network-error-once"))
		.await
		.expect("consumed once network_error should fall through to diagnostic response");
	assert_eq!(second.status().as_u16(), 500);
	assert_eq!(
		second.text().await.expect("diagnostic body should decode"),
		"MSW: No handler for GET /api/network-error-once"
	);
	worker.calls_to("/api/network-error-once").assert_count(2);
}

#[tokio::test]
async fn native_worker_stop_releases_listener() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::get("/api/lifecycle").respond(MockResponse::text("ok")));
	worker.start().await;
	let url = endpoint(&worker, "/api/lifecycle");

	let before_stop = reqwest::get(&url)
		.await
		.expect("mock request before stop should succeed");
	assert_eq!(before_stop.status().as_u16(), 200);

	worker.stop().await;

	let client = reqwest::Client::builder()
		.timeout(Duration::from_millis(200))
		.build()
		.expect("client should build");
	let after_stop = client.get(&url).send().await;
	assert!(
		after_stop.is_err(),
		"request after stop should not reach a live listener"
	);
}

#[tokio::test]
async fn native_worker_stop_closes_keep_alive_connections() {
	let worker = MockServiceWorker::new();
	worker.handle(rest::get("/api/keep-alive").respond(MockResponse::text("ok")));
	worker.start().await;

	let addr: std::net::SocketAddr = worker
		.url()
		.strip_prefix("http://")
		.expect("native worker URL should use http")
		.parse()
		.expect("native worker URL should contain a socket address");
	let mut stream = tokio::net::TcpStream::connect(addr)
		.await
		.expect("raw keep-alive connection should open");

	stream
		.write_all(
			b"GET /api/keep-alive HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: keep-alive\r\n\r\n",
		)
		.await
		.expect("first request should write");
	let mut buffer = vec![0; 1024];
	let mut response = Vec::new();
	tokio::time::timeout(Duration::from_millis(200), async {
		loop {
			let read = stream
				.read(&mut buffer)
				.await
				.expect("first response should read");
			assert!(read > 0, "first response should not close early");
			response.extend_from_slice(&buffer[..read]);
			if String::from_utf8_lossy(&response).contains("ok") {
				break;
			}
		}
	})
	.await
	.expect("first keep-alive response should complete promptly");
	assert!(
		String::from_utf8_lossy(&response).contains("ok"),
		"first keep-alive request should be served"
	);

	worker.stop().await;

	let write_after_stop = stream
		.write_all(b"GET /api/keep-alive HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
		.await;
	if write_after_stop.is_err() {
		return;
	}

	match tokio::time::timeout(Duration::from_millis(200), stream.read(&mut buffer)).await {
		Ok(Ok(0) | Err(_)) => {}
		Ok(Ok(read_after_stop)) => panic!(
			"stopped worker must not serve requests on an existing keep-alive connection: {}",
			String::from_utf8_lossy(&buffer[..read_after_stop])
		),
		Err(_) => panic!("stopped worker should close keep-alive connections promptly"),
	}
}
