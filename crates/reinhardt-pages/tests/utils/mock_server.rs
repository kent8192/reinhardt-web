//! Mock Server for Testing
//!
//! Provides HTTP mocking capabilities for WASM tests.
//!
//! # Features
//!
//! - Mock HTTP GET/POST/PUT/DELETE requests
//! - Response registration with status codes, headers, and bodies
//! - Request history tracking
//! - Pattern-based URL matching
//!
//! # Example
//!
//! ```rust
//! use reinhardt_pages_test_utils::MockServer;
//!
//! let mut server = MockServer::new();
//!
//! // Register a mock response
//! server.mock_get("/api/users", MockResponse {
//!     status: 200,
//!     body: r#"[{"id": 1, "name": "Alice"}]"#.to_string(),
//!     headers: vec![("Content-Type".to_string(), "application/json".to_string())],
//! });
//!
//! // Make request (in WASM environment, this would be intercepted)
//! let response = fetch("/api/users").await;
//! ```

use std::collections::HashMap;

/// A mock HTTP response
#[derive(Debug, Clone)]
pub struct MockResponse {
	/// HTTP status code
	pub status: u16,
	/// Response body
	pub body: String,
	/// Response headers
	pub headers: Vec<(String, String)>,
}

impl MockResponse {
	/// Creates a new mock response with status 200 and empty body
	pub fn new() -> Self {
		Self {
			status: 200,
			body: String::new(),
			headers: Vec::new(),
		}
	}

	/// Sets the status code
	pub fn status(mut self, status: u16) -> Self {
		self.status = status;
		self
	}

	/// Sets the response body
	pub fn body(mut self, body: impl Into<String>) -> Self {
		self.body = body.into();
		self
	}

	/// Adds a header
	pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.headers.push((name.into(), value.into()));
		self
	}

	/// Creates a JSON response
	pub fn json(body: impl Into<String>) -> Self {
		Self::new()
			.status(200)
			.header("Content-Type", "application/json")
			.body(body.into())
	}

	/// Creates an error response
	pub fn error(status: u16, message: impl Into<String>) -> Self {
		Self::new().status(status).body(message.into())
	}
}

impl Default for MockResponse {
	fn default() -> Self {
		Self::new()
	}
}

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
	GET,
	POST,
	PUT,
	DELETE,
	PATCH,
}

impl Method {
	pub fn as_str(&self) -> &'static str {
		match self {
			Method::GET => "GET",
			Method::POST => "POST",
			Method::PUT => "PUT",
			Method::DELETE => "DELETE",
			Method::PATCH => "PATCH",
		}
	}
}

/// A recorded HTTP request
#[derive(Debug, Clone)]
pub struct RecordedRequest {
	/// HTTP method
	pub method: Method,
	/// Request path
	pub path: String,
	/// Request headers
	pub headers: HashMap<String, String>,
	/// Request body (if any)
	pub body: Option<String>,
}

/// Mock HTTP server
pub struct MockServer {
	/// Registered mock responses
	routes: HashMap<(Method, String), MockResponse>,
	/// Request history
	history: Vec<RecordedRequest>,
	/// Whether to log requests
	log_requests: bool,
}

impl MockServer {
	/// Creates a new mock server
	pub fn new() -> Self {
		Self {
			routes: HashMap::new(),
			history: Vec::new(),
			log_requests: false,
		}
	}

	/// Enables request logging
	pub fn with_logging(mut self) -> Self {
		self.log_requests = true;
		self
	}

	/// Registers a mock GET response
	pub fn mock_get(&mut self, path: impl Into<String>, response: MockResponse) {
		self.routes.insert((Method::GET, path.into()), response);
	}

	/// Registers a mock POST response
	pub fn mock_post(&mut self, path: impl Into<String>, response: MockResponse) {
		self.routes.insert((Method::POST, path.into()), response);
	}

	/// Registers a mock PUT response
	pub fn mock_put(&mut self, path: impl Into<String>, response: MockResponse) {
		self.routes.insert((Method::PUT, path.into()), response);
	}

	/// Registers a mock DELETE response
	pub fn mock_delete(&mut self, path: impl Into<String>, response: MockResponse) {
		self.routes.insert((Method::DELETE, path.into()), response);
	}

	/// Registers a mock PATCH response
	pub fn mock_patch(&mut self, path: impl Into<String>, response: MockResponse) {
		self.routes.insert((Method::PATCH, path.into()), response);
	}

	/// Returns a mock response for the given request
	pub fn handle_request(
		&mut self,
		method: Method,
		path: &str,
		headers: HashMap<String, String>,
		body: Option<String>,
	) -> Option<MockResponse> {
		// Record request
		self.history.push(RecordedRequest {
			method,
			path: path.to_string(),
			headers: headers.clone(),
			body: body.clone(),
		});

		// Log if enabled
		if self.log_requests {
			eprintln!(
				"[MockServer] {} {} (headers: {:?}, body: {:?})",
				method.as_str(),
				path,
				headers,
				body
			);
		}

		// Find matching route
		self.routes.get(&(method, path.to_string())).cloned()
	}

	/// Returns the request history
	pub fn history(&self) -> &[RecordedRequest] {
		&self.history
	}

