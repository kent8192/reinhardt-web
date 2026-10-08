//! API Client for testing
//!
//! Similar to DRF's APIClient, provides methods for making test requests
//! with authentication, cookies, and headers support.

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, Request, Response, header::HeaderName};
use http_body_util::{BodyExt, Full};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

use reinhardt_di::InjectionContext;
use reinhardt_http::{Handler as HttpHandler, Request as HttpRequest, Response as HttpResponse};

use crate::response::TestResponse;

/// HTTP version configuration for APIClient
#[derive(Debug, Clone, Copy, Default)]
pub enum HttpVersion {
	/// Use HTTP/1.1 only
	Http1Only,
	/// Use HTTP/2 with prior knowledge (no upgrade negotiation)
	Http2PriorKnowledge,
	/// Auto-negotiate (default)
	#[default]
	Auto,
}

/// Errors that can occur when using the API test client.
#[derive(Debug, Error)]
pub enum ClientError {
	/// HTTP protocol error.
	#[error("HTTP error: {0}")]
	Http(#[from] http::Error),

	/// Hyper transport error.
	#[error("Hyper error: {0}")]
	Hyper(#[from] hyper::Error),

	/// JSON serialization/deserialization error.
	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),

	/// Invalid HTTP header value.
	#[error("Invalid header value: {0}")]
	InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),

	/// Reqwest HTTP client error.
	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),

	/// General request failure.
	#[error("Request failed: {0}")]
	RequestFailed(String),
}

impl ClientError {
	/// Returns true if the error is a timeout error
	pub fn is_timeout(&self) -> bool {
		match self {
			ClientError::Reqwest(e) => e.is_timeout(),
			_ => false,
		}
	}

	/// Returns true if the error is a connection error
	pub fn is_connect(&self) -> bool {
		match self {
			ClientError::Reqwest(e) => e.is_connect(),
			_ => false,
		}
	}

	/// Returns true if the error occurred during request building
	pub fn is_request(&self) -> bool {
		match self {
			ClientError::Reqwest(e) => e.is_request(),
			ClientError::Http(_) => true,
			ClientError::InvalidHeaderValue(_) => true,
			ClientError::Serialization(_) => true,
			ClientError::RequestFailed(_) => true,
			_ => false,
		}
	}
}

/// Result type for API client operations.
pub type ClientResult<T> = Result<T, ClientError>;

/// Type alias for request handler function
pub type RequestHandler = Arc<dyn Fn(Request<Full<Bytes>>) -> Response<Full<Bytes>> + Send + Sync>;

/// Builder for creating APIClient with custom configuration
///
/// # Example
/// ```rust,no_run
/// use reinhardt_testkit::client::{APIClientBuilder, HttpVersion};
/// use std::time::Duration;
///
/// let client = APIClientBuilder::new()
///     .base_url("http://localhost:8080")
///     .timeout(Duration::from_secs(30))
///     .http_version(HttpVersion::Http2PriorKnowledge)
///     .cookie_store(true)
///     .build();
/// ```
pub struct APIClientBuilder {
	base_url: String,
	timeout: Option<Duration>,
	http_version: HttpVersion,
	cookie_store: bool,
	framework_handler: Option<Arc<dyn HttpHandler>>,
	di_context: Option<Arc<InjectionContext>>,
}

impl APIClientBuilder {
	/// Create a new builder with default configuration
	pub fn new() -> Self {
		Self {
			base_url: "http://testserver".to_string(),
			timeout: None,
			http_version: HttpVersion::Auto,
			cookie_store: false,
			framework_handler: None,
			di_context: None,
		}
	}

	/// Set the base URL for requests
	pub fn base_url(mut self, url: impl Into<String>) -> Self {
		self.base_url = url.into();
		self
	}

	/// Set the request timeout
	pub fn timeout(mut self, duration: Duration) -> Self {
		self.timeout = Some(duration);
		self
	}

	/// Set the HTTP version
	pub fn http_version(mut self, version: HttpVersion) -> Self {
		self.http_version = version;
		self
	}

	/// Use HTTP/1.1 only (convenience method)
	pub fn http1_only(mut self) -> Self {
		self.http_version = HttpVersion::Http1Only;
		self
	}

	/// Use HTTP/2 with prior knowledge (convenience method)
	pub fn http2_prior_knowledge(mut self) -> Self {
		self.http_version = HttpVersion::Http2PriorKnowledge;
		self
	}

	/// Enable or disable automatic cookie storage
	pub fn cookie_store(mut self, enabled: bool) -> Self {
		self.cookie_store = enabled;
		self
	}

	/// Set a reinhardt `Handler` for in-process request dispatching.
	///
	/// When set, requests bypass the network and are handled directly
	/// by the given Handler, running the full middleware stack in-process.
	///
	/// The calling test must run inside a tokio runtime (e.g., `#[tokio::test]`).
	pub fn handler(mut self, handler: impl HttpHandler + 'static) -> Self {
		self.framework_handler = Some(Arc::new(handler));
		self
	}

	/// Set a DI context for in-process handler requests.
	///
	/// The context is injected into every reinhardt `Request` before
	/// dispatching to the Handler.
	pub fn di_context(mut self, ctx: Arc<InjectionContext>) -> Self {
		self.di_context = Some(ctx);
		self
	}

	/// Build the APIClient
	pub fn build(self) -> APIClient {
		let mut client_builder = reqwest::Client::builder();

		// Configure timeout
		if let Some(timeout) = self.timeout {
			client_builder = client_builder.timeout(timeout);
		}

		// Configure HTTP version
		match self.http_version {
			HttpVersion::Http1Only => {
				client_builder = client_builder.http1_only();
			}
			HttpVersion::Http2PriorKnowledge => {
				client_builder = client_builder.http2_prior_knowledge();
			}
			HttpVersion::Auto => {
				// Default behavior, no special configuration needed
			}
		}

		// Configure cookie store
		if self.cookie_store {
			client_builder = client_builder.cookie_store(true);
		}

		let http_client = client_builder
			.build()
			.expect("Failed to build reqwest client");

		let mut client = APIClient {
			base_url: self.base_url,
			default_headers: Arc::new(RwLock::new(HeaderMap::new())),
			cookies: Arc::new(RwLock::new(HashMap::new())),
			user: Arc::new(RwLock::new(None)),
			handler: None,
			async_handler: None,
			handler_di_context: None,
			http_client,
		};

		// Wire up framework Handler for in-process dispatch
		if let Some(fw_handler) = self.framework_handler {
			client.async_handler = Some(fw_handler);
			client.handler_di_context = self.di_context;

			// Set default Origin header for OriginGuardMiddleware compatibility
			if let Ok(mut headers) = client.default_headers.try_write()
				&& let Ok(origin) = HeaderValue::from_str(&client.base_url)
			{
				headers.insert(http::header::ORIGIN, origin);
			}
		}

		client
	}
}

