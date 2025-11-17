//! Request helper utilities for common patterns
//!
//! This module provides convenient helper functions for extracting and validating
//! common request data in microservices.

use crate::{Error, Request, Result};
use serde::de::DeserializeOwned;

/// Extract Bearer token from Authorization header
///
/// Extracts JWT or other bearer tokens from the Authorization header.
/// Returns `None` if the header is missing or not in "Bearer <token>" format.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::extract_bearer_token;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::AUTHORIZATION,
///     "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".parse().unwrap()
/// );
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(headers)
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let token = extract_bearer_token(&request);
/// assert_eq!(token, Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string()));
/// ```
///
/// # Missing or invalid header
///
/// ```
/// use reinhardt_micro::utils::extract_bearer_token;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap};
/// use bytes::Bytes;
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let token = extract_bearer_token(&request);
/// assert_eq!(token, None);
/// ```
pub fn extract_bearer_token(request: &Request) -> Option<String> {
	request
		.headers
		.get(hyper::header::AUTHORIZATION)
		.and_then(|value| value.to_str().ok())
		.and_then(|auth_str| auth_str.strip_prefix("Bearer ").map(|s| s.to_string()))
}

/// Parse query parameters into a typed structure
///
/// Deserializes query string parameters into the specified type `T`.
/// Returns an error if deserialization fails.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::parse_query_params;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Pagination {
///     page: u32,
///     limit: u32,
/// }
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/api/users?page=2&limit=10")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let params: Pagination = parse_query_params(&request).unwrap();
/// assert_eq!(params, Pagination { page: 2, limit: 10 });
/// ```
///
/// # Type mismatch error
///
/// ```
/// use reinhardt_micro::utils::parse_query_params;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Pagination {
///     page: u32,
///     limit: u32,
/// }
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/api/users?page=invalid")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let result: Result<Pagination, _> = parse_query_params(&request);
/// assert!(result.is_err());
/// ```
pub fn parse_query_params<T: DeserializeOwned>(request: &Request) -> Result<T> {
	// Convert HashMap<String, String> to Vec<(String, String)> for serde_urlencoded
	let params: Vec<(String, String)> = request
		.query_params
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	serde_urlencoded::from_str(&serde_urlencoded::to_string(&params).unwrap())
		.map_err(|e| Error::Http(format!("Failed to parse query parameters: {}", e)))
}

/// Validate Content-Type header
///
/// Checks if the Content-Type header matches the expected value.
/// Returns an error if the header is missing or doesn't match.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::validate_content_type;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::CONTENT_TYPE,
///     "application/json".parse().unwrap()
/// );
///
/// let request = Request::builder()
///     .method(Method::POST)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(headers)
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// assert!(validate_content_type(&request, "application/json").is_ok());
/// ```
///
/// # Content-Type mismatch
///
/// ```
/// use reinhardt_micro::utils::validate_content_type;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::CONTENT_TYPE,
///     "text/plain".parse().unwrap()
/// );
///
/// let request = Request::builder()
///     .method(Method::POST)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(headers)
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let result = validate_content_type(&request, "application/json");
/// assert!(result.is_err());
/// ```
///
/// # Missing Content-Type header
///
/// ```
/// use reinhardt_micro::utils::validate_content_type;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap};
/// use bytes::Bytes;
///
/// let request = Request::builder()
///     .method(Method::POST)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let result = validate_content_type(&request, "application/json");
/// assert!(result.is_err());
/// ```
pub fn validate_content_type(request: &Request, expected: &str) -> Result<()> {
	let content_type = request
		.headers
		.get(hyper::header::CONTENT_TYPE)
		.and_then(|value| value.to_str().ok())
		.ok_or_else(|| Error::Http("Missing Content-Type header".to_string()))?;

	if content_type.starts_with(expected) {
		Ok(())
	} else {
		Err(Error::Http(format!(
			"Invalid Content-Type: expected '{}', got '{}'",
			expected, content_type
		)))
	}
}

/// Get a specific header value from the request
///
/// Returns `None` if the header is missing or cannot be converted to a string.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::get_header;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::USER_AGENT,
///     "Mozilla/5.0".parse().unwrap()
/// );
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(headers)
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let user_agent = get_header(&request, "user-agent");
/// assert_eq!(user_agent, Some("Mozilla/5.0".to_string()));
/// ```
///
/// # Missing header
///
/// ```
/// use reinhardt_micro::utils::get_header;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap};
/// use bytes::Bytes;
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let header = get_header(&request, "x-custom-header");
/// assert_eq!(header, None);
/// ```
pub fn get_header(request: &Request, name: &str) -> Option<String> {
	request
		.headers
		.get(name)
		.and_then(|value| value.to_str().ok())
		.map(|s| s.to_string())
}

