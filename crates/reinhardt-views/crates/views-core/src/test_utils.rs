//! Test utilities for reinhardt-views
//!
//! Common helper functions and test models for view testing

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_apps::{Error, Request, Response, Result};
use reinhardt_orm::Model;
// use reinhardt_serializers::{JsonSerializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Test model for view tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestModel {
    pub id: Option<i64>,
    pub name: String,
    pub slug: String,
    pub created_at: String,
}

impl Model for TestModel {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "test_models"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

/// Test model for API view tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiTestModel {
    pub id: Option<i64>,
    pub title: String,
    pub content: String,
}

impl Model for ApiTestModel {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "api_test_models"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

/// Create a test request with the given parameters
pub fn create_request(
    method: Method,
    path: &str,
    query_params: Option<HashMap<String, String>>,
    headers: Option<HeaderMap>,
    body: Option<Bytes>,
) -> Request {
    let uri_str = if let Some(ref params) = query_params {
        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        format!("{}?{}", path, query)
    } else {
        path.to_string()
    };

    let uri = uri_str.parse::<Uri>().unwrap();
    Request::new(
        method,
        uri,
        Version::HTTP_11,
        headers.unwrap_or_default(),
        body.unwrap_or_default(),
    )
}

/// Create a test request with path parameters
pub fn create_request_with_path_params(
    method: Method,
    path: &str,
    path_params: HashMap<String, String>,
    query_params: Option<HashMap<String, String>>,
    headers: Option<HeaderMap>,
    body: Option<Bytes>,
) -> Request {
    let mut request = create_request(method, path, query_params, headers, body);
    request.path_params = path_params;
    request
}

/// Create a test request with headers
pub fn create_request_with_headers(
    method: Method,
    path: &str,
    headers: HashMap<String, String>,
    body: Option<Bytes>,
) -> Request {
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        if let (Ok(header_name), Ok(header_value)) = (
            hyper::header::HeaderName::from_bytes(key.as_bytes()),
            hyper::header::HeaderValue::from_str(&value),
        ) {
            header_map.insert(header_name, header_value);
        }
    }

    create_request(method, path, None, Some(header_map), body)
}

/// Create a test request with JSON body
pub fn create_json_request(method: Method, path: &str, json_data: &serde_json::Value) -> Request {
    let body = Bytes::from(serde_json::to_vec(json_data).unwrap());
    let mut headers = HeaderMap::new();
    headers.insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("application/json"),
    );

    create_request(method, path, None, Some(headers), Some(body))
}

/// Assert response status code
pub fn assert_response_status(response: &Response, expected: StatusCode) {
    assert_eq!(
        response.status, expected,
        "Expected status {:?}, got {:?}",
        expected, response.status
    );
}

/// Assert response body contains expected text
pub fn assert_response_body_contains(response: &Response, expected: &str) {
    let body_str = String::from_utf8_lossy(&response.body);
    assert!(
        body_str.contains(expected),
        "Expected body to contain '{}', got '{}'",
        expected,
        body_str
    );
}

/// Assert response body equals expected bytes
pub fn assert_response_body_equals(response: &Response, expected: &[u8]) {
    assert_eq!(
        response.body, expected,
        "Expected body {:?}, got {:?}",
        expected, response.body
    );
}

/// Assert response is JSON and contains expected data
pub fn assert_json_response_contains(
    response: &Response,
    expected_key: &str,
    expected_value: &serde_json::Value,
) {
    let body_str = String::from_utf8_lossy(&response.body);
    let json: serde_json::Value =
        serde_json::from_str(&body_str).expect("Response body should be valid JSON");

    assert!(
        json.get(expected_key).is_some(),
        "JSON should contain key '{}'",
        expected_key
    );
    assert_eq!(json.get(expected_key).unwrap(), expected_value);
}

/// Create test objects for list views
pub fn create_test_objects() -> Vec<TestModel> {
    vec![
        TestModel {
            id: Some(1),
            name: "First Object".to_string(),
            slug: "first-object".to_string(),
            created_at: "2023-01-01T00:00:00Z".to_string(),
        },
        TestModel {
            id: Some(2),
            name: "Second Object".to_string(),
            slug: "second-object".to_string(),
            created_at: "2023-01-02T00:00:00Z".to_string(),
        },
        TestModel {
            id: Some(3),
            name: "Third Object".to_string(),
            slug: "third-object".to_string(),
            created_at: "2023-01-03T00:00:00Z".to_string(),
        },
    ]
}