impl Default for APIClientBuilder {
	fn default() -> Self {
		Self::new()
	}
}

/// Test client for making API requests
///
/// # Example
/// ```rust,no_run
/// use reinhardt_testkit::APIClient;
/// use http::StatusCode;
/// use serde_json::json;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = APIClient::with_base_url("http://localhost:8080");
/// let credentials = json!({"username": "user", "password": "pass"});
/// client.post("/auth/login", &credentials, "json").await?;
/// let response = client.get("/api/users/").await?;
/// assert_eq!(response.status(), StatusCode::OK);
/// # Ok(())
/// # }
/// ```
pub struct APIClient {
	/// Base URL for requests (e.g., "http://testserver")
	base_url: String,

	/// Default headers to include in all requests
	default_headers: Arc<RwLock<HeaderMap>>,

	/// Cookies to include in requests (manual management)
	cookies: Arc<RwLock<HashMap<String, String>>>,

	/// Current authenticated user (if any)
	user: Arc<RwLock<Option<Value>>>,

	/// Handler function for processing requests (sync, for set_handler)
	handler: Option<RequestHandler>,

	/// In-process async handler for framework Handler trait dispatch
	async_handler: Option<Arc<dyn HttpHandler>>,

	/// DI context injected into requests when using async_handler
	handler_di_context: Option<Arc<InjectionContext>>,

	/// Reusable HTTP client with connection pooling
	http_client: reqwest::Client,
}

impl APIClient {
	/// Create a new API client
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// let client = APIClient::new();
	/// assert_eq!(client.base_url(), "http://testserver");
	/// ```
	pub fn new() -> Self {
		APIClientBuilder::new().build()
	}

	/// Create a client with a custom base URL
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// let client = APIClient::with_base_url("https://api.example.com");
	/// assert_eq!(client.base_url(), "https://api.example.com");
	/// ```
	pub fn with_base_url(base_url: impl Into<String>) -> Self {
		APIClientBuilder::new().base_url(base_url).build()
	}

	/// Create a test client that dispatches requests directly to a
	/// reinhardt `Handler` without TCP.
	///
	/// The Handler runs the full middleware stack in-process.
	/// Sets `base_url` to `"http://testserver"` and injects a default
	/// `Origin` header for `OriginGuardMiddleware` compatibility.
	///
	/// # Panics
	///
	/// Panics if called outside a tokio runtime.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_testkit::APIClient;
	///
	/// // let router = build_routes(scope).into_server();
	/// // let client = APIClient::from_handler(router);
	/// // let resp = client.get("/api/health/").await.unwrap();
	/// ```
	///
	/// File bodies and finite streaming bodies are collected into the test response.
	/// A streaming producer error is returned as `ClientError::RequestFailed`.
	pub fn from_handler(handler: impl HttpHandler + 'static) -> Self {
		APIClientBuilder::new().handler(handler).build()
	}

	/// Create a builder for customizing the client configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	/// use std::time::Duration;
	///
	/// let client = APIClient::builder()
	///     .base_url("http://localhost:8080")
	///     .timeout(Duration::from_secs(30))
	///     .build();
	/// ```
	pub fn builder() -> APIClientBuilder {
		APIClientBuilder::new()
	}
	/// Get the base URL of this client.
	pub fn base_url(&self) -> &str {
		&self.base_url
	}
	/// Set a request handler for testing
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
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
	/// use reinhardt_testkit::client::APIClient;
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
	/// Set credentials for Basic Authentication
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
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
	/// Clear authentication and cookies
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// client.clear_auth().await.unwrap();
	/// # });
	/// ```
	pub async fn clear_auth(&self) -> ClientResult<()> {
		{
			let mut current_user = self.user.write().await;
			*current_user = None;
		}
		let mut cookies = self.cookies.write().await;
		cookies.clear();
		drop(cookies);
		// Clear auth-related headers (Authorization, X-MFA-Code, X-Test-User)
		let mut headers = self.default_headers.write().await;
		headers.remove("authorization");
		headers.remove("x-mfa-code");
		headers.remove("x-test-user");
		Ok(())
	}

	/// Set a cookie that will be sent with subsequent requests.
	///
	/// # Panics
	///
	/// Panics if `name` contains `=` or `;`, or if `value` contains `;`.
	pub async fn set_cookie(&self, name: &str, value: &str) -> ClientResult<()> {
		validate_cookie_key(name);
		validate_cookie_value(value);
		let mut cookies = self.cookies.write().await;
		cookies.insert(name.to_string(), value.to_string());
		Ok(())
	}

	/// Remove a specific cookie.
	pub async fn remove_cookie(&self, name: &str) -> ClientResult<()> {
		let mut cookies = self.cookies.write().await;
		cookies.remove(name);
		Ok(())
	}

	/// Clear all authentication state (session cookies, auth headers, stored user).
	///
	/// Clears the current authenticated user and all related authentication state.
	pub async fn logout(&self) -> ClientResult<()> {
		self.clear_auth().await
	}