/// Extract client IP address from the request
///
/// Attempts to extract the client IP address from the request headers
/// (X-Forwarded-For, X-Real-IP) or the remote address.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::get_client_ip;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::HeaderName::from_static("x-forwarded-for"),
///     "203.0.113.1, 198.51.100.1".parse().unwrap()
/// );
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(headers)
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let ip = get_client_ip(&request);
/// assert_eq!(ip, Some("203.0.113.1".parse().unwrap()));
/// ```
///
/// # No IP headers present
///
/// ```
/// use reinhardt_micro::utils::get_client_ip;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Version, HeaderMap};
/// use bytes::Bytes;
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("/")
///     .version(Version::HTTP_11)
///     .headers(HeaderMap::new())
///     .body(Bytes::new())
///     .build()
///     .unwrap();
///
/// let ip = get_client_ip(&request);
/// assert_eq!(ip, None);
/// ```
pub fn get_client_ip(request: &Request) -> Option<std::net::IpAddr> {
	// Try X-Forwarded-For header first (common in proxy setups)
	if let Some(forwarded) = request
		.headers
		.get("x-forwarded-for")
		.and_then(|v| v.to_str().ok())
	{
		// X-Forwarded-For can contain multiple IPs, take the first one
		if let Some(first_ip) = forwarded.split(',').next()
			&& let Ok(ip) = first_ip.trim().parse()
		{
			return Some(ip);
		}
	}

	// Try X-Real-IP header
	if let Some(real_ip) = request
		.headers
		.get("x-real-ip")
		.and_then(|v| v.to_str().ok())
		&& let Ok(ip) = real_ip.parse()
	{
		return Some(ip);
	}

	// If no proxy headers, we'd normally check the socket address,
	// but since Request doesn't store that, return None
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version, header};

	#[test]
	fn test_extract_bearer_token() {
		let mut headers = HeaderMap::new();
		headers.insert(
			header::AUTHORIZATION,
			"Bearer test_token_123".parse().unwrap(),
		);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let token = extract_bearer_token(&request);
		assert_eq!(token, Some("test_token_123".to_string()));
	}

	#[test]
	fn test_extract_bearer_token_missing() {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let token = extract_bearer_token(&request);
		assert_eq!(token, None);
	}

	#[test]
	fn test_get_header() {
		let mut headers = HeaderMap::new();
		headers.insert(header::USER_AGENT, "TestClient/1.0".parse().unwrap());

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let user_agent = get_header(&request, "user-agent");
		assert_eq!(user_agent, Some("TestClient/1.0".to_string()));
	}

	#[test]
	fn test_get_header_missing() {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let header = get_header(&request, "x-custom-header");
		assert_eq!(header, None);
	}

	#[test]
	fn test_get_client_ip_forwarded_for() {
		let mut headers = HeaderMap::new();
		headers.insert(
			header::HeaderName::from_static("x-forwarded-for"),
			"192.168.1.1, 10.0.0.1".parse().unwrap(),
		);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let ip = get_client_ip(&request);
		assert_eq!(ip, Some("192.168.1.1".parse().unwrap()));
	}

	#[test]
	fn test_get_client_ip_real_ip() {
		let mut headers = HeaderMap::new();
		headers.insert(
			header::HeaderName::from_static("x-real-ip"),
			"203.0.113.5".parse().unwrap(),
		);

		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		let ip = get_client_ip(&request);
		assert_eq!(ip, Some("203.0.113.5".parse().unwrap()));
	}

	#[test]
	fn test_get_client_ip_none() {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let ip = get_client_ip(&request);
		assert_eq!(ip, None);
	}

	#[test]
	fn test_validate_content_type_valid() {
		let mut headers = HeaderMap::new();
		headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

		let request = Request::builder()
			.method(Method::POST)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		assert!(validate_content_type(&request, "application/json").is_ok());
	}

	#[test]
	fn test_validate_content_type_invalid() {
		let mut headers = HeaderMap::new();
		headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());

		let request = Request::builder()
			.method(Method::POST)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::new())
			.build()
			.unwrap();

		assert!(validate_content_type(&request, "application/json").is_err());
	}

	#[test]
	fn test_validate_content_type_missing() {
		let request = Request::builder()
			.method(Method::POST)
			.uri("/")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		assert!(validate_content_type(&request, "application/json").is_err());
	}
}