	/// Clears the request history
	pub fn clear_history(&mut self) {
		self.history.clear();
	}

	/// Returns the number of requests made
	pub fn request_count(&self) -> usize {
		self.history.len()
	}

	/// Returns the number of requests made to a specific path
	pub fn request_count_for(&self, method: Method, path: &str) -> usize {
		self.history
			.iter()
			.filter(|r| r.method == method && r.path == path)
			.count()
	}

	/// Asserts that a request was made
	pub fn assert_requested(&self, method: Method, path: &str) {
		assert!(
			self.request_count_for(method, path) > 0,
			"Expected request to {} {} but none was made",
			method.as_str(),
			path
		);
	}

	/// Asserts that a request was made exactly once
	pub fn assert_requested_once(&self, method: Method, path: &str) {
		let count = self.request_count_for(method, path);
		assert_eq!(
			count, 1,
			"Expected exactly one request to {} {} but {} were made",
			method.as_str(),
			path,
			count
		);
	}

	/// Asserts that a request was not made
	pub fn assert_not_requested(&self, method: Method, path: &str) {
		assert_eq!(
			self.request_count_for(method, path),
			0,
			"Expected no requests to {} {} but some were made",
			method.as_str(),
			path
		);
	}

	/// Resets the mock server
	pub fn reset(&mut self) {
		self.routes.clear();
		self.history.clear();
	}
}

impl Default for MockServer {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_mock_response_builder() {
		let response = MockResponse::new()
			.status(201)
			.body("Created")
			.header("X-Custom", "value");

		assert_eq!(response.status, 201);
		assert_eq!(response.body, "Created");
		assert_eq!(response.headers.len(), 1);
		assert_eq!(response.headers[0], ("X-Custom".to_string(), "value".to_string()));
	}

	#[test]
	fn test_mock_response_json() {
		let response = MockResponse::json(r#"{"id": 1}"#);

		assert_eq!(response.status, 200);
		assert_eq!(response.body, r#"{"id": 1}"#);
		assert!(response
			.headers
			.contains(&("Content-Type".to_string(), "application/json".to_string())));
	}

	#[test]
	fn test_mock_response_error() {
		let response = MockResponse::error(404, "Not Found");

		assert_eq!(response.status, 404);
		assert_eq!(response.body, "Not Found");
	}

	#[test]
	fn test_mock_server_registration() {
		let mut server = MockServer::new();

		server.mock_get("/api/users", MockResponse::json(r#"[]"#));
		server.mock_post("/api/users", MockResponse::json(r#"{"id": 1}"#).status(201));

		assert_eq!(server.routes.len(), 2);
	}

	#[test]
	fn test_mock_server_handle_request() {
		let mut server = MockServer::new();

		server.mock_get("/test", MockResponse::new().body("Hello"));

		let response = server.handle_request(Method::GET, "/test", HashMap::new(), None);

		assert!(response.is_some());
		let response = response.unwrap();
		assert_eq!(response.body, "Hello");
		assert_eq!(server.request_count(), 1);
	}

	#[test]
	fn test_mock_server_history() {
		let mut server = MockServer::new();

		server.mock_get("/test1", MockResponse::new());
		server.mock_post("/test2", MockResponse::new());

		server.handle_request(Method::GET, "/test1", HashMap::new(), None);
		server.handle_request(Method::POST, "/test2", HashMap::new(), Some("body".to_string()));

		assert_eq!(server.history().len(), 2);
		assert_eq!(server.history()[0].path, "/test1");
		assert_eq!(server.history()[1].path, "/test2");
		assert_eq!(server.history()[1].body, Some("body".to_string()));
	}

	#[test]
	fn test_mock_server_request_count_for() {
		let mut server = MockServer::new();

		server.mock_get("/test", MockResponse::new());

		server.handle_request(Method::GET, "/test", HashMap::new(), None);
		server.handle_request(Method::GET, "/test", HashMap::new(), None);

		assert_eq!(server.request_count_for(Method::GET, "/test"), 2);
		assert_eq!(server.request_count_for(Method::POST, "/test"), 0);
	}

	#[test]
	fn test_mock_server_assertions() {
		let mut server = MockServer::new();

		server.mock_get("/test", MockResponse::new());
		server.handle_request(Method::GET, "/test", HashMap::new(), None);

		server.assert_requested(Method::GET, "/test");
		server.assert_requested_once(Method::GET, "/test");
		server.assert_not_requested(Method::POST, "/test");
	}

	#[test]
	#[should_panic(expected = "Expected request to GET /missing but none was made")]
	fn test_mock_server_assert_requested_fails() {
		let server = MockServer::new();
		server.assert_requested(Method::GET, "/missing");
	}

	#[test]
	fn test_mock_server_reset() {
		let mut server = MockServer::new();

		server.mock_get("/test", MockResponse::new());
		server.handle_request(Method::GET, "/test", HashMap::new(), None);

		assert_eq!(server.routes.len(), 1);
		assert_eq!(server.history().len(), 1);

		server.reset();

		assert_eq!(server.routes.len(), 0);
		assert_eq!(server.history().len(), 0);
	}
}
