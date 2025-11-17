mod body;
mod methods;
mod params;

use crate::extensions::Extensions;
use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
#[cfg(feature = "parsers")]
use reinhardt_parsers::parser::{ParsedData, Parser};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// HTTP Request representation
pub struct Request {
	pub method: Method,
	pub uri: Uri,
	pub version: Version,
	pub headers: HeaderMap,
	body: Bytes,
	pub path_params: HashMap<String, String>,
	pub query_params: HashMap<String, String>,
	/// Indicates if this request came over HTTPS
	pub is_secure: bool,
	/// Remote address of the client (if available)
	pub remote_addr: Option<SocketAddr>,
	/// Parsers for request body
	#[cfg(feature = "parsers")]
	parsers: Vec<Box<dyn Parser>>,
	/// Cached parsed data (lazy parsing)
	#[cfg(feature = "parsers")]
	parsed_data: Arc<Mutex<Option<ParsedData>>>,
	/// Whether the body has been consumed
	body_consumed: Arc<AtomicBool>,
	/// Extensions for storing arbitrary typed data
	pub extensions: Extensions,
}

/// Builder for constructing `Request` instances.
///
/// Provides a fluent API for building HTTP requests with optional parameters.
///
/// # Examples
///
/// ```
/// use reinhardt_http::Request;
/// use hyper::Method;
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/api/users?page=1")
///     .build()
///     .unwrap();
///
/// assert_eq!(request.method, Method::GET);
/// assert_eq!(request.path(), "/api/users");
/// assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));
/// ```
pub struct RequestBuilder {
	method: Method,
	uri: Option<Uri>,
	version: Version,
	headers: HeaderMap,
	body: Bytes,
	is_secure: bool,
	remote_addr: Option<SocketAddr>,
	#[cfg(feature = "parsers")]
	parsers: Vec<Box<dyn Parser>>,
}

impl Default for RequestBuilder {
	fn default() -> Self {
		Self {
			method: Method::GET,
			uri: None,
			version: Version::HTTP_11,
			headers: HeaderMap::new(),
			body: Bytes::new(),
			is_secure: false,
			remote_addr: None,
			#[cfg(feature = "parsers")]
			parsers: Vec::new(),
		}
	}
}