	/// Start building an auth configuration for this client.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// client.auth()
	///     .session(&user, &session_store)
	///     .with_staff(true)
	///     .apply().await?;
	/// ```
	#[cfg(native)]
	pub fn auth(&self) -> crate::auth::AuthBuilder<'_> {
		crate::auth::AuthBuilder::new(self)
	}

	/// Clean up all client state for teardown
	///
	/// This method performs a complete cleanup of the client state including:
	/// - Clearing authentication
	/// - Clearing cookies
	/// - Clearing default headers
	///
	/// This is typically called during test teardown to ensure clean state
	/// between tests.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	/// client.set_header("X-Custom", "value").await.unwrap();
	/// client.cleanup().await;
	/// // All state is now cleared
	/// # });
	/// ```
	pub async fn cleanup(&self) {
		// Clear authentication
		{
			let mut current_user = self.user.write().await;
			*current_user = None;
		}

		// Clear cookies
		{
			let mut cookies = self.cookies.write().await;
			cookies.clear();
		}

		// Clear default headers
		{
			let mut headers = self.default_headers.write().await;
			headers.clear();
		}
	}
	/// Build a request with per-request headers and a body for any HTTP method.
	///
	/// Header operations apply in call order after default headers, the implied
	/// content type, manual cookies, and forced authentication. They affect only
	/// this request. Conversion and serialization errors are returned by `send()`.
	/// Cookies added by reqwest's automatic cookie jar at send time cannot be
	/// removed per request. Native-only (P0) through `reinhardt::test`.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// let response = client.request(Method::PUT, "/users/1")
	///     .header("Authorization", "Bearer alice")
	///     .json(&serde_json::json!({"name": "Alice"}))
	///     .send().await.unwrap();
	/// assert_eq!(response.headers()["authorization"], "Bearer alice");
	/// # });
	/// ```
	pub fn request(&self, method: Method, path: &str) -> TestRequestBuilder<'_> {
		TestRequestBuilder {
			client: self,
			method,
			path: path.to_owned(),
			body: Bytes::new(),
			content_type: None,
			headers: Vec::new(),
			error: None,
		}
	}

	/// Make a GET request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: get() requires a working handler
	// let response = client.get("/api/users/").await;
	/// # });
	/// ```
	pub async fn get(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::GET, path).send().await
	}
	/// Make a POST request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
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
		self.request(Method::POST, path)
			.serialized(data, format)
			.send()
			.await
	}
	/// Make a PUT request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
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
		self.request(Method::PUT, path)
			.serialized(data, format)
			.send()
			.await
	}
	/// Make a PATCH request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
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
		self.request(Method::PATCH, path)
			.serialized(data, format)
			.send()
			.await
	}
	/// Make a DELETE request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: delete() requires a working handler
	// let response = client.delete("/api/users/1/").await;
	/// # });
	/// ```
	pub async fn delete(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::DELETE, path).send().await
	}
	/// Make a HEAD request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: head() requires a working handler
	// let response = client.head("/api/users/").await;
	/// # });
	/// ```
	pub async fn head(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::HEAD, path).send().await
	}
	/// Make an OPTIONS request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::new();
	// Note: options() requires a working handler
	// let response = client.options("/api/users/").await;
	/// # });
	/// ```
	pub async fn options(&self, path: &str) -> ClientResult<TestResponse> {
		self.request(Method::OPTIONS, path).send().await
	}

	/// Make a GET request with additional per-request headers
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::with_base_url("http://localhost:8080");
	/// // let response = client.request(Method::GET, "/api/data")
	/// //     .header("Accept", "application/json").send().await;
	/// # });
	/// ```
	/// Deprecated in 0.4.0; removal is planned for 0.5.
	#[deprecated(since = "0.4.0", note = "use APIClient::request(..).header(..).send()")]
	pub async fn get_with_headers(
		&self,
		path: &str,
		headers: &[(&str, &str)],
	) -> ClientResult<TestResponse> {
		let mut request = self.request(Method::GET, path);
		for (name, value) in headers {
			request = request.header(*name, *value);
		}
		request.send().await
	}

	/// Make a POST request with raw body and additional per-request headers
	///
	/// Unlike `post()`, this method allows setting a raw body without automatic serialization.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::with_base_url("http://localhost:8080");
	/// // let response = client.request(Method::POST, "/api/echo")
	/// //     .body(bytes::Bytes::from_static(b"raw"))
	/// //     .header("Content-Type", "application/json")
	/// //     .header("X-Custom-Header", "value").send().await;
	/// # });
	/// ```
	/// Deprecated in 0.4.0; removal is planned for 0.5.
	#[deprecated(since = "0.4.0", note = "use APIClient::request(..).header(..).send()")]
	pub async fn post_raw_with_headers(
		&self,
		path: &str,
		body: &[u8],
		content_type: &str,
		headers: &[(&str, &str)],
	) -> ClientResult<TestResponse> {
		let mut request = self
			.request(Method::POST, path)
			.body(Bytes::copy_from_slice(body))
			.header(http::header::CONTENT_TYPE, content_type);
		for (name, value) in headers {
			request = request.header(*name, *value);
		}
		request.send().await
	}

	/// Make a POST request with raw body
	///
	/// Unlike `post()`, this method allows setting a raw body without automatic serialization.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_testkit::client::APIClient;
	///
	/// # tokio_test::block_on(async {
	/// let client = APIClient::with_base_url("http://localhost:8080");
	/// // let response = client.post_raw("/api/echo", b"{\"test\":\"data\"}", "application/json").await;
	/// # });
	/// ```
	pub async fn post_raw(
		&self,
		path: &str,
		body: &[u8],
		content_type: &str,
	) -> ClientResult<TestResponse> {
		self.request(Method::POST, path)
			.body(Bytes::copy_from_slice(body))
			.header(http::header::CONTENT_TYPE, content_type)
			.send()
			.await
	}

	/// Dispatch a fully assembled request through the configured handler or transport.
	async fn dispatch(&self, request: Request<Full<Bytes>>) -> ClientResult<TestResponse> {
		// Execute request
		let response = if let Some(async_handler) = &self.async_handler {
			// In-process dispatch via framework Handler trait
			let (parts, body) = request.into_parts();
			let body_bytes = body
				.collect()
				.await
				.map(|c| c.to_bytes())
				.unwrap_or_else(|_| Bytes::new());

			let mut fw_request = HttpRequest::builder()
				.method(parts.method)
				.uri(parts.uri)
				.version(parts.version)
				.headers(parts.headers)
				.body(body_bytes)
				.build()
				.expect("Failed to build reinhardt request");

			if let Some(ctx) = &self.handler_di_context {
				fw_request.set_di_context(Arc::clone(ctx));
			}

			let mut fw_response = async_handler
				.handle(fw_request)
				.await
				.unwrap_or_else(HttpResponse::from);

			let mut builder = http::Response::builder().status(fw_response.status);
			for (key, value) in fw_response.headers.iter() {
				builder = builder.header(key, value);
			}
			let body = if let Some(mut stream) = fw_response.take_stream_body() {
				use futures::StreamExt;
				let mut body = bytes::BytesMut::new();
				while let Some(chunk) = stream.next().await {
					let chunk =
						chunk.map_err(|error| ClientError::RequestFailed(error.to_string()))?;
					body.extend_from_slice(&chunk);
				}
				body.freeze()
			} else if let Some(source) = fw_response.file_body().cloned() {
				// The test client buffers its result, but file I/O must stay off the executor.
				tokio::task::spawn_blocking(move || {
					let mut body = bytes::BytesMut::new();
					let mut position = 0;
					while position < source.len() {
						let chunk = source.read_chunk(position, 64 * 1024)?;
						position += chunk.len() as u64;
						body.extend_from_slice(&chunk);
					}
					Ok::<_, std::io::Error>(body.freeze())
				})
				.await
				.map_err(|error| ClientError::RequestFailed(error.to_string()))?
				.map_err(|error| ClientError::RequestFailed(error.to_string()))?
			} else {
				fw_response.body
			};
			builder
				.body(Full::new(body))
				.expect("Failed to build http::Response")
		} else if let Some(handler) = &self.handler {
			// Use custom sync handler if set
			handler(request)
		} else {
			// Use reqwest for real HTTP requests when no handler is set
			let (parts, body) = request.into_parts();

			// Build reqwest request
			let url = if parts.uri.scheme_str().is_some() {
				// Absolute URL
				parts.uri.to_string()
			} else {
				// Relative path - use base_url
				format!(
					"{}{}",
					self.base_url.trim_end_matches('/'),
					parts.uri.path()
				)
			};

			// Use the stored http_client (connection pooling enabled)
			let mut reqwest_request = self.http_client.request(
				reqwest::Method::from_bytes(parts.method.as_str().as_bytes()).unwrap(),
				&url,
			);

			// Explicit headers take precedence over the automatic jar in reqwest.
			reqwest_request = reqwest_request.headers(parts.headers);

			// Copy body
			let body_bytes = body
				.collect()
				.await
				.map(|c| c.to_bytes())
				.unwrap_or_else(|_| Bytes::new());
			if !body_bytes.is_empty() {
				reqwest_request = reqwest_request.body(body_bytes.to_vec());
			}

			// Execute reqwest request
			let reqwest_response = reqwest_request.send().await?;

			// Convert reqwest response to http::Response
			let status = reqwest_response.status();
			let version = reqwest_response.version();
			let headers = reqwest_response.headers().clone();
			let body_bytes = reqwest_response.bytes().await?;

			let mut response_builder = Response::builder().status(status).version(version);
			for (name, value) in headers.iter() {
				response_builder = response_builder.header(name, value);
			}

			response_builder.body(Full::new(body_bytes))?
		};

		// Extract body from response using async collection
		let (parts, response_body) = response.into_parts();
		let body_data = response_body
			.collect()
			.await
			.map(|collected| collected.to_bytes())
			.unwrap_or_else(|_| Bytes::new());

		Ok(TestResponse::with_body_and_version(
			parts.status,
			parts.headers,
			body_data,
			parts.version,
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
	fn get_content_type(&self, format: &str) -> &'static str {
		match format {
			"json" => "application/json",
			"form" => "application/x-www-form-urlencoded",
			_ => "application/octet-stream",
		}
	}
}

/// Builds one request while borrowing its [`APIClient`].
///
/// Pair with [`TestResponse`] to configure and inspect requests for any HTTP method.
/// Chain methods are infallible: the first conversion or serialization error is
/// retained and returned by [`Self::send`]. Per-request header operations run last,
/// in call order, without changing client defaults. Reqwest's automatic jar cookies
/// added at send time cannot be removed per request.
/// Native-only (P0) through `reinhardt::test`.
///
/// # Examples
///
/// ```rust
/// use reinhardt_testkit::APIClient;
/// use http::Method;
/// # tokio_test::block_on(async {
/// # let mut client = APIClient::new();
/// # client.set_handler(|request| {
/// #     let (parts, body) = request.into_parts();
/// #     let mut response = http::Response::new(body);
/// #     *response.headers_mut() = parts.headers;
/// #     response
/// # });
/// let response = client.request(Method::PATCH, "/profile")
///     .without_header("Authorization")
///     .form(&serde_json::json!({"name": "Ada"}))
///     .send().await.unwrap();
/// assert_eq!(response.body().as_ref(), b"name=Ada");
/// # });
/// ```
pub struct TestRequestBuilder<'a> {
	client: &'a APIClient,
	method: Method,
	path: String,
	body: Bytes,
	content_type: Option<HeaderValue>,
	headers: Vec<HeaderOperation>,
	error: Option<ClientError>,
}

