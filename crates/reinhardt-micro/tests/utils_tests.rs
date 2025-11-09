//! Integration tests for reinhardt-micro utilities

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version, header};
use reinhardt_micro::utils::*;
use reinhardt_micro::Request;
use serde::{Deserialize, Serialize};

#[test]
fn test_response_builders_ok_json() {
	#[derive(Serialize)]
	struct TestData {
		message: String,
	}

	let data = TestData {
		message: "test".to_string(),
	};
	let response = ok_json(data).unwrap();
	assert_eq!(response.status, StatusCode::OK);
}

#[test]
fn test_response_builders_created_json() {
	#[derive(Serialize)]
	struct TestData {
		id: i64,
	}

	let data = TestData { id: 123 };
	let response = created_json(data).unwrap();
	assert_eq!(response.status, StatusCode::CREATED);
}

#[test]
fn test_response_builders_no_content() {
	let response = no_content();
	assert_eq!(response.status, StatusCode::NO_CONTENT);
}

#[test]
fn test_response_builders_accepted() {
	let response = accepted();
	assert_eq!(response.status, StatusCode::ACCEPTED);
}

#[test]
fn test_response_builders_bad_request() {
	let response = bad_request("invalid input");
	assert_eq!(response.status, StatusCode::BAD_REQUEST);
	assert!(String::from_utf8_lossy(&response.body).contains("invalid input"));
}

#[test]
fn test_response_builders_unauthorized() {
	let response = unauthorized("not authenticated");
	assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[test]
fn test_response_builders_forbidden() {
	let response = forbidden("access denied");
	assert_eq!(response.status, StatusCode::FORBIDDEN);
}

#[test]
fn test_response_builders_not_found() {
	let response = not_found("resource not found");
	assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[test]
fn test_response_builders_conflict() {
	let response = conflict("duplicate entry");
	assert_eq!(response.status, StatusCode::CONFLICT);
}

#[test]
fn test_response_builders_unprocessable_entity() {
	let response = unprocessable_entity("validation failed");
	assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[test]
fn test_response_builders_internal_error() {
	let response = internal_error("server error");
	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_response_builders_service_unavailable() {
	let response = service_unavailable("under maintenance");
	assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn test_request_helpers_extract_bearer_token() {
	let mut headers = HeaderMap::new();
	headers.insert(
		header::AUTHORIZATION,
		"Bearer test_token_123".parse().unwrap(),
	);

	let request = Request::new(
		Method::GET,
		"/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	let token = extract_bearer_token(&request);
	assert_eq!(token, Some("test_token_123".to_string()));
}

#[test]
fn test_request_helpers_parse_query_params() {
	#[derive(Deserialize, Debug, PartialEq)]
	struct QueryParams {
		page: u32,
		limit: u32,
	}

	let request = Request::new(
		Method::GET,
		"/api?page=2&limit=10".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let params: QueryParams = parse_query_params(&request).unwrap();
	assert_eq!(params, QueryParams { page: 2, limit: 10 });
}

#[test]
fn test_request_helpers_validate_content_type() {
	let mut headers = HeaderMap::new();
	headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

	let request = Request::new(
		Method::POST,
		"/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	assert!(validate_content_type(&request, "application/json").is_ok());
}

#[test]
fn test_request_helpers_get_header() {
	let mut headers = HeaderMap::new();
	headers.insert(header::USER_AGENT, "TestClient/1.0".parse().unwrap());

	let request = Request::new(
		Method::GET,
		"/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	let user_agent = get_header(&request, "user-agent");
	assert_eq!(user_agent, Some("TestClient/1.0".to_string()));
}

#[test]
fn test_request_helpers_get_client_ip() {
	let mut headers = HeaderMap::new();
	headers.insert(
		header::HeaderName::from_static("x-forwarded-for"),
		"192.168.1.1".parse().unwrap(),
	);

	let request = Request::new(
		Method::GET,
		"/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	let ip = get_client_ip(&request);
	assert_eq!(ip, Some("192.168.1.1".parse().unwrap()));
}

// TODO: Implement test helper functions (test_request, assert_json_response, assert_status, extract_json)
// These tests are temporarily disabled until the helper functions are implemented
/*
#[test]
fn test_testing_test_request() {
	let request = test_request(Method::GET, "/test", None);
	assert_eq!(request.method, Method::GET);
	assert_eq!(request.uri.path(), "/test");
}

#[test]
fn test_testing_assert_json_response() {
	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct TestData {
		id: i64,
		name: String,
	}

	let data = TestData {
		id: 1,
		name: "test".to_string(),
	};
	let json = serde_json::to_string(&data).unwrap();
	let response = Response::ok()
		.with_header("Content-Type", "application/json")
		.with_body(json);

	let expected = TestData {
		id: 1,
		name: "test".to_string(),
	};
	assert!(assert_json_response(response, expected).is_ok());
}

#[test]
fn test_testing_assert_status() {
	let response = Response::ok();
	assert!(assert_status(&response, StatusCode::OK).is_ok());
}

#[test]
fn test_testing_extract_json() {
	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct TestData {
		id: i64,
		name: String,
	}

	let data = TestData {
		id: 1,
		name: "test".to_string(),
	};
	let json = serde_json::to_string(&data).unwrap();
	let response = Response::ok()
		.with_header("Content-Type", "application/json")
		.with_body(json);

	let extracted: TestData = extract_json(response).unwrap();
	assert_eq!(extracted.id, 1);
	assert_eq!(extracted.name, "test");
}
*/
