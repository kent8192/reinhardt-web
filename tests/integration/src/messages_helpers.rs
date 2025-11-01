//! Test helpers and mocks for message integration tests

use std::collections::HashMap;

/// Mock HTTP request for testing
#[derive(Debug, Clone)]
pub struct MockRequest {
	pub cookies: HashMap<String, String>,
	pub session: Option<MockSession>,
}

impl MockRequest {
	pub fn new() -> Self {
		Self {
			cookies: HashMap::new(),
			session: None,
		}
	}

	pub fn with_session(mut self) -> Self {
		self.session = Some(MockSession::new());
		self
	}

	pub fn with_cookie(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
		self.cookies.insert(name.into(), value.into());
		self
	}

	pub fn get_cookie(&self, name: &str) -> Option<&String> {
		self.cookies.get(name)
	}

	pub fn set_cookie(&mut self, name: impl Into<String>, value: impl Into<String>) {
		self.cookies.insert(name.into(), value.into());
	}

	pub fn session(&self) -> Option<&MockSession> {
		self.session.as_ref()
	}

	pub fn session_mut(&mut self) -> Option<&mut MockSession> {
		self.session.as_mut()
	}
}

impl Default for MockRequest {
	fn default() -> Self {
		Self::new()
	}
}

/// Mock HTTP response for testing
#[derive(Debug, Clone)]
pub struct MockResponse {
	pub status: u16,
	pub cookies: HashMap<String, String>,
	pub body: String,
}

impl MockResponse {
	pub fn new() -> Self {
		Self {
			status: 200,
			cookies: HashMap::new(),
			body: String::new(),
		}
	}

	pub fn with_status(mut self, status: u16) -> Self {
		self.status = status;
		self
	}

	pub fn set_cookie(&mut self, name: impl Into<String>, value: impl Into<String>) {
		self.cookies.insert(name.into(), value.into());
	}

	pub fn get_cookie(&self, name: &str) -> Option<&String> {
		self.cookies.get(name)
	}
}

impl Default for MockResponse {
	fn default() -> Self {
		Self::new()
	}
}

/// Mock session for testing
#[derive(Debug, Clone)]
pub struct MockSession {
	pub data: HashMap<String, String>,
}

impl MockSession {
	pub fn new() -> Self {
		Self {
			data: HashMap::new(),
		}
	}

	pub fn get(&self, key: &str) -> Option<&String> {
		self.data.get(key)
	}

	pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
		self.data.insert(key.into(), value.into());
	}

	pub fn remove(&mut self, key: &str) -> Option<String> {
		self.data.remove(key)
	}

	pub fn clear(&mut self) {
		self.data.clear();
	}
}

impl Default for MockSession {
	fn default() -> Self {
		Self::new()
	}
}

/// Test assertion helpers
pub mod assertions {
	use super::*;

	pub fn assert_cookie_exists(response: &MockResponse, cookie_name: &str) {
		assert!(
			response.get_cookie(cookie_name).is_some(),
			"Expected cookie '{}' to exist in response",
			cookie_name
		);
	}

	pub fn assert_cookie_value(response: &MockResponse, cookie_name: &str, expected: &str) {
		let value = response
			.get_cookie(cookie_name)
			.expect(&format!("Cookie '{}' not found", cookie_name));
		assert_eq!(value, expected, "Cookie '{}' value mismatch", cookie_name);
	}

	pub fn assert_session_key_exists(session: &MockSession, key: &str) {
		assert!(
			session.get(key).is_some(),
			"Expected session key '{}' to exist",
			key
		);
	}

	pub fn assert_session_value(session: &MockSession, key: &str, expected: &str) {
		let value = session
			.get(key)
			.expect(&format!("Session key '{}' not found", key));
		assert_eq!(value, expected, "Session key '{}' value mismatch", key);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_mock_request() {
		let mut request = MockRequest::new();
		assert!(request.cookies.is_empty());

		request.set_cookie("test", "value");
		assert_eq!(request.get_cookie("test"), Some(&"value".to_string()));
	}

	#[test]
	fn test_mock_request_with_session() {
		let request = MockRequest::new().with_session();
		assert!(request.session().is_some());
	}

	#[test]
	fn test_mock_response() {
		let mut response = MockResponse::new();
		assert_eq!(response.status, 200);

		response.set_cookie("message", "test");
		assert_eq!(response.get_cookie("message"), Some(&"test".to_string()));
	}

	#[test]
	fn test_mock_session() {
		let mut session = MockSession::new();
		session.set("key", "value");

		assert_eq!(session.get("key"), Some(&"value".to_string()));

		session.remove("key");
		assert!(session.get("key").is_none());
	}
}