enum HeaderOperation {
	Replace(HeaderName, HeaderValue),
	Append(HeaderName, HeaderValue),
	Remove(HeaderName),
}

impl TestRequestBuilder<'_> {
	/// Replace all values for a header, including client-generated values.
	///
	/// Names are case-insensitive. Accepts strings and `http::header` constants.
	/// Invalid names or values are returned as [`ClientError::Http`] by `send()`.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// let response = client.request(Method::GET, "/")
	///     .header(http::header::ACCEPT, "application/json")
	///     .send().await.unwrap();
	/// assert_eq!(response.headers()["accept"], "application/json");
	/// # });
	/// ```
	pub fn header<K, V>(mut self, name: K, value: V) -> Self
	where
		K: TryInto<HeaderName>,
		K::Error: Into<http::Error>,
		V: TryInto<HeaderValue>,
		V::Error: Into<http::Error>,
	{
		self.add_header(name, value, HeaderOperation::Replace);
		self
	}

	/// Append a header value, retaining existing values in their original order.
	///
	/// Invalid names or values are returned as [`ClientError::Http`] by `send()`.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// let response = client.request(Method::GET, "/")
	///     .header("x-tag", "first").append_header("x-tag", "second")
	///     .send().await.unwrap();
	/// assert_eq!(response.headers().get_all("x-tag").iter().count(), 2);
	/// # });
	/// ```
	pub fn append_header<K, V>(mut self, name: K, value: V) -> Self
	where
		K: TryInto<HeaderName>,
		K::Error: Into<http::Error>,
		V: TryInto<HeaderValue>,
		V::Error: Into<http::Error>,
	{
		self.add_header(name, value, HeaderOperation::Append);
		self
	}

	/// Remove a header for this request, including manual cookies and forced auth.
	///
	/// Later `header()` or `append_header()` calls can add it again. This cannot
	/// suppress cookies added by reqwest's automatic cookie jar at send time.
	/// An invalid name is returned as [`ClientError::Http`] by `send()`.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// client.set_header("Authorization", "Bearer base").await.unwrap();
	/// let response = client.request(Method::GET, "/")
	///     .without_header(http::header::AUTHORIZATION).send().await.unwrap();
	/// assert_eq!(response.headers().get("authorization"), None);
	/// # });
	/// ```
	pub fn without_header<K>(mut self, name: K) -> Self
	where
		K: TryInto<HeaderName>,
		K::Error: Into<http::Error>,
	{
		if self.error.is_none() {
			match name.try_into() {
				Ok(name) => self.headers.push(HeaderOperation::Remove(name)),
				Err(error) => self.error = Some(ClientError::Http(error.into())),
			}
		}
		self
	}

	/// Serialize JSON immediately and imply `Content-Type: application/json`.
	///
	/// Serialization errors are returned by `send()`. Explicit header operations
	/// take precedence over the implied content type, regardless of call order.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// let response = client.request(Method::POST, "/")
	///     .json(&serde_json::json!({"id": 1})).send().await.unwrap();
	/// assert_eq!(response.headers()["content-type"], "application/json");
	/// # });
	/// ```
	pub fn json<T: Serialize>(self, data: &T) -> Self {
		self.serialized(data, "json")
	}

	/// Serialize URL-encoded form data immediately and imply its content type.
	///
	/// Uses the same object serialization as [`APIClient::post`] with `"form"`.
	/// Serialization errors are returned by `send()`.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// let response = client.request(Method::POST, "/")
	///     .form(&serde_json::json!({"name": "Ada Lovelace"}))
	///     .send().await.unwrap();
	/// assert_eq!(response.body().as_ref(), b"name=Ada+Lovelace");
	/// # });
	/// ```
	pub fn form<T: Serialize>(self, data: &T) -> Self {
		self.serialized(data, "form")
	}

	/// Set the raw body without setting or removing a content type.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// let response = client.request(Method::DELETE, "/")
	///     .body(bytes::Bytes::from_static(b"raw")).send().await.unwrap();
	/// assert_eq!(response.body().as_ref(), b"raw");
	/// assert_eq!(response.headers().get("content-type"), None);
	/// # });
	/// ```
	pub fn body(mut self, body: impl Into<Bytes>) -> Self {
		self.body = body.into();
		self
	}

	/// Assemble and dispatch the request, returning any stored builder error first.
	///
	/// Snapshots defaults without holding their locks while the handler or network
	/// runs. Applies the implied content type, manual cookies, forced auth, then
	/// each per-request header operation in call order.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_testkit::APIClient;
	/// use http::Method;
	/// # tokio_test::block_on(async {
	/// # let mut client = APIClient::new();
	/// # client.set_handler(|request| {
	/// #     let (parts, body) = request.into_parts();
	/// #     let mut response = http::Response::new(body);
	/// #     *response.headers_mut() = parts.headers;
	/// #     response
	/// # });
	/// let response = client.request(Method::GET, "/").send().await.unwrap();
	/// assert_eq!(response.status(), http::StatusCode::OK);
	/// # });
	/// ```
	pub async fn send(self) -> ClientResult<TestResponse> {
		if let Some(error) = self.error {
			return Err(error);
		}
		let mut headers = self.client.default_headers.read().await.clone();
		if let Some(content_type) = self.content_type {
			headers.insert(http::header::CONTENT_TYPE, content_type);
		}
		{
			let cookies = self.client.cookies.read().await;
			if !cookies.is_empty() {
				let cookie = cookies
					.iter()
					.map(|(key, value)| {
						validate_cookie_key(key);
						validate_cookie_value(value);
						format!("{key}={value}")
					})
					.collect::<Vec<_>>()
					.join("; ");
				headers.insert(http::header::COOKIE, HeaderValue::from_str(&cookie)?);
			}
		}
		if self.client.user.read().await.is_some() {
			headers.insert("x-test-user", HeaderValue::from_static("authenticated"));
		}
		for operation in self.headers {
			match operation {
				HeaderOperation::Replace(name, value) => {
					headers.insert(name, value);
				}
				HeaderOperation::Append(name, value) => {
					headers.append(name, value);
				}
				HeaderOperation::Remove(name) => {
					headers.remove(name);
				}
			}
		}
		let url = if self.path.starts_with("http://") || self.path.starts_with("https://") {
			self.path
		} else {
			format!("{}{}", self.client.base_url, self.path)
		};
		let mut request = Request::builder()
			.method(self.method)
			.uri(url)
			.body(Full::new(self.body))?;
		*request.headers_mut() = headers;
		self.client.dispatch(request).await
	}

	fn add_header<K, V>(
		&mut self,
		name: K,
		value: V,
		operation: impl FnOnce(HeaderName, HeaderValue) -> HeaderOperation,
	) where
		K: TryInto<HeaderName>,
		K::Error: Into<http::Error>,
		V: TryInto<HeaderValue>,
		V::Error: Into<http::Error>,
	{
		if self.error.is_none() {
			let header = name.try_into().map_err(Into::into).and_then(|name| {
				value
					.try_into()
					.map_err(Into::into)
					.map(|value| operation(name, value))
			});
			match header {
				Ok(header) => self.headers.push(header),
				Err(error) => self.error = Some(ClientError::Http(error)),
			}
		}
	}

	fn serialized<T: Serialize>(mut self, data: &T, format: &str) -> Self {
		if self.error.is_none() {
			match self.client.serialize_data(data, format) {
				Ok(body) => {
					self.body = body;
					self.content_type = Some(HeaderValue::from_static(
						self.client.get_content_type(format),
					));
				}
				Err(error) => self.error = Some(error),
			}
		}
		self
	}
}

