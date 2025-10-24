use crate::extensions::Extensions;
use bytes::Bytes;
use hyper::{header::ACCEPT_LANGUAGE, HeaderMap, Method, Uri, Version};
use percent_encoding::percent_decode_str;
use reinhardt_parsers::parser::{ParsedData, Parser};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
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
            parsers: Vec::new(),
            parsed_data: Arc::new(Mutex::new(None)),
            body_consumed: Arc::new(AtomicBool::new(false)),
            extensions: Extensions::new(),
        }
    }

    fn parse_query_params(uri: &Uri) -> HashMap<String, String> {
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
    /// Parse the request body as JSON
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Request;
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// let json_body = r#"{"name": "Alice", "age": 30}"#;
    /// let request = Request::new(
    ///     Method::POST,
    ///     "/api/users".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::from(json_body)
    /// );
    ///
    /// let user: User = request.json().unwrap();
    /// assert_eq!(user.name, "Alice");
    /// assert_eq!(user.age, 30);
    /// ```
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> crate::Result<T> {
        use crate::Error;
        serde_json::from_slice(&self.body).map_err(|e| Error::Serialization(e.to_string()))
    }
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
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    // Direct HTTPS connection
    /// let request = Request::new_with_secure(
    ///     Method::GET,
    ///     "/".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::new(),
    ///     true
    /// );
    /// assert!(request.is_secure());
    ///
    // Behind reverse proxy
    /// let mut headers = HeaderMap::new();
    /// headers.insert("x-forwarded-proto", "https".parse().unwrap());
    /// let request = Request::new(
    ///     Method::GET,
    ///     "/".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     headers,
    ///     Bytes::new()
    /// );
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
    /// let request = Request::new_with_secure(
    ///     Method::GET,
    ///     "/".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::new(),
    ///     true
    /// );
    /// assert_eq!(request.scheme(), "https");
    ///
    /// let request = Request::new(
    ///     Method::GET,
    ///     "/".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::new()
    /// );
    /// assert_eq!(request.scheme(), "http");
    /// ```
    pub fn scheme(&self) -> &str {
        if self.is_secure() {
            "https"
        } else {
            "http"
        }
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
    /// let mut headers = HeaderMap::new();
    /// headers.insert("host", "example.com".parse().unwrap());
    ///
    /// let request = Request::new_with_secure(
    ///     Method::GET,
    ///     "/api/users".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     headers,
    ///     Bytes::new(),
    ///     true
    /// );
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
    /// Set parsers for request body parsing
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Request;
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    /// let request = Request::new(
    ///     Method::POST,
    ///     "/".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::new()
    /// );
    ///
    // Set up parsers (empty vec for this example)
    /// let request = request.with_parsers(vec![]);
    /// assert_eq!(request.method, Method::POST);
    /// ```
    pub fn with_parsers(mut self, parsers: Vec<Box<dyn Parser>>) -> Self {
        self.parsers = parsers;
        self
    }
    /// Get a reference to the request body
    ///
    /// This is a non-consuming accessor that can be called multiple times.
    /// Useful for testing and inspection purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Request;
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    /// let body = Bytes::from("test body");
    /// let request = Request::new(
    ///     Method::POST,
    ///     "/".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     body.clone()
    /// );
    ///
    /// assert_eq!(request.body(), &body);
    /// ```
    pub fn body(&self) -> &Bytes {
        &self.body
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
    /// Read and consume the request body
    /// This marks the body as consumed and subsequent parse attempts will fail
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Request;
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    /// let request = Request::new(
    ///     Method::POST,
    ///     "/".parse::<Uri>().unwrap(),
    ///     Version::HTTP_11,
    ///     HeaderMap::new(),
    ///     Bytes::from("request body")
    /// );
    ///
    /// let body = request.read_body().unwrap();
    /// assert_eq!(body, Bytes::from("request body"));
    ///
    // Second read fails because body is consumed
    /// assert!(request.read_body().is_err());
    /// ```
    pub fn read_body(&self) -> crate::Result<Bytes> {
        use crate::Error;
        if self.body_consumed.load(Ordering::SeqCst) {
            return Err(Error::Http(
                "Request body has already been consumed".to_string(),
            ));
        }
        self.body_consumed.store(true, Ordering::SeqCst);
        Ok(self.body.clone())
    }
    /// Get POST data (form-encoded data)
    /// Returns data only if using FormParser or MultiPartParser
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_http::Request;
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    /// async fn example() {
    ///     let request = Request::new(
    ///         Method::POST,
    ///         "/".parse::<Uri>().unwrap(),
    ///         Version::HTTP_11,
    ///         HeaderMap::new(),
    ///         Bytes::new()
    ///     );
    ///
    ///     // Without parsers, returns empty HashMap
    ///     let post_data = request.post().await.unwrap();
    ///     assert!(post_data.is_empty());
    /// }
    /// ```
    pub async fn post(&self) -> crate::Result<HashMap<String, Vec<String>>> {
        use crate::Error;
        if self.body_consumed.load(Ordering::SeqCst) {
            return Err(Error::Http(
                "Request body has already been consumed".to_string(),
            ));
        }

        // Check if we have form parsers
        let has_form_parser = self.parsers.iter().any(|p| {
            let media_types = p.media_types();
            media_types.contains(&"application/x-www-form-urlencoded".to_string())
                || media_types.contains(&"multipart/form-data".to_string())
        });

        if !has_form_parser {
            // No form parser, return empty
            return Ok(HashMap::new());
        }

        // Parse the body
        let parsed = self.parse_body_internal().await?;

        match parsed {
            ParsedData::Form(form) => {
                // Convert HashMap<String, String> to HashMap<String, Vec<String>>
                Ok(form.into_iter().map(|(k, v)| (k, vec![v])).collect())
            }
            ParsedData::MultiPart { fields, .. } => {
                // Convert fields to the expected format
                Ok(fields.into_iter().map(|(k, v)| (k, vec![v])).collect())
            }
            _ => Ok(HashMap::new()),
        }
    }
    /// Get parsed data
    /// This performs lazy parsing - only parses once and caches the result
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_http::Request;
    /// use hyper::{Method, Uri, Version, HeaderMap};
    /// use bytes::Bytes;
    ///
    /// async fn example() {
    ///     let request = Request::new(
    ///         Method::POST,
    ///         "/".parse::<Uri>().unwrap(),
    ///         Version::HTTP_11,
    ///         HeaderMap::new(),
    ///         Bytes::new()
    ///     );
    ///
    ///     // Without parsers, this will fail
    ///     assert!(request.data().await.is_err());
    /// }
    /// ```
    pub async fn data(&self) -> crate::Result<ParsedData> {
        use crate::Error;
        if self.body_consumed.load(Ordering::SeqCst) {
            return Err(Error::Http(
                "Request body has already been consumed".to_string(),
            ));
        }

        self.parse_body_internal().await
    }

    /// Internal method to parse body with caching
    async fn parse_body_internal(&self) -> crate::Result<ParsedData> {
        // Check cache first
        {
            let cache = self.parsed_data.lock().unwrap();
            if let Some(data) = &*cache {
                return Ok(data.clone());
            }
        }

        // Parse body
        let content_type = self
            .headers
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok());

        // Try each parser
        for parser in &self.parsers {
            if parser.can_parse(content_type) {
                match parser.parse(content_type, self.body.clone()).await {
                    Ok(data) => {
                        // Cache the result
                        let mut cache = self.parsed_data.lock().unwrap();
                        *cache = Some(data.clone());
                        return Ok(data);
                    }
                    Err(e) => {
                        use crate::Error;
                        return Err(Error::Http(format!("Parse error: {}", e)));
                    }
                }
            }
        }

        // No suitable parser found
        use crate::Error;
        Err(Error::Http(
            "No suitable parser found for content type".to_string(),
        ))
    }
}
