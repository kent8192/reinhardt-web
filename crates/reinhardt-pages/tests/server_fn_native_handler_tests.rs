#![cfg(not(target_arch = "wasm32"))]
//! Native server function handler regression tests.

use bytes::Bytes;
use hyper::{Method, header};
use reinhardt_di::params::{FromRequest, ParamContext, ParamError, ParamResult};
use reinhardt_http::Request;
use reinhardt_pages::server_fn::{ServerFnError, ServerFnRegistration, server_fn};
use rstest::rstest;

#[server_fn]
async fn echo_name(name: String) -> Result<String, ServerFnError> {
	Ok(name)
}

#[server_fn]
async fn echo_alias(name: String) -> Result<String, ServerFnError> {
	Ok(name)
}

struct Authorization;

#[async_trait::async_trait]
impl FromRequest for Authorization {
	async fn from_request(_request: &Request, _context: &ParamContext) -> ParamResult<Self> {
		Err(ParamError::Authentication("token=top-secret".to_string()))
	}
}

struct SessionId;

#[async_trait::async_trait]
impl FromRequest for SessionId {
	async fn from_request(_request: &Request, _context: &ParamContext) -> ParamResult<Self> {
		Err(ParamError::Internal(
			"database password=top-secret".to_string(),
		))
	}
}

#[server_fn]
async fn authentication_extractor(_authorization: Authorization) -> Result<(), ServerFnError> {
	Ok(())
}

#[server_fn]
async fn internal_extractor(_session_id: SessionId) -> Result<(), ServerFnError> {
	Ok(())
}

#[tokio::test]
async fn authentication_extractor_returns_sanitized_unauthorized_error() {
	let request = Request::builder()
		.method(Method::POST)
		.uri("/api/server_fn/authentication_extractor")
		.build()
		.expect("request should build");

	let body = authentication_extractor::marker::handle(request)
		.await
		.expect_err("authentication extractor should reject the request");

	assert_eq!(authentication_extractor::marker::error_status(&body), 401);
	assert_eq!(
		serde_json::from_slice::<ServerFnError>(&body).expect("error should be valid JSON"),
		ServerFnError::server(401, "Authentication required")
	);
	assert!(
		!String::from_utf8(body.to_vec())
			.unwrap()
			.contains("top-secret")
	);
}

#[tokio::test]
async fn internal_extractor_returns_sanitized_internal_error() {
	let request = Request::builder()
		.method(Method::POST)
		.uri("/api/server_fn/internal_extractor")
		.build()
		.expect("request should build");

	let body = internal_extractor::marker::handle(request)
		.await
		.expect_err("internal extractor should reject the request");

	assert_eq!(internal_extractor::marker::error_status(&body), 500);
	assert_eq!(
		serde_json::from_slice::<ServerFnError>(&body).expect("error should be valid JSON"),
		ServerFnError::server(500, "Internal server error")
	);
	assert!(
		!String::from_utf8(body.to_vec())
			.unwrap()
			.contains("top-secret")
	);
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
	// Act
	let echo_key = echo_name::key("Alice".to_string());
	let alias_key = echo_alias::key("Alice".to_string());

	// Assert
	assert_eq!(
		echo_key.id(),
		"server_fn:/api/server_fn/echo_name:json:sha256:ab576365fddb09f8b9117212e0d01bf2b8ce8202923d6cff26034af8dfd88e15"
	);
	assert_eq!(
		alias_key.id(),
		"server_fn:/api/server_fn/echo_alias:json:sha256:ab576365fddb09f8b9117212e0d01bf2b8ce8202923d6cff26034af8dfd88e15"
	);
	assert_ne!(echo_key.id(), alias_key.id());
}
