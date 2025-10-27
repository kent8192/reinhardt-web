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
/// use hyper::{Method, Uri, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::AUTHORIZATION,
///     "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".parse().unwrap()
/// );
///
/// let request = Request::new(
///     Method::GET,
///     "/".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     headers,
///     Bytes::new()
/// );
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
/// use hyper::{Method, Uri, Version, HeaderMap};
/// use bytes::Bytes;
///
/// let request = Request::new(
///     Method::GET,
///     "/".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     HeaderMap::new(),
///     Bytes::new()
/// );
///
/// let token = extract_bearer_token(&request);
/// assert_eq!(token, None);
/// ```
pub fn extract_bearer_token(request: &Request) -> Option<String> {
    request
        .headers
        .get(hyper::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|auth_str| {
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        })
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
/// use hyper::{Method, Uri, Version, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Pagination {
///     page: u32,
///     limit: u32,
/// }
///
/// let request = Request::new(
///     Method::GET,
///     "/api/users?page=2&limit=10".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     HeaderMap::new(),
///     Bytes::new()
/// );
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
/// use hyper::{Method, Uri, Version, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Pagination {
///     page: u32,
///     limit: u32,
/// }
///
/// let request = Request::new(
///     Method::GET,
///     "/api/users?page=invalid".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     HeaderMap::new(),
///     Bytes::new()
/// );
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
        .map_err(|e| Error::BadRequest(format!("Failed to parse query parameters: {}", e)))
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
/// use hyper::{Method, Uri, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::CONTENT_TYPE,
///     "application/json".parse().unwrap()
/// );
///
/// let request = Request::new(
///     Method::POST,
///     "/".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     headers,
///     Bytes::new()
/// );
///
/// assert!(validate_content_type(&request, "application/json").is_ok());
/// ```
///
/// # Content-Type mismatch
///
/// ```
/// use reinhardt_micro::utils::validate_content_type;
/// use reinhardt_micro::Request;
/// use hyper::{Method, Uri, Version, HeaderMap, header};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     header::CONTENT_TYPE,
///     "text/plain".parse().unwrap()
/// );
///
/// let request = Request::new(
///     Method::POST,
///     "/".parse::<Uri>().unwrap(),
///     Version::HTTP_11,
///     headers,
///     Bytes::new()
/// );
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
/// let result = validate_content_type(&request, "application/json");
/// assert!(result.is_err());
/// ```
pub fn validate_content_type(request: &Request, expected: &str) -> Result<()> {
    let content_type = request
        .headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| Error::BadRequest("Missing Content-Type header".to_string()))?;

    if content_type.starts_with(expected) {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "Invalid Content-Type: expected '{}', got '{}'",
            expected, content_type
        )))
    }
}
