#![cfg(not(target_arch = "wasm32"))]
//! Native server function handler regression tests.

use bytes::Bytes;
use hyper::{Method, header};
use reinhardt_http::Request;
use reinhardt_pages::server_fn::{ServerFnError, ServerFnRegistration, server_fn};
use rstest::rstest;

#[server_fn]
async fn echo_name(name: String) -> Result<String, ServerFnError> {
	Ok(name)
}

#[tokio::test]
async fn json_server_fn_accepts_form_content_type_without_extractors() {
	// Arrange
	let request = Request::builder()
		.method(Method::POST)
		.uri("/api/server_fn/echo_name")
		.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
		.body(Bytes::from_static(b"name=Alice"))
		.build()
		.expect("request should build");

	// Act
	let body = echo_name::marker::handle(request)
		.await
		.expect("server function should accept form-encoded input");
	let name: String = serde_json::from_slice(&body).expect("response should be JSON");

	// Assert
	assert_eq!(name, "Alice");
}

#[rstest]
fn generated_query_key_helper_encodes_server_fn_identity_and_args() {
	// Arrange & Act
	let module_key = echo_name::key("Alice".to_string());
	let method_key = echo_name.key("Alice".to_string());

	// Assert
	assert_eq!(
		module_key.id(),
		r#"server_fn:/api/server_fn/echo_name:json:["Alice"]"#
	);
	assert_eq!(method_key.id(), module_key.id());
}
