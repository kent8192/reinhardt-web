use hyper::Method;
use reinhardt_http::Response;
use reinhardt_testkit::{stub::StubRouter, test_server_guard};
use rstest::rstest;
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[rstest]
#[tokio::test]
async fn captured_counter_increments_and_post_returns_body_over_http() {
	// Arrange
	let calls = Arc::new(AtomicUsize::new(0));
	let captured_calls = Arc::clone(&calls);
	let router = StubRouter::new().post("/webhook", move |request| {
		let calls = Arc::clone(&captured_calls);
		async move {
			calls.fetch_add(1, Ordering::SeqCst);
			Ok(Response::ok().with_body(request.body().clone()))
		}
	});
	let server = test_server_guard(router.into_server_router()).await;
	let client = reqwest::Client::new();
	let url = format!("{}/webhook", server.url);

	// Act
	let first = client
		.post(&url)
		.body("first payload")
		.send()
		.await
		.unwrap();
	let first_status = first.status();
	let first_body = first.text().await.unwrap();
	let second = client
		.post(&url)
		.body("second payload")
		.send()
		.await
		.unwrap();
	let second_status = second.status();
	let second_body = second.text().await.unwrap();

	// Assert
	assert_eq!(first_status, reqwest::StatusCode::OK);
	assert_eq!(first_body, "first payload");
	assert_eq!(second_status, reqwest::StatusCode::OK);
	assert_eq!(second_body, "second payload");
	assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[rstest]
#[tokio::test]
async fn get_on_post_only_path_returns_405_without_calling_closure() {
	// Arrange
	let calls = Arc::new(AtomicUsize::new(0));
	let captured_calls = Arc::clone(&calls);
	let router = StubRouter::new().post("/webhook", move |_request| {
		captured_calls.fetch_add(1, Ordering::SeqCst);
		async { Ok(Response::ok()) }
	});
	let server = test_server_guard(router.into()).await;

	// Act
	let response = reqwest::get(format!("{}/webhook", server.url))
		.await
		.unwrap();

	// Assert
	assert_eq!(response.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);
	assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[rstest]
#[case(Method::GET)]
#[case(Method::POST)]
#[case(Method::PUT)]
#[case(Method::PATCH)]
#[case(Method::DELETE)]
#[case(Method::OPTIONS)]
#[tokio::test]
async fn shorthands_and_explicit_method_route_dispatch_over_http(#[case] method: Method) {
	// Arrange
	let handler = |_request| async { Ok(Response::ok().with_body("stub response")) };
	let router = StubRouter::new();
	let router = match method {
		Method::GET => router.get("/stub", handler),
		Method::POST => router.post("/stub", handler),
		Method::PUT => router.put("/stub", handler),
		Method::PATCH => router.patch("/stub", handler),
		Method::DELETE => router.delete("/stub", handler),
		_ => router.route("/stub", method.clone(), handler),
	};
	let server = test_server_guard(router.into()).await;

	// Act
	let response = reqwest::Client::new()
		.request(method, format!("{}/stub", server.url))
		.send()
		.await
		.unwrap();
	let status = response.status();
	let body = response.text().await.unwrap();

	// Assert
	assert_eq!(status, reqwest::StatusCode::OK);
	assert_eq!(body, "stub response");
}

#[rstest]
#[tokio::test]
async fn different_methods_on_one_path_dispatch_to_their_own_closures() {
	// Arrange
	let router = StubRouter::new()
		.get("/stub", |_request| async {
			Ok(Response::ok().with_body("read"))
		})
		.post("/stub", |_request| async {
			Ok(Response::ok().with_body("write"))
		});
	let server = test_server_guard(router.into()).await;
	let url = format!("{}/stub", server.url);
	let client = reqwest::Client::new();

	// Act
	let read = client.get(&url).send().await.unwrap();
	let write = client.post(&url).send().await.unwrap();

	// Assert
	assert_eq!(read.status(), reqwest::StatusCode::OK);
	assert_eq!(read.text().await.unwrap(), "read");
	assert_eq!(write.status(), reqwest::StatusCode::OK);
	assert_eq!(write.text().await.unwrap(), "write");
}

#[rstest]
#[should_panic(expected = "duplicate stub route: POST /webhook")]
fn duplicate_path_and_method_panics_at_registration() {
	// Arrange
	let router = StubRouter::new().post("/webhook", |_request| async { Ok(Response::ok()) });

	// Act
	// Assert: registration must panic before any server is started.
	let _router = router.route("/webhook", Method::POST, |_request| async {
		Ok(Response::ok())
	});
}
