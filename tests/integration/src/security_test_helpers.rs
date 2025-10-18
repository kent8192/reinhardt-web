//! Security integration test helpers
//!
//! Common utilities and mocks for security integration tests

use bytes::Bytes;
use hyper::header::{HeaderMap, HeaderName, HeaderValue};
use hyper::{Method, StatusCode, Uri, Version};
use reinhardt_apps::{Request, Response};
use std::str::FromStr;

/// Create a mock HTTP request for testing
pub fn create_test_request(method: &str, uri: &str, secure: bool) -> Request {
    let method = Method::from_str(method).unwrap_or(Method::GET);
    let uri = Uri::from_str(uri).unwrap_or_else(|_| Uri::from_static("/"));

    let mut headers = HeaderMap::new();

    // Add X-Forwarded-Proto header if secure
    if secure {
        headers.insert(
            HeaderName::from_static("x-forwarded-proto"),
            HeaderValue::from_static("https"),
        );
    }

    let mut request = Request::new(method, uri, Version::HTTP_11, headers, Bytes::new());
    request.is_secure = secure;
    request
}

/// Create a mock HTTPS request
pub fn create_secure_request(method: &str, uri: &str) -> Request {
    create_test_request(method, uri, true)
}

/// Create a mock HTTP request
pub fn create_insecure_request(method: &str, uri: &str) -> Request {
    create_test_request(method, uri, false)
}

/// Create a mock response for testing
pub fn create_test_response() -> Response {
    Response::ok()
}

/// Create a response with custom status code
pub fn create_response_with_status(status: StatusCode) -> Response {
    Response::new(status)
}

/// Create a response with custom headers
pub fn create_response_with_headers(headers: HeaderMap) -> Response {
    let mut response = Response::ok();
    response.headers = headers;
    response
}

/// Check if response has a specific header
pub fn has_header(response: &Response, header_name: &str) -> bool {
    response.headers.contains_key(header_name)
}

/// Get header value from response
pub fn get_header<'a>(response: &'a Response, header_name: &str) -> Option<&'a str> {
    response
        .headers
        .get(header_name)
        .and_then(|v| v.to_str().ok())
}

/// Check if header has specific value
pub fn header_equals(response: &Response, header_name: &str, expected_value: &str) -> bool {
    get_header(response, header_name)
        .map(|v| v == expected_value)
        .unwrap_or(false)
}

/// Check if header contains substring
pub fn header_contains(response: &Response, header_name: &str, substring: &str) -> bool {
    get_header(response, header_name)
        .map(|v| v.contains(substring))
        .unwrap_or(false)
}

/// Assert response status code
pub fn assert_status(response: &Response, expected: StatusCode) {
    assert_eq!(
        response.status, expected,
        "Expected status {}, got {}",
        expected, response.status
    );
}

/// Assert response has header
pub fn assert_has_header(response: &Response, header_name: &str) {
    assert!(
        has_header(response, header_name),
        "Expected response to have header '{}'",
        header_name
    );
}

/// Assert response doesn't have header
pub fn assert_no_header(response: &Response, header_name: &str) {
    assert!(
        !has_header(response, header_name),
        "Expected response to NOT have header '{}'",
        header_name
    );
}

/// Assert header value equals expected
pub fn assert_header_equals(response: &Response, header_name: &str, expected_value: &str) {
    let actual = get_header(response, header_name)
        .unwrap_or_else(|| panic!("Header '{}' not found", header_name));
    assert_eq!(
        actual, expected_value,
        "Expected header '{}' to be '{}', got '{}'",
        header_name, expected_value, actual
    );
}

/// Assert header contains substring
pub fn assert_header_contains(response: &Response, header_name: &str, substring: &str) {
    let actual = get_header(response, header_name)
        .unwrap_or_else(|| panic!("Header '{}' not found", header_name));
    assert!(
        actual.contains(substring),
        "Expected header '{}' to contain '{}', got '{}'",
        header_name,
        substring,
        actual
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_request() {
        let req = create_test_request("GET", "/test", false);
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.uri.path(), "/test");
    }

    #[test]
    fn test_create_secure_request() {
        let req = create_secure_request("POST", "/api");
        assert_eq!(req.method, Method::POST);
        assert!(req.headers.contains_key("x-forwarded-proto"));
    }

    #[test]
    fn test_create_response() {
        let res = create_test_response();
        assert_eq!(res.status, StatusCode::OK);
    }

    #[test]
    fn test_header_helpers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-test-header"),
            HeaderValue::from_static("test-value"),
        );
        let res = create_response_with_headers(headers);

        assert!(has_header(&res, "x-test-header"));
        assert_eq!(get_header(&res, "x-test-header"), Some("test-value"));
        assert!(header_equals(&res, "x-test-header", "test-value"));
        assert!(header_contains(&res, "x-test-header", "test"));
    }
}
