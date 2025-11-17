use super::Request;

impl Request {
	/// Returns true if the request was made over HTTPS
	///
	/// This can be determined either by:
	/// 1. The actual connection being TLS (is_secure flag)
	/// 2. X-Forwarded-Proto header indicating HTTPS (behind reverse proxy)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::Method;
	///
	/// // Direct HTTPS connection
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .secure(true)
	///     .build()
	///     .unwrap();
	/// assert!(request.is_secure());
	///
	/// // Behind reverse proxy
	/// let mut headers = hyper::HeaderMap::new();
	/// headers.insert("x-forwarded-proto", "https".parse().unwrap());
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .headers(headers)
	///     .build()
	///     .unwrap();
	/// assert!(request.is_secure());
	/// ```
	pub fn is_secure(&self) -> bool {
		if self.is_secure {
			return true;
		}

		// Check X-Forwarded-Proto header for reverse proxy scenarios
		self.headers
			.get("x-forwarded-proto")
			.and_then(|h| h.to_str().ok())
			.map(|proto| proto.eq_ignore_ascii_case("https"))
			.unwrap_or(false)
	}

	/// Returns the scheme of the request (http or https)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .secure(true)
	///     .build()
	///     .unwrap();
	/// assert_eq!(request.scheme(), "https");
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .build()
	///     .unwrap();
	/// assert_eq!(request.scheme(), "http");
	/// ```
	pub fn scheme(&self) -> &str {
		if self.is_secure() { "https" } else { "http" }
	}

	/// Build an absolute URI for the request
	///
	/// Example: "https://example.com:8000/path?query=value"
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let mut headers = hyper::HeaderMap::new();
	/// headers.insert("host", "example.com".parse().unwrap());
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/api/users")
	///     .headers(headers)
	///     .secure(true)
	///     .build()
	///     .unwrap();
	///
	/// let uri = request.build_absolute_uri(None);
	/// assert_eq!(uri, "https://example.com/api/users");
	///
	/// let uri = request.build_absolute_uri(Some("/other/path"));
	/// assert_eq!(uri, "https://example.com/other/path");
	/// ```
	pub fn build_absolute_uri(&self, path: Option<&str>) -> String {
		let scheme = self.scheme();
		let host = self.get_host().unwrap_or_else(|| "localhost".to_string());
		let path = path.unwrap_or_else(|| self.path());

		format!("{}://{}{}", scheme, host, path)
	}

	/// Get the host from the request headers
	fn get_host(&self) -> Option<String> {
		self.headers
			.get(hyper::header::HOST)
			.and_then(|h| h.to_str().ok())
			.map(|s| s.to_string())
	}
}
