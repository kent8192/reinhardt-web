//! API Client for testing
//!
//! Similar to DRF's APIClient, provides methods for making test requests
//! with authentication, cookies, and headers support.

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::response::TestResponse;

#[derive(Debug, Error)]
pub enum ClientError {
	#[error("HTTP error: {0}")]
	Http(#[from] http::Error),

	#[error("Hyper error: {0}")]
	Hyper(#[from] hyper::Error),

	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),

	#[error("Invalid header value: {0}")]
	InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),

	#[error("Request failed: {0}")]
	RequestFailed(String),
}

pub type ClientResult<T> = Result<T, ClientError>;

/// Type alias for request handler function
pub type RequestHandler = Arc<dyn Fn(Request<Full<Bytes>>) -> Response<Full<Bytes>> + Send + Sync>;

/// Test client for making API requests
///
/// # Example
/// ```ignore
/// let client = APIClient::new();
/// client.login("username", "password").await?;
/// let response = client.get("/api/users/").await?;
/// assert_eq!(response.status(), StatusCode::OK);
/// ```
pub struct APIClient {
	/// Base URL for requests (e.g., "http://testserver")
	base_url: String,

	/// Default headers to include in all requests
	default_headers: Arc<RwLock<HeaderMap>>,

	/// Cookies to include in requests
	cookies: Arc<RwLock<HashMap<String, String>>>,

	/// Current authenticated user (if any)
	user: Arc<RwLock<Option<Value>>>,

	/// Handler function for processing requests
	handler: Option<RequestHandler>,
}

