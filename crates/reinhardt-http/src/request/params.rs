use super::Request;
use hyper::Uri;
use percent_encoding::percent_decode_str;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::collections::hash_map::Iter;
use std::ops::Deref;
use std::ops::Range;
use std::sync::OnceLock;

const INLINE_QUERY_PAIR_CAPACITY: usize = 8;

#[derive(Clone, Debug)]
struct RawQueryPair {
	key: Range<usize>,
	value: Range<usize>,
}

/// Lazily parsed query string parameters.
///
/// `QueryParams` keeps a cheap URI clone when a query string exists. Direct
/// `get` lookups cache raw query key/value byte ranges without allocating for
/// common small query strings, while map-shaped APIs materialize a `HashMap` on
/// first use. This keeps request construction cheap for handlers that do not
/// inspect query parameters while preserving the common `HashMap`-like read
/// API.
#[derive(Debug)]
pub struct QueryParams {
	uri_with_query: Option<Uri>,
	raw_pairs: OnceLock<SmallVec<[RawQueryPair; INLINE_QUERY_PAIR_CAPACITY]>>,
	parsed: OnceLock<HashMap<String, String>>,
}

impl QueryParams {
	/// Create an empty query parameter set.
	pub fn empty() -> Self {
		Self {
			uri_with_query: None,
			raw_pairs: OnceLock::new(),
			parsed: OnceLock::new(),
		}
	}

	/// Capture query parameters from a URI without parsing them.
	pub fn from_uri(uri: &Uri) -> Self {
		Self {
			uri_with_query: uri.query().map(|_| uri.clone()),
			raw_pairs: OnceLock::new(),
			parsed: OnceLock::new(),
		}
	}

	/// Return the raw query string captured from the URI, if present.
	pub fn raw_query(&self) -> Option<&str> {
		self.uri_with_query.as_ref().and_then(Uri::query)
	}

	/// Return the parsed query parameters as a map.
	///
	/// Calling this method initializes the parsed map on first access.
	pub fn as_map(&self) -> &HashMap<String, String> {
		self.parsed.get_or_init(|| match self.raw_query() {
			Some(query) => Self::parse_raw(query),
			None => HashMap::new(),
		})
	}

	/// Return a cloned `HashMap` of the parsed query parameters.
	pub fn to_hash_map(&self) -> HashMap<String, String> {
		self.as_map().clone()
	}

	/// Return the value for a query parameter key.
	///
	/// This looks up cached raw query ranges when available, avoiding `HashMap`
	/// initialization for handlers that only need a small number of known
	/// parameters.
	pub fn get(&self, key: &str) -> Option<&str> {
		self.raw_query()
			.and_then(|query| self.find_cached_raw_value(query, key))
			.or_else(|| {
				self.parsed
					.get()
					.and_then(|map| map.get(key).map(String::as_str))
			})
	}

	/// Return `true` when the query parameter key exists.
	pub fn contains_key(&self, key: &str) -> bool {
		self.get(key).is_some()
	}

	/// Return `true` when there are no query parameters.
	pub fn is_empty(&self) -> bool {
		self.as_map().is_empty()
	}

	/// Return the number of query parameters.
	pub fn len(&self) -> usize {
		self.as_map().len()
	}

	/// Iterate over parsed query parameter key-value pairs.
	pub fn iter(&self) -> Iter<'_, String, String> {
		self.as_map().iter()
	}

	fn parse_raw(query: &str) -> HashMap<String, String> {
		query
			.split('&')
			.filter_map(|pair| {
				// Split on first '=' only to preserve '=' in values (e.g., Base64).
				let mut parts = pair.splitn(2, '=');
				Some((
					parts.next()?.to_string(),
					parts.next().unwrap_or("").to_string(),
				))
			})
			.collect()
	}

	fn raw_pairs(&self, query: &str) -> &SmallVec<[RawQueryPair; INLINE_QUERY_PAIR_CAPACITY]> {
		self.raw_pairs.get_or_init(|| Self::parse_raw_pairs(query))
	}

	fn parse_raw_pairs(query: &str) -> SmallVec<[RawQueryPair; INLINE_QUERY_PAIR_CAPACITY]> {
		let mut pairs = SmallVec::new();
		let mut offset = 0;
		for pair in query.split('&') {
			let pair_start = offset;
			let pair_end = pair_start + pair.len();
			let (key, value) = match pair.find('=') {
				Some(separator) => {
					let key = pair_start..pair_start + separator;
					let value = pair_start + separator + 1..pair_end;
					(key, value)
				}
				None => (pair_start..pair_end, pair_end..pair_end),
			};
			pairs.push(RawQueryPair { key, value });
			offset = pair_end + 1;
		}
		pairs
	}

	fn find_cached_raw_value<'a>(&self, query: &'a str, key: &str) -> Option<&'a str> {
		for pair in self.raw_pairs(query).iter().rev() {
			if &query[pair.key.clone()] == key {
				return Some(&query[pair.value.clone()]);
			}
		}
		None
	}

	#[cfg(test)]
	fn is_parsed(&self) -> bool {
		self.parsed.get().is_some()
	}

	#[cfg(test)]
	fn has_raw_pair_cache(&self) -> bool {
		self.raw_pairs.get().is_some()
	}
}

