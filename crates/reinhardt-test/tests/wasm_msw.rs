//! Integration tests for MSW-style fetch interception.
//!
//! All test scenarios run sequentially in a single wasm_bindgen_test
//! because wasm-bindgen-test runs tests concurrently in the browser
//! and window.fetch override is a global state.

#![cfg(all(target_arch = "wasm32", feature = "msw"))]

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{RequestInit, Response};

use reinhardt_pages::server_fn::{ServerFnError, ServerFnMetadata, server_fn};
use reinhardt_test::msw::MockResponse;
use reinhardt_test::msw::MockServiceWorker;
use reinhardt_test::msw::UnhandledPolicy;
use reinhardt_test::msw::rest;

wasm_bindgen_test_configure!(run_in_browser);

#[server_fn]
async fn msw_echo(value: String) -> Result<String, ServerFnError> {
	Ok(value)
}

async fn do_fetch(url: &str, method: &str, body: Option<&str>) -> (u16, String) {
	let window = web_sys::window().unwrap();
	let opts = RequestInit::new();
	opts.set_method(method);
	if let Some(b) = body {
		opts.set_body(&JsValue::from_str(b));
	}
	// Use fetch(url, init) form so interceptor can extract body from init
	let resp: Response = JsFuture::from(window.fetch_with_str_and_init(url, &opts))
		.await
		.unwrap()
		.unchecked_into();
	let status = resp.status();
	let body_text = JsFuture::from(resp.text().unwrap())
		.await
		.unwrap()
		.as_string()
		.unwrap_or_default();
	(status, body_text)
}

async fn do_fetch_expect_error(url: &str, method: &str) {
	let window = web_sys::window().unwrap();
	let opts = RequestInit::new();
	opts.set_method(method);
	let result = JsFuture::from(window.fetch_with_str_and_init(url, &opts)).await;
	assert!(
		result.is_err(),
		"Expected fetch to fail for {} {}",
		method,
		url
	);
}