impl APIClient {
	/// Create a new API client
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// let client = APIClient::new();
	/// assert_eq!(client.base_url(), "http://testserver");
	/// ```
	pub fn new() -> Self {
		Self {
			base_url: "http://testserver".to_string(),
			default_headers: Arc::new(RwLock::new(HeaderMap::new())),
			cookies: Arc::new(RwLock::new(HashMap::new())),
			user: Arc::new(RwLock::new(None)),
			handler: None,
		}
	}
	/// Create a client with a custom base URL
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// let client = APIClient::with_base_url("https://api.example.com");
	/// assert_eq!(client.base_url(), "https://api.example.com");
	/// ```
	pub fn with_base_url(base_url: impl Into<String>) -> Self {
		Self {
			base_url: base_url.into(),
			default_headers: Arc::new(RwLock::new(HeaderMap::new())),
			cookies: Arc::new(RwLock::new(HashMap::new())),
			user: Arc::new(RwLock::new(None)),
			handler: None,
		}
	}
	pub fn base_url(&self) -> &str {
		&self.base_url
	}
	/// Set a request handler for testing
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	/// use http::{Request, Response, StatusCode};
	/// use http_body_util::Full;
	/// use bytes::Bytes;
	///
	/// let mut client = APIClient::new();
	/// client.set_handler(|_req| {
	///     Response::builder()
	///         .status(StatusCode::OK)
	///         .body(Full::new(Bytes::from("test")))
	///         .unwrap()
	/// });
	/// ```
	pub fn set_handler<F>(&mut self, handler: F)
	where
		F: Fn(Request<Full<Bytes>>) -> Response<Full<Bytes>> + Send + Sync + 'static,
	{
		self.handler = Some(Arc::new(handler));
	}
	/// Set a default header for all requests
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// client.set_header("User-Agent", "TestClient/1.0").await.unwrap();
	/// # });
	/// ```
	pub async fn set_header(
		&self,
		name: impl AsRef<str>,
		value: impl AsRef<str>,
	) -> ClientResult<()> {
		let mut headers = self.default_headers.write().await;
		let header_name: http::header::HeaderName = name.as_ref().parse().map_err(|_| {
			ClientError::RequestFailed(format!("Invalid header name: {}", name.as_ref()))
		})?;
		headers.insert(header_name, HeaderValue::from_str(value.as_ref())?);
		Ok(())
	}
	/// Force authenticate as a user (for testing)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// let user = json!({"id": 1, "username": "testuser"});
	/// client.force_authenticate(Some(user)).await;
	/// # });
	/// ```
	pub async fn force_authenticate(&self, user: Option<Value>) {
		let mut current_user = self.user.write().await;
		*current_user = user;
	}
	/// Set credentials for Basic Authentication
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// client.credentials("username", "password").await.unwrap();
	/// # });
	/// ```
	pub async fn credentials(&self, username: &str, password: &str) -> ClientResult<()> {
		let encoded = base64::encode(format!("{}:{}", username, password));
		self.set_header("Authorization", format!("Basic {}", encoded))
			.await
	}
	/// Login with username and password
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// let response = client.login("username", "password").await;
	/// # });
	/// ```
	pub async fn login(&self, username: &str, password: &str) -> ClientResult<TestResponse> {
		let body = serde_json::json!({
			"username": username,
			"password": password,
		});
		self.post("/api/login/", &body, "json").await
	}
	/// Logout and clear authentication
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// client.logout().await.unwrap();
	/// # });
	/// ```
	pub async fn logout(&self) -> ClientResult<()> {
		self.force_authenticate(None).await;
		let mut cookies = self.cookies.write().await;
		cookies.clear();
		Ok(())
	}
	/// Make a GET request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: get() requires a working handler
	// let response = client.get("/api/users/").await;
	/// # });
	/// ```
	pub async fn get(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::GET, path, None, None).await
	}
	/// Make a POST request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// let data = json!({"name": "test"});
	// Note: post() requires a working handler
	// let response = client.post("/api/users/", &data, "json").await;
	/// # });
	/// ```
	pub async fn post<T: Serialize>(
		&self,
		path: &str,
		data: &T,
		format: &str,
	) -> ClientResult<TestResponse> {
		let body = self.serialize_data(data, format)?;
		let content_type = self.get_content_type(format);
		self.request(Method::POST, path, Some(body), Some(content_type))
			.await
	}
	/// Make a PUT request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// let data = json!({"name": "updated"});
	// Note: put() requires a working handler
	// let response = client.put("/api/users/1/", &data, "json").await;
	/// # });
	/// ```
	pub async fn put<T: Serialize>(
		&self,
		path: &str,
		data: &T,
		format: &str,
	) -> ClientResult<TestResponse> {
		let body = self.serialize_data(data, format)?;
		let content_type = self.get_content_type(format);
		self.request(Method::PUT, path, Some(body), Some(content_type))
			.await
	}
	/// Make a PATCH request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	/// use serde_json::json;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// let data = json!({"name": "partial_update"});
	// Note: patch() requires a working handler
	// let response = client.patch("/api/users/1/", &data, "json").await;
	/// # });
	/// ```
	pub async fn patch<T: Serialize>(
		&self,
		path: &str,
		data: &T,
		format: &str,
	) -> ClientResult<TestResponse> {
		let body = self.serialize_data(data, format)?;
		let content_type = self.get_content_type(format);
		self.request(Method::PATCH, path, Some(body), Some(content_type))
			.await
	}
	/// Make a DELETE request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: delete() requires a working handler
	// let response = client.delete("/api/users/1/").await;
	/// # });
	/// ```
	pub async fn delete(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::DELETE, path, None, None).await
	}
	/// Make a HEAD request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: head() requires a working handler
	// let response = client.head("/api/users/").await;
	/// # });
	/// ```
	pub async fn head(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::HEAD, path, None, None).await
	}
	/// Make an OPTIONS request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_test::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: options() requires a working handler
	// let response = client.options("/api/users/").await;
	/// # });
	/// ```
	pub async fn options(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::OPTIONS, path, None, None).await
	}

	/// Generic request method
	async fn request(
		&self,
		method: Method,
		path: &str,
		body: Option<Bytes>,
		content_type: Option<&str>,
	) -> ClientResult<TestResponse> {
		let url = if path.starts_with("http://") || path.starts_with("https://") {
			path.to_string()
		} else {
			format!("{}{}", self.base_url, path)
		};

		let mut req_builder = Request::builder().method(method).uri(url);

		// Add default headers
		let default_headers = self.default_headers.read().await;
		for (name, value) in default_headers.iter() {
			req_builder = req_builder.header(name, value);
		}

		// Add content type if provided
		if let Some(ct) = content_type {
			req_builder = req_builder.header("Content-Type", ct);
		}

		// Add cookies
		let cookies = self.cookies.read().await;
		if !cookies.is_empty() {
			let cookie_header = cookies
				.iter()
				.map(|(k, v)| format!("{}={}", k, v))
				.collect::<Vec<_>>()
				.join("; ");
			req_builder = req_builder.header("Cookie", cookie_header);
		}

		// Add authentication if user is set
		let user = self.user.read().await;
		if user.is_some() {
			// Add custom header to indicate forced authentication
			req_builder = req_builder.header("X-Test-User", "authenticated");
		}

		// Build request with body
		let request = if let Some(body_bytes) = body {
			req_builder.body(Full::new(body_bytes))?
		} else {
			req_builder.body(Full::new(Bytes::new()))?
		};

		// Execute request
		let response = if let Some(handler) = &self.handler {
			handler(request)
		} else {
			// Default mock response for testing without a handler
			Response::builder()
				.status(StatusCode::NOT_IMPLEMENTED)
				.body(Full::new(Bytes::from("No handler set")))?
		};

		// Extract body from response using async collection
		let (parts, response_body) = response.into_parts();
		let body_data = response_body
			.collect()
			.await
			.map(|collected| collected.to_bytes())
			.unwrap_or_else(|_| Bytes::new());

		Ok(TestResponse::with_body(
			parts.status,
			parts.headers,
			body_data,
		))
	}

	/// Serialize data based on format
	fn serialize_data<T: Serialize>(&self, data: &T, format: &str) -> ClientResult<Bytes> {
		match format {
			"json" => {
				let json = serde_json::to_vec(data)?;
				Ok(Bytes::from(json))
			}
			"form" => {
				// URL-encoded form data
				let json_value = serde_json::to_value(data)?;
				if let Value::Object(map) = json_value {
					let form_data = map
						.iter()
						.map(|(k, v)| {
							let value_str = match v {
								Value::String(s) => s.clone(),
								_ => v.to_string(),
							};
							format!(
								"{}={}",
								urlencoding::encode(k),
								urlencoding::encode(&value_str)
							)
						})
						.collect::<Vec<_>>()
						.join("&");
					Ok(Bytes::from(form_data))
				} else {
					Err(ClientError::RequestFailed(
						"Expected object for form data".to_string(),
					))
				}
			}
			_ => Err(ClientError::RequestFailed(format!(
				"Unsupported format: {}",
				format
			))),
		}
	}

	/// Get content type for format
	fn get_content_type(&self, format: &str) -> &str {
		match format {
			"json" => "application/json",
			"form" => "application/x-www-form-urlencoded",
			_ => "application/octet-stream",
		}
	}
}

impl Default for APIClient {
	fn default() -> Self {
		Self::new()
	}
}

// Need to add base64 dependency
mod base64 {
	pub fn encode(input: String) -> String {
		// Simple base64 encoding (in production, use a proper library)
		use base64_simd::STANDARD;
		STANDARD.encode_to_string(input.as_bytes())
	}
}

// Need to add urlencoding
mod urlencoding {
	pub fn encode(input: &str) -> String {
		url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
	}
}