/// Validate a cookie key to prevent header injection attacks.
///
/// Cookie keys must not contain `=`, `;`, whitespace, or control characters.
///
/// # Panics
///
/// Panics if the cookie key contains invalid characters.
fn validate_cookie_key(key: &str) {
	assert!(!key.is_empty(), "cookie key must not be empty");
	assert!(
		!key.contains('='),
		"cookie key must not contain '=' (found in key: {:?})",
		key
	);
	assert!(
		!key.contains(';'),
		"cookie key must not contain ';' (found in key: {:?})",
		key
	);
	assert!(
		!key.chars().any(|c| c.is_ascii_whitespace()),
		"cookie key must not contain whitespace (found in key: {:?})",
		key
	);
	assert!(
		!key.chars().any(|c| c.is_control()),
		"cookie key must not contain control characters (found in key: {:?})",
		key
	);
}

/// Validate a cookie value to prevent header injection attacks.
///
/// Cookie values must not contain `;`, newlines (`\r`, `\n`), or control characters.
///
/// # Panics
///
/// Panics if the cookie value contains invalid characters.
fn validate_cookie_value(value: &str) {
	assert!(!value.contains(';'), "cookie value must not contain ';'");
	assert!(
		!value.contains('\r') && !value.contains('\n'),
		"cookie value must not contain newlines"
	);
	assert!(
		!value.chars().any(|c| c.is_control()),
		"cookie value must not contain control characters"
	);
}

impl Default for APIClient {
	fn default() -> Self {
		Self::new()
	}
}

// Need to add base64 dependency
mod base64 {
	pub(super) fn encode(input: String) -> String {
		// Simple base64 encoding (in production, use a proper library)
		use base64_simd::STANDARD;
		STANDARD.encode_to_string(input.as_bytes())
	}
}

