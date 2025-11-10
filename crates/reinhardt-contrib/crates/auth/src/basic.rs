//! HTTP Basic Authentication

use crate::drf_authentication::Authentication;
use crate::{AuthenticationBackend, AuthenticationError, SimpleUser, User};
use base64::{Engine, engine::general_purpose::STANDARD};
use reinhardt_core::types::Request;
use std::collections::HashMap;
use uuid::Uuid;

/// Basic Authentication backend
pub struct BasicAuthentication {
	users: HashMap<String, String>, // username -> password
}

impl BasicAuthentication {
	/// Creates a new BasicAuthentication backend with no users.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{HttpBasicAuth, AuthenticationBackend};
	/// use bytes::Bytes;
	/// use hyper::{HeaderMap, Method, Uri, Version};
	/// use reinhardt_core::types::Request;
	///
	/// # async fn example() {
	/// let auth = HttpBasicAuth::new();
	///
	// Create a request without authentication header
	/// let request = Request::new(
	///     Method::GET,
	///     Uri::from_static("/"),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new(),
	/// );
	///
	// Since no users are registered, authentication should return None
	/// let result = auth.authenticate(&request).await.unwrap();
	/// assert!(result.is_none());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn new() -> Self {
		Self {
			users: HashMap::new(),
		}
	}
	/// Adds a user with the given username and password.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{HttpBasicAuth, AuthenticationBackend};
	/// use bytes::Bytes;
	/// use hyper::{HeaderMap, Method, Uri, Version};
	/// use reinhardt_core::types::Request;
	///
	/// # async fn example() {
	/// let mut auth = HttpBasicAuth::new();
	/// auth.add_user("alice", "secret123");
	/// auth.add_user("bob", "password456");
	///
	/// // Create a request with valid Basic auth credentials
	/// // "alice:secret123" in base64 is "YWxpY2U6c2VjcmV0MTIz"
	/// let mut headers = HeaderMap::new();
	/// headers.insert("Authorization", "Basic YWxpY2U6c2VjcmV0MTIz".parse().unwrap());
	/// let request = Request::new(
	///     Method::GET,
	///     Uri::from_static("/"),
	///     Version::HTTP_11,
	///     headers,
	///     Bytes::new(),
	/// );
	///
	/// // Authentication should succeed
	/// let result = auth.authenticate(&request).await.unwrap();
	/// assert!(result.is_some());
	/// assert_eq!(result.unwrap().get_username(), "alice");
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn add_user(&mut self, username: impl Into<String>, password: impl Into<String>) {
		self.users.insert(username.into(), password.into());
	}

	/// Parse Authorization header
	fn parse_auth_header(&self, header: &str) -> Option<(String, String)> {
		if !header.starts_with("Basic ") {
			return None;
		}

		let encoded = header.strip_prefix("Basic ")?;
		let decoded = STANDARD.decode(encoded).ok()?;
		let decoded_str = String::from_utf8(decoded).ok()?;

		let parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
		if parts.len() != 2 {
			return None;
		}

		Some((parts[0].to_string(), parts[1].to_string()))
	}
}

impl Default for BasicAuthentication {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl AuthenticationBackend for BasicAuthentication {
	async fn authenticate(
		&self,
		request: &Request,
	) -> Result<Option<Box<dyn User>>, AuthenticationError> {
		let auth_header = request
			.headers
			.get("Authorization")
			.and_then(|h| h.to_str().ok());

		if let Some(header) = auth_header
			&& let Some((username, password)) = self.parse_auth_header(header)
		{
			if let Some(stored_password) = self.users.get(&username)
				&& stored_password == &password
			{
				return Ok(Some(Box::new(SimpleUser {
					id: Uuid::new_v4(),
					username: username.clone(),
					email: format!("{}@example.com", username),
					is_active: true,
					is_admin: false,
					is_staff: false,
					is_superuser: false,
				})));
			}
			return Err(AuthenticationError::InvalidCredentials);
		}

		Ok(None)
	}

	async fn get_user(&self, user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError> {
		if self.users.contains_key(user_id) {
			Ok(Some(Box::new(SimpleUser {
				id: Uuid::new_v4(),
				username: user_id.to_string(),
				email: format!("{}@example.com", user_id),
				is_active: true,
				is_admin: false,
				is_staff: false,
				is_superuser: false,
			})))
		} else {
			Ok(None)
		}
	}
}

// Implement DRF-style Authentication trait by forwarding to AuthenticationBackend
#[async_trait::async_trait]
impl Authentication for BasicAuthentication {
	async fn authenticate(
		&self,
		request: &Request,
	) -> Result<Option<Box<dyn User>>, AuthenticationError> {
		// Forward to AuthenticationBackend implementation
		AuthenticationBackend::authenticate(self, request).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Uri, Version};

	fn create_request_with_auth(auth: &str) -> Request {
		let mut headers = HeaderMap::new();
		headers.insert("Authorization", auth.parse().unwrap());
		Request::new(
			Method::GET,
			Uri::from_static("/"),
			Version::HTTP_11,
			headers,
			Bytes::new(),
		)
	}

	#[tokio::test]
	async fn test_basic_auth_success() {
		let mut backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		// Base64 encode "testuser:testpass"
		let auth = "Basic dGVzdHVzZXI6dGVzdHBhc3M=";
		let request = create_request_with_auth(auth);

		let result = AuthenticationBackend::authenticate(&backend, &request)
			.await
			.unwrap();
		assert!(result.is_some());
		assert_eq!(result.unwrap().get_username(), "testuser");
	}

	#[tokio::test]
	async fn test_basic_auth_invalid_password() {
		let mut backend = BasicAuthentication::new();
		backend.add_user("testuser", "correctpass");

		// Base64 encode "testuser:wrongpass"
		let auth = "Basic dGVzdHVzZXI6d3JvbmdwYXNz";
		let request = create_request_with_auth(auth);

		let result = AuthenticationBackend::authenticate(&backend, &request).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_basic_auth_no_header() {
		let backend = BasicAuthentication::new();
		let request = Request::new(
			Method::GET,
			Uri::from_static("/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let result = AuthenticationBackend::authenticate(&backend, &request)
			.await
			.unwrap();
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_auth_header() {
		let backend = BasicAuthentication::new();

		let (user, pass) = backend.parse_auth_header("Basic dGVzdDpwYXNz").unwrap();
		assert_eq!(user, "test");
		assert_eq!(pass, "pass");
	}

	#[tokio::test]
	async fn test_get_user() {
		let mut backend = BasicAuthentication::new();
		backend.add_user("testuser", "testpass");

		let user = backend.get_user("testuser").await.unwrap();
		assert!(user.is_some());
		assert_eq!(user.unwrap().get_username(), "testuser");

		let no_user = backend.get_user("nonexistent").await.unwrap();
		assert!(no_user.is_none());
	}
}
