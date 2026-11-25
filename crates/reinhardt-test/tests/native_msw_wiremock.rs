#![cfg(all(
	not(all(target_family = "wasm", target_os = "unknown")),
	feature = "msw"
))]

use reinhardt_test::msw::wiremock::matchers::{header, method, path};
use reinhardt_test::msw::wiremock::{Mock, MockServer, ResponseTemplate};
use rstest::rstest;
use serde_json::json;

#[rstest]
#[tokio::test]
async fn reexported_mock_server_answers_a_matching_request() {
	// Arrange
	let server = MockServer::start().await;
	Mock::given(method("POST"))
		.and(path("/v1/items"))
		.and(header("authorization", "Bearer token"))
		.respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": 7})))
		.expect(1)
		.mount(&server)
		.await;

	// Act
	let response = reqwest::Client::new()
		.post(format!("{}/v1/items", server.uri()))
		.bearer_auth("token")
		.send()
		.await
		.expect("the mock server should answer");

	// Assert
	assert_eq!(response.status().as_u16(), 201);
	let body: serde_json::Value = response.json().await.expect("JSON body");
	assert_eq!(body, json!({"id": 7}));
	server.verify().await;
}

#[rstest]
#[tokio::test]
async fn reexported_mock_server_rejects_an_unmatched_request() {
	// Arrange
	let server = MockServer::start().await;

	// Act
	let response = reqwest::get(format!("{}/missing", server.uri()))
		.await
		.expect("the mock server should answer");

	// Assert
	assert_eq!(response.status().as_u16(), 404);
	assert_eq!(server.received_requests().await.map(|r| r.len()), Some(1));
}