// Need to add urlencoding
mod urlencoding {
	pub(super) fn encode(input: &str) -> String {
		url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use async_trait::async_trait;
	use reinhardt_core::exception::{Error as HttpError, Result as HttpResult};
	use rstest::rstest;

	#[rstest::fixture]
	fn header_echo_client() -> APIClient {
		let mut client = APIClient::new();
		client.set_handler(|request| {
			let (parts, body) = request.into_parts();
			let mut response = Response::new(body);
			*response.headers_mut() = parts.headers;
			response.headers_mut().insert(
				"x-echo-method",
				HeaderValue::from_str(parts.method.as_str()).unwrap(),
			);
			response
		});
		client
	}

	fn header_values<'a>(response: &'a TestResponse, name: &str) -> Vec<&'a str> {
		response
			.headers()
			.get_all(name)
			.iter()
			.map(|value| value.to_str().unwrap())
			.collect()
	}

	#[rstest]
	#[tokio::test]
	async fn request_header_replaces_all_values_without_mutating_defaults(
		header_echo_client: APIClient,
	) {
		// Arrange
		let client = header_echo_client;
		client
			.set_header("Authorization", "Bearer bob")
			.await
			.unwrap();
		// Act
		let overridden = client
			.request(Method::GET, "/")
			.append_header("Authorization", "Bearer previous")
			.header("authorization", "Bearer alice")
			.send()
			.await
			.unwrap();
		let plain = client.get("/").await.unwrap();
		// Assert
		assert_eq!(
			header_values(&overridden, "authorization"),
			["Bearer alice"]
		);
		assert_eq!(header_values(&plain, "authorization"), ["Bearer bob"]);
	}

	// Exercise deprecated wrappers deliberately until their planned 0.5 removal.
	#[allow(deprecated)]
	async fn deprecated_header_request(
		client: &APIClient,
		post: bool,
	) -> ClientResult<TestResponse> {
		if post {
			client
				.post_raw_with_headers(
					"/",
					b"raw",
					"text/plain",
					&[("authorization", "Bearer alice")],
				)
				.await
		} else {
			client
				.get_with_headers("/", &[("authorization", "Bearer alice")])
				.await
		}
	}

	#[rstest]
	#[case(false)]
	#[case(true)]
	#[tokio::test]
	async fn deprecated_wrappers_replace_default_authorization(
		header_echo_client: APIClient,
		#[case] post: bool,
	) {
		// Arrange
		let client = header_echo_client;
		client
			.set_header("Authorization", "Bearer bob")
			.await
			.unwrap();
		// Act
		let overridden = deprecated_header_request(&client, post).await.unwrap();
		let plain = client.get("/").await.unwrap();
		// Assert
		assert_eq!(
			header_values(&overridden, "authorization"),
			["Bearer alice"]
		);
		assert_eq!(header_values(&plain, "authorization"), ["Bearer bob"]);
	}

	#[rstest]
	#[tokio::test]
	async fn request_header_operations_apply_in_order_after_generated_headers(
		header_echo_client: APIClient,
	) {
		// Arrange
		let client = header_echo_client;
		client.set_header("X-Tag", "default").await.unwrap();
		client.set_header("X-Remove", "default").await.unwrap();
		client.set_cookie("session", "manual").await.unwrap();
		*client.user.write().await = Some(serde_json::json!({"id": 1}));
		// Act
		let response = client
			.request(Method::POST, "/")
			.append_header("X-Tag", "second")
			.append_header("x-tag", "third")
			.without_header("x-remove")
			.without_header(http::header::COOKIE)
			.without_header("X-Test-User")
			.json(&serde_json::json!({"id": 1}))
			.header(http::header::CONTENT_TYPE, "application/custom")
			.header("x-order", "first")
			.without_header("x-order")
			.append_header("x-order", "last")
			.send()
			.await
			.unwrap();
		let plain = client.get("/").await.unwrap();
		// Assert
		assert_eq!(
			header_values(&response, "x-tag"),
			["default", "second", "third"]
		);
		for name in ["x-remove", "cookie", "x-test-user"] {
			assert_eq!(header_values(&response, name), Vec::<&str>::new());
		}
		assert_eq!(
			header_values(&response, "content-type"),
			["application/custom"]
		);
		assert_eq!(header_values(&response, "x-order"), ["last"]);
		assert_eq!(plain.header("x-remove"), Some("default"));
		assert_eq!(plain.header("cookie"), Some("session=manual"));
		assert_eq!(plain.header("x-test-user"), Some("authenticated"));
	}

	#[rstest]
	#[case(Method::PUT)]
	#[case(Method::PATCH)]
	#[case(Method::DELETE)]
	#[tokio::test]
	async fn request_builder_supports_headers_and_body_for_each_method(
		header_echo_client: APIClient,
		#[case] method: Method,
	) {
		// Arrange
		let client = header_echo_client;
		// Act
		let response = client
			.request(method.clone(), "/")
			.header("x-subject", "alice")
			.body(Bytes::from_static(b"payload"))
			.send()
			.await
			.unwrap();
		// Assert
		assert_eq!(response.header("x-echo-method"), Some(method.as_str()));
		assert_eq!(response.header("x-subject"), Some("alice"));
		assert_eq!(response.body().as_ref(), b"payload");
		assert_eq!(response.header("content-type"), None);
	}

	#[rstest]
	#[case("name")]
	#[case("value")]
	#[case("append")]
	#[case("remove")]
	#[tokio::test]
	async fn request_builder_defers_invalid_header_errors(#[case] invalid: &str) {
		// Arrange
		let mut client = APIClient::new();
		let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
		let captured = Arc::clone(&calls);
		client.set_handler(move |_| {
			captured.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
			Response::new(Full::new(Bytes::new()))
		});
		let request = client.request(Method::GET, "/");
		// Act
		let request = match invalid {
			"name" => request.header("invalid name", "value"),
			"value" => request.header("x-valid", "bad\nvalue"),
			"append" => request.append_header("invalid name", "value"),
			"remove" => request.without_header("invalid name"),
			_ => unreachable!(),
		};
		let result = request.header("x-valid", "valid").send().await;
		// Assert
		assert!(matches!(result, Err(ClientError::Http(_))));
		assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
	}

	#[rstest]
	#[tokio::test]
	async fn request_builder_retains_first_serialization_error() {
		// Arrange
		let mut client = APIClient::new();
		client.set_handler(|_| panic!("invalid builder must not dispatch"));
		let invalid = HashMap::from([((1, 2), "value")]);
		// Act
		let result = client
			.request(Method::POST, "/")
			.json(&invalid)
			.json(&serde_json::json!({"valid": true}))
			.header("invalid name", "value")
			.send()
			.await;
		let form_result = client.request(Method::POST, "/").form(&[1, 2]).send().await;
		// Assert
		assert!(matches!(result, Err(ClientError::Serialization(_))));
		match form_result {
			Err(ClientError::RequestFailed(message)) => {
				assert_eq!(message, "Expected object for form data")
			}
			_ => panic!("expected form serialization failure"),
		}
	}

	#[rstest]
	#[tokio::test]
	async fn request_serializes_immediately_and_explicit_headers_win(
		header_echo_client: APIClient,
	) {
		// Arrange
		let client = header_echo_client;
		client
			.set_header("content-type", "default/type")
			.await
			.unwrap();
		let mut data = serde_json::json!({"name": "before"});
		let request = client
			.request(Method::POST, "/")
			.header("content-type", "explicit/type")
			.json(&data);
		data["name"] = serde_json::json!("after");
		// Act
		let json = request.send().await.unwrap();
		let form = client
			.request(Method::POST, "/")
			.form(&serde_json::json!({"name": "A B", "count": 2}))
			.send()
			.await
			.unwrap();
		let raw = client
			.request(Method::POST, "/")
			.json(&data)
			.body("raw")
			.send()
			.await
			.unwrap();
		let removed = client
			.request(Method::POST, "/")
			.without_header("content-type")
			.form(&data)
			.send()
			.await
			.unwrap();
		// Assert
		assert_eq!(json.body().as_ref(), br#"{"name":"before"}"#);
		assert_eq!(header_values(&json, "content-type"), ["explicit/type"]);
		assert_eq!(form.body().as_ref(), b"count=2&name=A+B");
		assert_eq!(
			header_values(&form, "content-type"),
			["application/x-www-form-urlencoded"]
		);
		assert_eq!(raw.body().as_ref(), b"raw");
		assert_eq!(header_values(&raw, "content-type"), ["application/json"]);
		assert_eq!(removed.header("content-type"), None);
	}

	struct FileHandler {
		response: HttpResponse,
	}

	#[async_trait]
	impl HttpHandler for FileHandler {
		async fn handle(&self, _: HttpRequest) -> HttpResult<HttpResponse> {
			Ok(self.response.clone())
		}
	}

	#[rstest]
	#[case(0, 150_000)]
	#[case(13, 75_000)]
	#[case(0, 0)]
	#[tokio::test]
	async fn in_process_client_reads_owned_file_range(#[case] offset: u64, #[case] length: u64) {
		use std::io::Write;
		// Arrange
		let mut file = tempfile::tempfile().unwrap();
		let data: Vec<_> = (0..150_000).map(|i| (i % 251) as u8).collect();
		file.write_all(&data).unwrap();
		let response = HttpResponse::new(http::StatusCode::PARTIAL_CONTENT)
			.with_file_body(file, offset, length)
			.unwrap();
		let client = APIClient::from_handler(FileHandler { response });
		// Act
		let result = client.get("/asset").await.unwrap();
		// Assert
		assert_eq!(result.status(), http::StatusCode::PARTIAL_CONTENT);
		assert_eq!(
			result.body().as_ref(),
			&data[offset as usize..(offset + length) as usize]
		);
		assert_eq!(result.headers()["content-length"], length.to_string());
	}

	#[rstest]
	#[tokio::test]
	async fn in_process_client_propagates_truncated_file_error() {
		use std::io::Write;
		// Arrange
		let mut file = tempfile::tempfile().unwrap();
		file.write_all(b"original").unwrap();
		let truncate = file.try_clone().unwrap();
		let response = HttpResponse::ok().with_file_body(file, 0, 8).unwrap();
		let client = APIClient::from_handler(FileHandler { response });
		truncate.set_len(0).unwrap();
		// Act & Assert
		assert!(matches!(
			client.get("/asset").await,
			Err(ClientError::RequestFailed(_))
		));
	}

	#[rstest]
	#[tokio::test]
	async fn in_process_client_collects_streaming_chunks() {
		// Arrange
		let response = HttpResponse::ok()
			.with_stream(futures::stream::iter([
				Ok(Bytes::from_static(b"first")),
				Ok(Bytes::from_static(b"second")),
			]))
			.with_header("content-type", "text/event-stream");
		let client = APIClient::from_handler(FileHandler { response });

		// Act
		let result = client.get("/stream").await.unwrap();

		// Assert
		assert_eq!(result.status(), http::StatusCode::OK);
		assert_eq!(result.body().as_ref(), b"firstsecond");
		assert_eq!(result.headers()["content-type"], "text/event-stream");
	}

	#[rstest]
	#[tokio::test]
	async fn in_process_client_propagates_stream_error() {
		// Arrange
		let error: Box<dyn std::error::Error + Send + Sync> =
			std::io::Error::other("producer failed").into();
		let response = HttpResponse::ok().with_stream(futures::stream::iter([
			Ok(Bytes::from_static(b"partial")),
			Err(error),
		]));
		let client = APIClient::from_handler(FileHandler { response });

		// Act
		let result = client.get("/stream").await;

		// Assert
		match result {
			Err(ClientError::RequestFailed(message)) => assert_eq!(message, "producer failed"),
			Err(other) => panic!("expected producer failure, got {other}"),
			Ok(_) => panic!("expected producer failure, got a successful response"),
		}
	}

	/// Handler that echoes request metadata through X-Echo-* response headers.
	struct EchoHandler;

	#[async_trait]
	impl HttpHandler for EchoHandler {
		async fn handle(&self, request: HttpRequest) -> HttpResult<HttpResponse> {
			let path = request.uri.path().to_string();
			let method = request.method.as_str().to_string();
			let has_custom = request.headers.get("X-Custom").is_some();
			let content_type = request
				.headers
				.get("Content-Type")
				.and_then(|v| v.to_str().ok())
				.unwrap_or("")
				.to_string();
			let request_header = request
				.headers
				.get("X-Request")
				.and_then(|v| v.to_str().ok())
				.unwrap_or("missing");
			let raw_header = request
				.headers
				.get("X-Raw")
				.and_then(|v| v.to_str().ok())
				.unwrap_or("missing");
			let mut response = HttpResponse::ok().with_body(path.clone());
			response = response.try_with_header("X-Echo-Path", &path)?;
			response = response.try_with_header("X-Echo-Method", &method)?;
			response = response.try_with_header("X-Echo-X-Request", request_header)?;
			response = response.try_with_header("X-Echo-X-Raw", raw_header)?;

			if has_custom {
				response = response.try_with_header("X-Echo-Custom", "present")?;
			}
			if !content_type.is_empty() {
				response = response.try_with_header("X-Echo-Content-Type", &content_type)?;
			}
			Ok(response)
		}
	}

	/// Handler that always returns an error.
	struct ErrorHandler;

	#[async_trait]
	impl HttpHandler for ErrorHandler {
		async fn handle(&self, _request: HttpRequest) -> HttpResult<HttpResponse> {
			Err(HttpError::NotFound("test resource".to_string()))
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_from_handler_basic() {
		// Arrange
		let client = APIClient::from_handler(EchoHandler);

		// Act
		let response = client.get("/test/path/").await.expect("request failed");

		// Assert
		assert_eq!(response.status(), http::StatusCode::OK);
		assert_eq!(response.body().as_ref(), b"/test/path/");
	}

	#[rstest]
	#[tokio::test]
	async fn test_from_handler_post_body() {
		// Arrange
		let client = APIClient::from_handler(EchoHandler);
		let body = serde_json::json!({"key": "value"});

		// Act
		let response = client
			.post("/echo/", &body, "json")
			.await
			.expect("request failed");

		// Assert
		assert_eq!(response.status(), http::StatusCode::OK);
		assert_eq!(
			response
				.header("X-Echo-Content-Type")
				.expect("missing header"),
			"application/json"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_from_handler_headers() {
		// Arrange
		let client = APIClient::from_handler(EchoHandler);
		client
			.set_header("X-Custom", "test-value")
			.await
			.expect("set_header failed");

		// Act
		let response = client.get("/test/").await.expect("request failed");

		// Assert
		assert_eq!(response.status(), http::StatusCode::OK);
		assert_eq!(
			response.header("X-Echo-Custom").expect("missing header"),
			"present"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_from_handler_error_conversion() {
		// Arrange
		let client = APIClient::from_handler(ErrorHandler);

		// Act
		let response = client.get("/anything/").await.expect("request failed");

		// Assert
		assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
	}

	#[rstest]
	#[tokio::test]
	async fn test_from_handler_origin_header() {
		// Arrange
		let client = APIClient::from_handler(EchoHandler);

		// Act
		let headers = client.default_headers.read().await;

		// Assert
		let origin = headers
			.get(http::header::ORIGIN)
			.expect("Origin header not set");
		assert_eq!(origin.to_str().unwrap(), "http://testserver");
	}

	#[rstest]
	#[tokio::test]
	async fn test_builder_with_handler() {
		// Arrange
		let client = APIClient::builder()
			.base_url("http://mytest")
			.handler(EchoHandler)
			.build();

		// Act
		let response = client.get("/api/").await.expect("request failed");

		// Assert
		assert_eq!(response.status(), http::StatusCode::OK);
		let headers = client.default_headers.read().await;
		let origin = headers
			.get(http::header::ORIGIN)
			.expect("Origin header not set");
		assert_eq!(origin.to_str().unwrap(), "http://mytest");
	}

	#[rstest]
	#[tokio::test]
	async fn api_client_sync_handler_and_invalid_headers_report_exact_public_results() {
		// Arrange
		let mut client = APIClient::new();
		client.set_handler(|request| {
			let method = request.method().as_str().to_string();
			let content_type = request
				.headers()
				.get(http::header::CONTENT_TYPE)
				.and_then(|value| value.to_str().ok())
				.unwrap_or("missing")
				.to_string();
			Response::builder()
				.status(http::StatusCode::CREATED)
				.header("X-Method", method)
				.header("X-Content-Type", content_type)
				.body(Full::new(Bytes::from("synchronous")))
				.unwrap()
		});

		// Act
		let response = client
			.post_raw("/sync", b"body", "text/plain")
			.await
			.unwrap();
		let invalid_name = client
			.set_header("invalid header", "value")
			.await
			.unwrap_err();
		let invalid_value = client
			.set_header("X-Valid", "bad\nvalue")
			.await
			.unwrap_err();

		// Assert
		assert_eq!(response.status(), http::StatusCode::CREATED);
		assert_eq!(response.body().as_ref(), b"synchronous");
		assert_eq!(response.header("X-Method"), Some("POST"));
		assert_eq!(response.header("X-Content-Type"), Some("text/plain"));
		assert_eq!(
			invalid_name.to_string(),
			"Request failed: Invalid header name: invalid header"
		);
		assert_eq!(invalid_name.is_request(), true);
		assert!(matches!(&invalid_value, ClientError::InvalidHeaderValue(_)));
		assert_eq!(invalid_value.is_request(), true);
		assert_eq!(invalid_value.is_timeout(), false);
		assert_eq!(invalid_value.is_connect(), false);
	}

	#[rstest]
	fn test_validate_cookie_key_accepts_valid_key() {
		// Arrange
		let key = "session_id";

		// Act & Assert (should not panic)
		validate_cookie_key(key);
	}

	#[rstest]
	#[should_panic(expected = "must not be empty")]
	fn test_validate_cookie_key_rejects_empty() {
		// Arrange
		let key = "";

		// Act
		validate_cookie_key(key);
	}

	#[rstest]
	#[should_panic(expected = "must not contain '='")]
	fn test_validate_cookie_key_rejects_equals_sign() {
		// Arrange
		let key = "key=value";

		// Act
		validate_cookie_key(key);
	}

	#[rstest]
	#[should_panic(expected = "must not contain ';'")]
	fn test_validate_cookie_key_rejects_semicolon() {
		// Arrange
		let key = "key;injection";

		// Act
		validate_cookie_key(key);
	}

	#[rstest]
	#[should_panic(expected = "must not contain whitespace")]
	fn test_validate_cookie_key_rejects_whitespace() {
		// Arrange
		let key = "key name";

		// Act
		validate_cookie_key(key);
	}

	#[rstest]
	#[should_panic(expected = "must not contain control characters")]
	fn test_validate_cookie_key_rejects_control_chars() {
		// Arrange
		let key = "key\x00name";

		// Act
		validate_cookie_key(key);
	}

	#[rstest]
	fn test_validate_cookie_value_accepts_valid_value() {
		// Arrange
		let value = "abc123-token";

		// Act & Assert (should not panic)
		validate_cookie_value(value);
	}

	#[rstest]
	fn test_validate_cookie_value_accepts_empty() {
		// Arrange
		let value = "";

		// Act & Assert (should not panic)
		validate_cookie_value(value);
	}

	#[rstest]
	#[should_panic(expected = "must not contain ';'")]
	fn test_validate_cookie_value_rejects_semicolon() {
		// Arrange
		let value = "value; extra=injected";

		// Act
		validate_cookie_value(value);
	}

	#[rstest]
	#[should_panic(expected = "must not contain newlines")]
	fn test_validate_cookie_value_rejects_newline() {
		// Arrange
		let value = "value\r\nInjected-Header: malicious";

		// Act
		validate_cookie_value(value);
	}

	#[rstest]
	#[should_panic(expected = "must not contain control characters")]
	fn test_validate_cookie_value_rejects_control_chars() {
		// Arrange
		let value = "value\x01hidden";

		// Act
		validate_cookie_value(value);
	}

	#[rstest]
	#[should_panic(expected = "must not contain newlines")]
	fn test_validate_cookie_value_rejects_lf_only() {
		// Arrange
		let value = "value\nInjected-Header: evil";

		// Act
		validate_cookie_value(value);
	}
}