impl RequestBuilder {
	/// Set the HTTP method.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// let request = Request::builder()
	///     .method(Method::POST)
	///     .uri("/api/users")
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.method, Method::POST);
	/// ```
	pub fn method(mut self, method: Method) -> Self {
		self.method = method;
		self
	}

	/// Set the request URI.
	///
	/// Accepts either a `&str` or `Uri`. Query parameters will be automatically parsed.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/api/users?page=1&limit=10")
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.path(), "/api/users");
	/// assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));
	/// assert_eq!(request.query_params.get("limit"), Some(&"10".to_string()));
	/// ```
	pub fn uri<T>(mut self, uri: T) -> Self
	where
		T: TryInto<Uri>,
	{
		if let Ok(uri) = uri.try_into() {
			self.uri = Some(uri);
		}
		self
	}

	/// Set the HTTP version.
	///
	/// Defaults to HTTP/1.1 if not specified.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Version};
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/api/users")
	///     .version(Version::HTTP_2)
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.version, Version::HTTP_2);
	/// ```
	pub fn version(mut self, version: Version) -> Self {
		self.version = version;
		self
	}

	/// Set the request headers.
	///
	/// Replaces all existing headers.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, HeaderMap, header};
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
	///
	/// let request = Request::builder()
	///     .method(Method::POST)
	///     .uri("/api/users")
	///     .headers(headers.clone())
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.headers.get(header::CONTENT_TYPE).unwrap(), "application/json");
	/// ```
	pub fn headers(mut self, headers: HeaderMap) -> Self {
		self.headers = headers;
		self
	}

	/// Add a single header to the request.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, header};
	///
	/// let request = Request::builder()
	///     .method(Method::POST)
	///     .uri("/api/users")
	///     .header(header::CONTENT_TYPE, "application/json")
	///     .header(header::AUTHORIZATION, "Bearer token123")
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.headers.get(header::CONTENT_TYPE).unwrap(), "application/json");
	/// assert_eq!(request.headers.get(header::AUTHORIZATION).unwrap(), "Bearer token123");
	/// ```
	pub fn header<K, V>(mut self, key: K, value: V) -> Self
	where
		K: hyper::header::IntoHeaderName,
		V: TryInto<hyper::header::HeaderValue>,
	{
		if let Ok(val) = value.try_into() {
			self.headers.insert(key, val);
		}
		self
	}

	/// Set the request body.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	/// use bytes::Bytes;
	///
	/// let body = Bytes::from(r#"{"name":"Alice"}"#);
	/// let request = Request::builder()
	///     .method(Method::POST)
	///     .uri("/api/users")
	///     .body(body.clone())
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.body(), &body);
	/// ```
	pub fn body(mut self, body: Bytes) -> Self {
		self.body = body;
		self
	}

	/// Set whether the request is secure (HTTPS).
	///
	/// Defaults to `false` if not specified.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .secure(true)
	///     .build()
	///     .unwrap();
	///
	/// assert!(request.is_secure());
	/// assert_eq!(request.scheme(), "https");
	/// ```
	pub fn secure(mut self, is_secure: bool) -> Self {
		self.is_secure = is_secure;
		self
	}

	/// Set the remote address of the client.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	/// use std::net::{SocketAddr, IpAddr, Ipv4Addr};
	///
	/// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .remote_addr(addr)
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.remote_addr, Some(addr));
	/// ```
	pub fn remote_addr(mut self, addr: SocketAddr) -> Self {
		self.remote_addr = Some(addr);
		self
	}

	/// Add a parser to the request.
	///
	/// Parsers are used to parse the request body into specific formats.
	/// The parser will be boxed internally.
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// let request = Request::builder()
	///     .method(Method::POST)
	///     .uri("/api/users")
	///     .parser(JsonParser::new())
	///     .build()
	///     .unwrap();
	/// ```
	#[cfg(feature = "parsers")]
	pub fn parser<P: Parser + 'static>(mut self, parser: P) -> Self {
		self.parsers.push(Box::new(parser));
		self
	}

	/// Build the final `Request` instance.
	///
	/// Returns an error if the URI is missing.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/api/users")
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.method, Method::GET);
	/// assert_eq!(request.path(), "/api/users");
	/// ```
	pub fn build(self) -> Result<Request, &'static str> {
		let uri = self.uri.ok_or("URI is required")?;
		let query_params = Request::parse_query_params(&uri);

		Ok(Request {
			method: self.method,
			uri,
			version: self.version,
			headers: self.headers,
			body: self.body,
			path_params: HashMap::new(),
			query_params,
			is_secure: self.is_secure,
			remote_addr: self.remote_addr,
			#[cfg(feature = "parsers")]
			parsers: self.parsers,
			#[cfg(feature = "parsers")]
			parsed_data: Arc::new(Mutex::new(None)),
			body_consumed: Arc::new(AtomicBool::new(false)),
			extensions: Extensions::new(),
		})
	}
}

impl Request {
	/// Create a new `RequestBuilder`.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/api/users")
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.method, Method::GET);
	/// ```
	pub fn builder() -> RequestBuilder {
		RequestBuilder::default()
	}

	/// Set the DI context for this request (used by routers with dependency injection)
	///
	/// This method stores the DI context in the request's extensions,
	/// allowing handlers to access dependency injection services.
	///
	/// The context will be wrapped in an Arc internally for efficient sharing.
	/// The DI context type is generic to avoid circular dependencies.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// # struct DummyDiContext;
	/// let mut request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .build()
	///     .unwrap();
	///
	/// let di_ctx = DummyDiContext;
	/// request.set_di_context(di_ctx);
	/// ```
	pub fn set_di_context<T: Send + Sync + 'static>(&mut self, ctx: T) {
		self.extensions.insert(Arc::new(ctx));
	}

	/// Get the DI context from this request
	///
	/// Returns `None` if no DI context was set.
	///
	/// The DI context type is generic to avoid circular dependencies.
	/// Returns a reference to the context.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// # struct DummyDiContext;
	/// let mut request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .build()
	///     .unwrap();
	///
	/// let di_ctx = DummyDiContext;
	/// request.set_di_context(di_ctx);
	///
	/// let ctx = request.get_di_context::<DummyDiContext>();
	/// assert!(ctx.is_some());
	/// ```
	pub fn get_di_context<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
		self.extensions.get::<Arc<T>>()
	}
}
