mod body;
mod methods;
mod params;

use crate::extensions::Extensions;
use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
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
	parsers: Vec<Box<dyn Parser>>,
	/// Cached parsed data (lazy parsing)
	parsed_data: Arc<Mutex<Option<ParsedData>>>,
	/// Whether the body has been consumed
	body_consumed: Arc<AtomicBool>,
	/// Extensions for storing arbitrary typed data
	pub extensions: Extensions,
}

impl Request {
	/// Create a new Request
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let request = Request::new(
	///     Method::GET,
	///     "/api/users?page=1".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new()
	/// );
	///
	/// assert_eq!(request.method, Method::GET);
	/// assert_eq!(request.path(), "/api/users");
	/// assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));
	/// ```
	pub fn new(
		method: Method,
		uri: Uri,
		version: Version,
		headers: HeaderMap,
		body: Bytes,
	) -> Self {
		let query_params = Self::parse_query_params(&uri);

		Self {
			method,
			uri,
			version,
			headers,
			body,
			path_params: HashMap::new(),
			query_params,
			is_secure: false, // Default to insecure, can be set later
			remote_addr: None,
			parsers: Vec::new(),
			parsed_data: Arc::new(Mutex::new(None)),
			body_consumed: Arc::new(AtomicBool::new(false)),
			extensions: Extensions::new(),
		}
	}

	/// Create a new Request with explicit secure flag
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let request = Request::new_with_secure(
	///     Method::GET,
	///     "/".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new(),
	///     true  // HTTPS
	/// );
	///
	/// assert!(request.is_secure());
	/// assert_eq!(request.scheme(), "https");
	/// ```
	pub fn new_with_secure(
		method: Method,
		uri: Uri,
		version: Version,
		headers: HeaderMap,
		body: Bytes,
		is_secure: bool,
	) -> Self {
		let query_params = Self::parse_query_params(&uri);

		Self {
			method,
			uri,
			version,
			headers,
			body,
			path_params: HashMap::new(),
			query_params,
			is_secure,
			remote_addr: None,
			parsers: Vec::new(),
			parsed_data: Arc::new(Mutex::new(None)),
			body_consumed: Arc::new(AtomicBool::new(false)),
			extensions: Extensions::new(),
		}
	}

	/// Set the DI context for this request (used by routers with dependency injection)
	///
	/// This method stores the DI context in the request's extensions,
	/// allowing handlers to access dependency injection services.
	///
	/// The DI context type is generic to avoid circular dependencies.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	/// use std::sync::Arc;
	///
	/// # struct DummyDiContext;
	/// let mut request = Request::new(
	///     Method::GET,
	///     "/".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new()
	/// );
	///
	/// let di_ctx = Arc::new(DummyDiContext);
	/// request.set_di_context(di_ctx);
	/// ```
	pub fn set_di_context<T: Send + Sync + 'static>(&mut self, ctx: Arc<T>) {
		self.extensions.insert(ctx);
	}

	/// Get the DI context from this request
	///
	/// Returns `None` if no DI context was set.
	///
	/// The DI context type is generic to avoid circular dependencies.
	/// Returns a cloned Arc for the DI context (cheap operation).
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	/// use std::sync::Arc;
	///
	/// # struct DummyDiContext;
	/// let mut request = Request::new(
	///     Method::GET,
	///     "/".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new()
	/// );
	///
	/// let di_ctx = Arc::new(DummyDiContext);
	/// request.set_di_context(di_ctx);
	///
	/// assert!(request.get_di_context::<DummyDiContext>().is_some());
	/// ```
	pub fn get_di_context<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
		self.extensions.get::<Arc<T>>()
	}
}
