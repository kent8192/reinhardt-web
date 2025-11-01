use super::Request;
use hyper::Uri;
use percent_encoding::percent_decode_str;
use std::collections::HashMap;

impl Request {
	/// Parse query parameters from URI
	pub(super) fn parse_query_params(uri: &Uri) -> HashMap<String, String> {
		uri.query()
			.map(|q| {
				q.split('&')
					.filter_map(|pair| {
						let mut parts = pair.split('=');
						Some((
							parts.next()?.to_string(),
							parts.next().unwrap_or("").to_string(),
						))
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// Get the request path
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
	///     "/api/users".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new()
	/// );
	///
	/// assert_eq!(request.path(), "/api/users");
	/// ```
	pub fn path(&self) -> &str {
		self.uri.path()
	}

	/// Get URL-decoded query parameters
	///
	/// Returns a new HashMap with all query parameter keys and values URL-decoded.
	/// This is useful when query parameters contain special characters or Unicode.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let uri = "/test?name=John%20Doe".parse::<Uri>().unwrap();
	/// let request = Request::new(
	///     Method::GET,
	///     uri,
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new(),
	/// );
	///
	/// let decoded = request.decoded_query_params();
	/// assert_eq!(decoded.get("name"), Some(&"John Doe".to_string()));
	/// ```
	pub fn decoded_query_params(&self) -> HashMap<String, String> {
		self.query_params
			.iter()
			.map(|(k, v)| {
				let decoded_key = percent_decode_str(k).decode_utf8_lossy().to_string();
				let decoded_value = percent_decode_str(v).decode_utf8_lossy().to_string();
				(decoded_key, decoded_value)
			})
			.collect()
	}

	/// Set a path parameter (used by routers for path variable extraction)
	///
	/// This method is typically called by routers when extracting path parameters
	/// from URL patterns like `/users/{id}/`.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let mut request = Request::new(
	///     Method::GET,
	///     "/users/123".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     HeaderMap::new(),
	///     Bytes::new()
	/// );
	///
	/// request.set_path_param("id", "123");
	/// assert_eq!(request.path_params.get("id"), Some(&"123".to_string()));
	/// ```
	pub fn set_path_param(&mut self, key: impl Into<String>, value: impl Into<String>) {
		self.path_params.insert(key.into(), value.into());
	}

	/// Parse Accept-Language header and return ordered list of language codes
	///
	/// Returns languages sorted by quality value (q parameter), highest first.
	/// Example: "en-US,en;q=0.9,ja;q=0.8" -> ["en-US", "en", "ja"]
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert("accept-language", "en-US,en;q=0.9,ja;q=0.8".parse().unwrap());
	///
	/// let request = Request::new(
	///     Method::GET,
	///     "/".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     headers,
	///     Bytes::new()
	/// );
	///
	/// let languages = request.get_accepted_languages();
	/// assert_eq!(languages[0].0, "en-US");
	/// assert_eq!(languages[0].1, 1.0);
	/// assert_eq!(languages[1].0, "en");
	/// assert_eq!(languages[1].1, 0.9);
	/// ```
	pub fn get_accepted_languages(&self) -> Vec<(String, f32)> {
		use hyper::header::ACCEPT_LANGUAGE;
		self.headers
			.get(ACCEPT_LANGUAGE)
			.and_then(|h| h.to_str().ok())
			.map(|header_value| Self::parse_accept_language(header_value))
			.unwrap_or_default()
	}

	/// Get the most preferred language from Accept-Language header
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert("accept-language", "ja;q=0.8,en-US,en;q=0.9".parse().unwrap());
	///
	/// let request = Request::new(
	///     Method::GET,
	///     "/".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     headers,
	///     Bytes::new()
	/// );
	///
	/// assert_eq!(request.get_preferred_language(), Some("en-US".to_string()));
	/// ```
	pub fn get_preferred_language(&self) -> Option<String> {
		self.get_accepted_languages()
			.into_iter()
			.max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
			.map(|(lang, _)| lang)
	}

	/// Parse Accept-Language header value
	///
	/// Handles both weighted (q=) and unweighted language preferences.
	/// Example: "en-US,en;q=0.9,ja;q=0.8" -> [("en-US", 1.0), ("en", 0.9), ("ja", 0.8)]
	fn parse_accept_language(header: &str) -> Vec<(String, f32)> {
		let mut languages: Vec<(String, f32)> = header
			.split(',')
			.filter_map(|lang_part| {
				let lang_part = lang_part.trim();
				if lang_part.is_empty() {
					return None;
				}

				// Split on ';' to separate language from quality
				let parts: Vec<&str> = lang_part.split(';').collect();
				let language = parts[0].trim().to_string();

				// Parse quality value if present
				let quality = if parts.len() > 1 {
					parts[1]
						.trim()
						.strip_prefix("q=")
						.and_then(|q| q.parse::<f32>().ok())
						.unwrap_or(1.0)
				} else {
					1.0
				};

				// Validate language code
				if Self::is_valid_language_code(&language) {
					Some((language, quality))
				} else {
					None
				}
			})
			.collect();

		// Sort by quality (descending)
		languages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
		languages
	}

	/// Validate language code format
	///
	/// Accepts formats like:
	/// - "en"
	/// - "en-US"
	/// - "zh-Hans"
	/// - "sr-Latn-RS"
	/// - "nl-nl-x-informal" (with private use subtag)
	///
	/// Rejects:
	/// - Too long (>255 chars)
	/// - Invalid characters
	/// - Starting/ending with hyphen
	fn is_valid_language_code(code: &str) -> bool {
		if code.is_empty() || code.len() > 255 {
			return false;
		}

		// Must not start or end with hyphen
		if code.starts_with('-') || code.ends_with('-') {
			return false;
		}

		// Check for valid characters (alphanumeric and hyphen)
		code.chars().all(|c| c.is_alphanumeric() || c == '-')
	}

	/// Get language from cookie
	///
	/// Looks for a language cookie (typically named "reinhardt_language" or similar)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_http::Request;
	/// use hyper::{Method, Uri, Version, HeaderMap};
	/// use bytes::Bytes;
	///
	/// let mut headers = HeaderMap::new();
	/// headers.insert("cookie", "session_id=abc123; language=ja; theme=dark".parse().unwrap());
	///
	/// let request = Request::new(
	///     Method::GET,
	///     "/".parse::<Uri>().unwrap(),
	///     Version::HTTP_11,
	///     headers,
	///     Bytes::new()
	/// );
	///
	/// assert_eq!(request.get_language_from_cookie("language"), Some("ja".to_string()));
	/// assert_eq!(request.get_language_from_cookie("nonexistent"), None);
	/// ```
	pub fn get_language_from_cookie(&self, cookie_name: &str) -> Option<String> {
		use hyper::header::COOKIE;

		self.headers
			.get(COOKIE)
			.and_then(|h| h.to_str().ok())
			.and_then(|cookies| {
				cookies.split(';').find_map(|cookie| {
					let mut parts = cookie.trim().splitn(2, '=');
					let name = parts.next()?.trim();
					let value = parts.next()?.trim();
					if name == cookie_name {
						Some(value.to_string())
					} else {
						None
					}
				})
			})
			.filter(|lang| Self::is_valid_language_code(lang))
	}
}