impl Clone for QueryParams {
	fn clone(&self) -> Self {
		let parsed = OnceLock::new();
		if let Some(map) = self.parsed.get() {
			parsed
				.set(map.clone())
				.expect("new OnceLock must accept initial query params");
		}
		let raw_pairs = OnceLock::new();
		if let Some(pairs) = self.raw_pairs.get() {
			raw_pairs
				.set(pairs.clone())
				.expect("new OnceLock must accept initial raw query pairs");
		}

		Self {
			uri_with_query: self.uri_with_query.clone(),
			raw_pairs,
			parsed,
		}
	}
}

impl Default for QueryParams {
	fn default() -> Self {
		Self::empty()
	}
}

impl Deref for QueryParams {
	type Target = HashMap<String, String>;

	fn deref(&self) -> &Self::Target {
		self.as_map()
	}
}

impl AsRef<HashMap<String, String>> for QueryParams {
	fn as_ref(&self) -> &HashMap<String, String> {
		self.as_map()
	}
}

impl PartialEq for QueryParams {
	fn eq(&self, other: &Self) -> bool {
		self.as_map() == other.as_map()
	}
}

impl Eq for QueryParams {}

impl<'a> IntoIterator for &'a QueryParams {
	type IntoIter = Iter<'a, String, String>;
	type Item = (&'a String, &'a String);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl Request {
	/// Get the request path
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
	/// use hyper::Method;
	///
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/test?name=John%20Doe")
	///     .build()
	///     .unwrap();
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
	/// let mut request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/users/123")
	///     .build()
	///     .unwrap();
	///
	/// request.set_path_param("id", "123");
	/// assert_eq!(request.path_params.get("id"), Some("123"));
	/// ```
	pub fn set_path_param(&mut self, key: impl Into<String>, value: impl AsRef<str>) {
		self.path_params.insert(key, value);
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
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .headers(headers)
	///     .build()
	///     .unwrap();
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
			.map(Self::parse_accept_language)
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
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .headers(headers)
	///     .build()
	///     .unwrap();
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
	/// let request = Request::builder()
	///     .method(Method::GET)
	///     .uri("/")
	///     .headers(headers)
	///     .build()
	///     .unwrap();
	///
	/// assert_eq!(request.get_language_from_cookie("language"), Some("ja".to_string()));
	/// assert_eq!(request.get_language_from_cookie("nonexistent"), None);
	/// ```
	pub fn get_language_from_cookie(&self, cookie_name: &str) -> Option<String> {
		use hyper::header::COOKIE;

		self.headers
			.get(COOKIE)
			.and_then(|h| h.to_str().ok())
			.and_then(Self::parse_cookies)
			.and_then(|parsed| {
				parsed.into_iter().find_map(|(name, value)| {
					if name == cookie_name {
						Some(value)
					} else {
						None
					}
				})
			})
			.filter(|lang| Self::is_valid_language_code(lang))
	}

	/// Parse cookie header with strict validation.
	///
	/// Rejects malformed cookies:
	/// - Missing `=` separator
	/// - Cookie name containing separators (`;`, `=`, whitespace, control chars)
	/// - Empty cookie name
	fn parse_cookies(header: &str) -> Option<Vec<(String, String)>> {
		let mut cookies = Vec::new();
		for cookie in header.split(';') {
			let cookie = cookie.trim();
			if cookie.is_empty() {
				continue;
			}
			// Cookie must contain '=' separator
			let mut parts = cookie.splitn(2, '=');
			let name = parts.next()?.trim();
			let value = match parts.next() {
				Some(v) => v.trim(),
				// Missing '=' means malformed cookie - skip it
				None => continue,
			};
			// Validate cookie name: must not be empty or contain separators/control chars
			if name.is_empty() || !Self::is_valid_cookie_name(name) {
				continue;
			}
			cookies.push((name.to_string(), value.to_string()));
		}
		Some(cookies)
	}

	/// Validate cookie name per RFC 6265.
	///
	/// Cookie name must not contain separators, whitespace, or control characters.
	fn is_valid_cookie_name(name: &str) -> bool {
		name.chars().all(|c| {
			// Must be a visible ASCII character (0x21-0x7E) excluding separators
			let code = c as u32;
			(0x21..=0x7E).contains(&code)
				&& !matches!(
					c,
					'(' | ')'
						| '<' | '>' | '@' | ','
						| ';' | ':' | '\\' | '"'
						| '/' | '[' | ']' | '?'
						| '=' | '{' | '}' | ' '
						| '\t'
				)
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	fn parse_query_params(uri: &hyper::Uri) -> HashMap<String, String> {
		uri.query().map(QueryParams::parse_raw).unwrap_or_default()
	}

	// =================================================================
	// Query parameter '=' preservation tests (Issue #362)
	// =================================================================

	#[rstest]
	fn test_query_params_parse_lazily() {
		// Arrange
		let uri: hyper::Uri = "/test?key=value".parse().unwrap();
		let params = QueryParams::from_uri(&uri);

		// Assert
		assert_eq!(params.raw_query(), Some("key=value"));
		assert!(!params.is_parsed());
		assert!(!params.has_raw_pair_cache());

		// Act
		let value = params.get("key");

		// Assert
		assert_eq!(value, Some("value"));
		assert!(!params.is_parsed());
		assert!(params.has_raw_pair_cache());

		// Act - repeated lookups reuse the cached raw ranges without building a map.
		let value = params.get("key");

		// Assert
		assert_eq!(value, Some("value"));
		assert!(!params.is_parsed());

		// Act
		let parsed = params.as_map();

		// Assert
		assert_eq!(parsed.get("key"), Some(&"value".to_string()));
		assert!(params.is_parsed());
	}

	#[rstest]
	fn test_query_params_clone_preserves_unparsed_state() {
		// Arrange
		let uri: hyper::Uri = "/test?key=value".parse().unwrap();
		let params = QueryParams::from_uri(&uri);

		// Act
		let cloned = params.clone();

		// Assert
		assert!(!params.is_parsed());
		assert!(!cloned.is_parsed());
		assert_eq!(cloned.get("key"), Some("value"));
		assert!(!params.is_parsed());
		assert!(!cloned.is_parsed());
	}

	#[rstest]
	fn test_query_params_get_uses_last_duplicate_value() {
		// Arrange
		let uri: hyper::Uri = "/test?tag=rust&tag=web&page=2".parse().unwrap();
		let params = QueryParams::from_uri(&uri);

		// Act + Assert
		assert_eq!(params.get("tag"), Some("web"));
		assert_eq!(params.get("page"), Some("2"));
		assert!(!params.is_parsed());
		assert!(params.has_raw_pair_cache());
	}

	#[rstest]
	fn test_query_params_clone_preserves_parsed_state() {
		// Arrange
		let uri: hyper::Uri = "/test?key=value".parse().unwrap();
		let params = QueryParams::from_uri(&uri);
		assert_eq!(params.as_map().get("key"), Some(&"value".to_string()));

		// Act
		let cloned = params.clone();

		// Assert
		assert!(cloned.is_parsed());
		assert_eq!(cloned.get("key"), Some("value"));
	}

	#[rstest]
	fn test_parse_query_params_preserves_equals_in_value() {
		// Arrange
		let uri: hyper::Uri = "/test?token=abc==".parse().unwrap();

		// Act
		let params = parse_query_params(&uri);

		// Assert
		assert_eq!(params.get("token"), Some(&"abc==".to_string()));
	}

	#[rstest]
	fn test_parse_query_params_base64_encoded_value() {
		// Arrange
		let uri: hyper::Uri = "/test?data=dGVzdA==".parse().unwrap();

		// Act
		let params = parse_query_params(&uri);

		// Assert
		assert_eq!(params.get("data"), Some(&"dGVzdA==".to_string()));
	}

	#[rstest]
	fn test_parse_query_params_multiple_equals_in_value() {
		// Arrange
		let uri: hyper::Uri = "/test?formula=a=b=c".parse().unwrap();

		// Act
		let params = parse_query_params(&uri);

		// Assert
		assert_eq!(params.get("formula"), Some(&"a=b=c".to_string()));
	}

	#[rstest]
	fn test_parse_query_params_simple_key_value() {
		// Arrange
		let uri: hyper::Uri = "/test?key=value".parse().unwrap();

		// Act
		let params = parse_query_params(&uri);

		// Assert
		assert_eq!(params.get("key"), Some(&"value".to_string()));
	}

	#[rstest]
	fn test_parse_query_params_key_without_value() {
		// Arrange
		let uri: hyper::Uri = "/test?key=".parse().unwrap();

		// Act
		let params = parse_query_params(&uri);

		// Assert
		assert_eq!(params.get("key"), Some(&"".to_string()));
	}

	#[rstest]
	fn test_parse_query_params_no_query_string() {
		// Arrange
		let uri: hyper::Uri = "/test".parse().unwrap();

		// Act
		let params = parse_query_params(&uri);

		// Assert
		assert!(params.is_empty());
	}

	#[rstest]
	fn test_parse_query_params_multiple_params_with_equals() {
		// Arrange
		let uri: hyper::Uri = "/test?a=1&b=x=y=z&c=3".parse().unwrap();

		// Act
		let params = parse_query_params(&uri);

		// Assert
		assert_eq!(params.get("a"), Some(&"1".to_string()));
		assert_eq!(params.get("b"), Some(&"x=y=z".to_string()));
		assert_eq!(params.get("c"), Some(&"3".to_string()));
	}
}