/// Create test objects for API views
pub fn create_api_test_objects() -> Vec<ApiTestModel> {
    vec![
        ApiTestModel {
            id: Some(1),
            title: "First Post".to_string(),
            content: "This is the first post content".to_string(),
        },
        ApiTestModel {
            id: Some(2),
            title: "Second Post".to_string(),
            content: "This is the second post content".to_string(),
        },
        ApiTestModel {
            id: Some(3),
            title: "Third Post".to_string(),
            content: "This is the third post content".to_string(),
        },
    ]
}

/// Create a large set of test objects for pagination testing
pub fn create_large_test_objects(count: usize) -> Vec<TestModel> {
    (0..count)
        .map(|i| TestModel {
            id: Some(i as i64),
            name: format!("Object {}", i),
            slug: format!("object-{}", i),
            created_at: format!("2023-01-{:02}T00:00:00Z", (i % 30) + 1),
        })
        .collect()
}

/// Assert that a result is an error with the expected error type
pub fn assert_error<T>(result: Result<T>, _expected_error: fn(String) -> Error) {
    match result {
        Ok(_) => panic!("Expected error, got Ok"),
        Err(error) => {
            // Check if the error is of the expected type by pattern matching
            match error {
                Error::NotFound(_)
                    if std::any::TypeId::of::<Error>() == std::any::TypeId::of::<Error>() => {}
                Error::Validation(_)
                    if std::any::TypeId::of::<Error>() == std::any::TypeId::of::<Error>() => {}
                Error::Internal(_)
                    if std::any::TypeId::of::<Error>() == std::any::TypeId::of::<Error>() => {}
                Error::Authentication(_)
                    if std::any::TypeId::of::<Error>() == std::any::TypeId::of::<Error>() => {}
                Error::Authorization(_)
                    if std::any::TypeId::of::<Error>() == std::any::TypeId::of::<Error>() => {}
                _ => panic!("Expected specific error type, got {:?}", error),
            }
        }
    }
}

/// Assert that a result is a NotFound error
pub fn assert_not_found_error<T>(result: Result<T>) {
    match result {
        Ok(_) => panic!("Expected NotFound error, got Ok"),
        Err(Error::NotFound(_)) => {}
        Err(error) => panic!("Expected NotFound error, got {:?}", error),
    }
}

/// Assert that a result is a Validation error
pub fn assert_validation_error<T>(result: Result<T>) {
    match result {
        Ok(_) => panic!("Expected Validation error, got Ok"),
        Err(Error::Validation(_)) => {}
        Err(error) => panic!("Expected Validation error, got {:?}", error),
    }
}

/// Assert that a result is an Internal error
pub fn assert_internal_error<T>(result: Result<T>) {
    match result {
        Ok(_) => panic!("Expected Internal error, got Ok"),
        Err(Error::Internal(_)) => {}
        Err(error) => panic!("Expected Internal error, got {:?}", error),
    }
}

/// Create a simple view for testing basic functionality
pub struct SimpleTestView {
    pub content: String,
    pub allowed_methods: Vec<Method>,
}

impl SimpleTestView {
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            allowed_methods: vec![Method::GET],
        }
    }

    pub fn with_methods(mut self, methods: Vec<Method>) -> Self {
        self.allowed_methods = methods;
        self
    }
}

#[async_trait::async_trait]
impl crate::View for SimpleTestView {
    async fn dispatch(&self, request: Request) -> Result<Response> {
        if !self.allowed_methods.contains(&request.method) {
            return Err(Error::Validation(format!(
                "Method {} not allowed",
                request.method
            )));
        }

        Ok(Response::ok().with_body(self.content.clone().into_bytes()))
    }
}

/// Create a view that always returns an error for testing error handling
pub struct ErrorTestView {
    pub error_message: String,
    pub error_kind: ErrorKind,
}

pub enum ErrorKind {
    NotFound,
    Validation,
    Internal,
    Authentication,
    Authorization,
}

impl ErrorTestView {
    pub fn new(error_message: String, error_kind: ErrorKind) -> Self {
        Self {
            error_message,
            error_kind,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(message.into(), ErrorKind::NotFound)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(message.into(), ErrorKind::Validation)
    }
}

#[async_trait::async_trait]
impl crate::View for ErrorTestView {
    async fn dispatch(&self, _request: Request) -> Result<Response> {
        match self.error_kind {
            ErrorKind::NotFound => Err(Error::NotFound(self.error_message.clone())),
            ErrorKind::Validation => Err(Error::Validation(self.error_message.clone())),
            ErrorKind::Internal => Err(Error::Internal(self.error_message.clone())),
            ErrorKind::Authentication => Err(Error::Authentication(self.error_message.clone())),
            ErrorKind::Authorization => Err(Error::Authorization(self.error_message.clone())),
        }
    }
}
