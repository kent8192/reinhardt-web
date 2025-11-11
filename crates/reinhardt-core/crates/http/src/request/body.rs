use super::Request;
use bytes::Bytes;
#[cfg(feature = "parsers")]
use reinhardt_parsers::parser::{ParsedData, Parser};
#[cfg(feature = "parsers")]
use std::collections::HashMap;
use std::sync::atomic::Ordering;

impl Request {
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
	#[cfg(feature = "parsers")]
	pub fn with_parsers(mut self, parsers: Vec<Box<dyn Parser>>) -> Self {
		self.parsers = parsers;
		self
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
	#[cfg(feature = "parsers")]
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
	#[cfg(feature = "parsers")]
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
	#[cfg(feature = "parsers")]
	pub(super) async fn parse_body_internal(&self) -> crate::Result<ParsedData> {
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
				match parser
					.parse(content_type, self.body.clone(), &self.headers)
					.await
				{
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