#[wasm_bindgen_test]
async fn msw_integration_tests() {
	// === Scenario 1: GET handler with JSON response ===
	{
		let worker = MockServiceWorker::new();
		worker.handle(
			rest::get("/api/test")
				.respond(MockResponse::json(serde_json::json!({"hello": "world"}))),
		);
		worker.start().await;

		let (status, body) = do_fetch("/api/test", "GET", None).await;
		assert_eq!(status, 200);
		assert_eq!(body, r#"{"hello":"world"}"#);
		worker.calls_to("/api/test").assert_called();
		worker.calls_to("/api/test").assert_count(1);
		worker.stop().await;
	}

	// === Scenario 2: POST handler with custom status ===
	{
		let worker = MockServiceWorker::new();
		worker.handle(
			rest::post("/api/create")
				.respond(MockResponse::json(serde_json::json!({"id": 1})).with_status(201)),
		);
		worker.start().await;

		let (status, body) = do_fetch("/api/create", "POST", Some(r#"{"name":"Alice"}"#)).await;
		assert_eq!(status, 201);
		assert_eq!(body, r#"{"id":1}"#);
		worker.stop().await;
	}

	// === Scenario 3: Parameterized URL matching ===
	{
		let worker = MockServiceWorker::new();
		worker.handle(
			rest::get("/api/users/:id")
				.respond_with(|req| MockResponse::json(serde_json::json!({"url": req.url}))),
		);
		worker.start().await;

		let (status, body) = do_fetch("/api/users/42", "GET", None).await;
		assert_eq!(status, 200);
		assert!(body.contains("/api/users/42"));
		worker.stop().await;
	}

	// === Scenario 4: Multiple handlers ===
	{
		let worker = MockServiceWorker::new();
		worker.handle(rest::get("/api/a").respond(MockResponse::json(serde_json::json!({"a": 1}))));
		worker.handle(rest::get("/api/b").respond(MockResponse::json(serde_json::json!({"b": 2}))));
		worker.start().await;

		let (_, body_a) = do_fetch("/api/a", "GET", None).await;
		let (_, body_b) = do_fetch("/api/b", "GET", None).await;
		assert_eq!(body_a, r#"{"a":1}"#);
		assert_eq!(body_b, r#"{"b":2}"#);
		worker.calls_to("/api/a").assert_count(1);
		worker.calls_to("/api/b").assert_count(1);
		worker.stop().await;
	}

	// === Scenario 5: Echo request body ===
	{
		let worker = MockServiceWorker::new();
		worker.handle(rest::post("/api/echo").respond_with(|req| {
			let body = req.body.clone().unwrap_or_default();
			MockResponse::text(body)
		}));
		worker.start().await;

		let (status, body) = do_fetch("/api/echo", "POST", Some("hello world")).await;
		assert_eq!(status, 200);
		assert_eq!(body, "hello world");
		worker.stop().await;
	}

	// === Scenario 6: Reset clears state ===
	{
		let worker = MockServiceWorker::new();
		worker.handle(rest::get("/api/reset").respond(MockResponse::empty()));
		worker.start().await;

		let _ = do_fetch("/api/reset", "GET", None).await;
		let calls_before = worker.all_calls().len();
		assert!(
			calls_before >= 1,
			"Expected at least 1 call, got {calls_before}"
		);
		worker.reset();
		assert_eq!(worker.all_calls().len(), 0);
		worker.stop().await;
	}

	// === Scenario 7: Network error ===
	{
		let worker = MockServiceWorker::new();
		worker.handle(rest::get("/api/fail").network_error());
		worker.start().await;

		do_fetch_expect_error("/api/fail", "GET").await;
		worker.stop().await;
	}

	// === Scenario 8: Unhandled policy error ===
	{
		let worker = MockServiceWorker::new();
		worker.start().await;

		do_fetch_expect_error("/api/no-handler", "GET").await;
		worker.stop().await;
	}

	// === Scenario 9: Once handler consumed ===
	{
		let worker = MockServiceWorker::with_policy(UnhandledPolicy::Passthrough);
		worker.handle(
			rest::get("/api/once")
				.once()
				.respond(MockResponse::json(serde_json::json!({"first": true}))),
		);
		worker.start().await;

		let (status, _) = do_fetch("/api/once", "GET", None).await;
		assert_eq!(status, 200);
		worker.stop().await;
	}

	// === Scenario 10: Starting a new worker recovers stale global fetch state ===
	{
		let stale = MockServiceWorker::new();
		stale.handle(rest::get("/api/stale").respond(MockResponse::text("stale")));
		stale.start().await;

		let replacement = MockServiceWorker::new();
		replacement.handle(rest::get("/api/recovered").respond(MockResponse::text("recovered")));
		replacement.start().await;

		let (status, body) = do_fetch("/api/recovered", "GET", None).await;
		assert_eq!(status, 200);
		assert_eq!(body, "recovered");
		replacement.stop().await;
	}

	// === Scenario 11: Generated server_fn clients reach MSW with absolute URLs ===
	{
		let worker = MockServiceWorker::new();
		worker.handle_server_fn::<msw_echo::marker>(|args| Ok(args.value));
		worker.start().await;

		let endpoint = reinhardt_pages::server_fn::resolve_endpoint(
			<msw_echo::marker as ServerFnMetadata>::PATH,
		);
		assert!(
			web_sys::Url::new(&endpoint).is_ok(),
			"resolved server_fn endpoint should be absolute: {endpoint}"
		);
		assert!(
			endpoint.starts_with("http://") || endpoint.starts_with("https://"),
			"resolved server_fn endpoint should include browser HTTP origin: {endpoint}"
		);

		let manual_response = reinhardt_pages::__private::reqwest::Client::new()
			.post(&endpoint)
			.header("Content-Type", "application/json")
			.body(r#"{"value":"manual"}"#)
			.send()
			.await
			.expect("manual reqwest POST should accept resolved endpoint");
		assert!(manual_response.status().is_success());

		let response = msw_echo("pong".to_string())
			.await
			.expect("server_fn should be handled by MSW");
		assert_eq!(response, "pong");

		let calls = worker.all_calls();
		assert_eq!(calls.len(), 2);
		assert!(
			calls[0]
				.url
				.ends_with(<msw_echo::marker as ServerFnMetadata>::PATH)
		);
		assert!(
			web_sys::Url::new(&calls[0].url).is_ok(),
			"recorded server_fn URL should be absolute: {}",
			calls[0].url
		);
		worker.stop().await;
	}
}
