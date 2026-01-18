//! Middleware for automatic API version detection
//!
//! This module provides middleware that automatically detects the API version
//! from requests and stores it in request extensions for easy access in handlers.

use crate::{BaseVersioning, VersioningError};
use async_trait::async_trait;
use reinhardt_core::exception::{Error, Result};
use reinhardt_http::{Request, Response};
use reinhardt_core::{Handler, Middleware};
use std::sync::Arc;

/// API version extracted from request
#[derive(Debug, Clone)]
pub struct ApiVersion(pub String);

impl ApiVersion {
	/// Get the version string as a string slice
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::versioning::ApiVersion;
	///
	/// let version = ApiVersion::new("2.0".to_string());
	/// assert_eq!(version.as_str(), "2.0");
	/// ```
	pub fn as_str(&self) -> &str {
		&self.0
	}

	/// Create a new ApiVersion with the given version string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::versioning::ApiVersion;
	///
	/// let version = ApiVersion::new("1.0".to_string());
	/// assert_eq!(version.as_str(), "1.0");
	/// ```
	pub fn new(version: String) -> Self {
		Self(version)
	}
}

impl std::fmt::Display for ApiVersion {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Middleware for automatic API version detection
///
/// This middleware uses a versioning strategy to automatically detect
/// the API version from incoming requests and stores it in request extensions.
///
/// # Example
///
/// ```rust
/// use reinhardt_rest::versioning::{URLPathVersioning, VersioningMiddleware};
///
/// let versioning = URLPathVersioning::new()
///     .with_default_version("1.0")
///     .with_allowed_versions(vec!["1.0", "2.0"]);
///
/// let middleware = VersioningMiddleware::new(versioning);
/// ```
pub struct VersioningMiddleware<V: BaseVersioning> {
	versioning: Arc<V>,
}

impl<V: BaseVersioning> VersioningMiddleware<V> {
	/// Create a new versioning middleware with the given versioning strategy
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::versioning::{URLPathVersioning, VersioningMiddleware};
	///
	/// let versioning = URLPathVersioning::new()
	///     .with_default_version("1.0");
	/// let middleware = VersioningMiddleware::new(versioning);
	/// ```
	pub fn new(versioning: V) -> Self {
		Self {
			versioning: Arc::new(versioning),
		}
	}
	/// Get a reference to the underlying versioning strategy
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::versioning::{URLPathVersioning, VersioningMiddleware, BaseVersioning};
	///
	/// let url_versioning = URLPathVersioning::new()
	///     .with_default_version("1.0");
	/// let middleware = VersioningMiddleware::new(url_versioning);
	///
	/// assert_eq!(middleware.versioning().default_version(), Some("1.0"));
	/// ```
	pub fn versioning(&self) -> &V {
		&self.versioning
	}
}

impl<V: BaseVersioning> Clone for VersioningMiddleware<V> {
	fn clone(&self) -> Self {
		Self {
			versioning: Arc::clone(&self.versioning),
		}
	}
}

#[async_trait]
impl<V: BaseVersioning + 'static> Middleware for VersioningMiddleware<V> {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		// Determine version from request
		let version = self
			.versioning
			.determine_version(&request)
			.await
			.map_err(|e| match e {
				Error::Validation(msg) => Error::Validation(msg),
				_ => Error::Validation(VersioningError::InvalidAcceptHeader.to_string()),
			})?;

		// Store version in request extensions
		request.extensions.insert(ApiVersion(version));

		// Call next handler
		next.handle(request).await
	}
}

/// Extension trait to get API version from request
pub trait RequestVersionExt {
	/// Get the API version from request extensions
	fn version(&self) -> Option<String>;

	/// Get the API version or return default
	fn version_or(&self, default: &str) -> String;
}

impl RequestVersionExt for Request {
	fn version(&self) -> Option<String> {
		self.extensions.get::<ApiVersion>().map(|v| v.0)
	}

	fn version_or(&self, default: &str) -> String {
		self.version().unwrap_or_else(|| default.to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{QueryParameterVersioning, URLPathVersioning};
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Uri, Version};

	fn create_test_request(uri: &str) -> Request {
		let uri = uri.parse::<Uri>().unwrap();
		Request::builder()
			.method(Method::GET)
			.uri(uri)
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	// Dummy handler for testing
	struct DummyHandler;

	#[async_trait]
	impl Handler for DummyHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::ok())
		}
	}

	#[tokio::test]
	async fn test_middleware_url_path_versioning() {
		let versioning = URLPathVersioning::new()
			.with_default_version("1.0")
			.with_allowed_versions(vec!["1.0", "2.0", "2"]);

		let middleware = VersioningMiddleware::new(versioning);
		let handler = Arc::new(DummyHandler);

		// Test with version in path
		let request = create_test_request("/v2/users/");
		let _ = middleware.process(request, handler.clone()).await.unwrap();

		// Test without version (should use default)
		let request = create_test_request("/users/");
		let _ = middleware.process(request, handler.clone()).await.unwrap();
	}

	#[tokio::test]
	async fn test_middleware_query_parameter_versioning() {
		let versioning = QueryParameterVersioning::new()
			.with_default_version("1.0")
			.with_allowed_versions(vec!["1.0", "2.0", "3.0"]);

		let middleware = VersioningMiddleware::new(versioning);
		let handler = Arc::new(DummyHandler);

		// Test with version in query
		let request = create_test_request("/users/?version=2.0");
		let _ = middleware.process(request, handler.clone()).await.unwrap();

		// Test without version (should use default)
		let request = create_test_request("/users/");
		let _ = middleware.process(request, handler.clone()).await.unwrap();
	}

	#[tokio::test]
	async fn test_request_version_extension() {
		let versioning = URLPathVersioning::new()
			.with_default_version("1.0")
			.with_allowed_versions(vec!["1.0", "2.0", "2"]);

		let middleware = VersioningMiddleware::new(versioning);
		let handler = Arc::new(DummyHandler);

		let request = create_test_request("/v2/users/");
		let _ = middleware.process(request, handler.clone()).await.unwrap();
	}

	#[tokio::test]
	async fn test_request_version_extension_with_default() {
		let request = create_test_request("/users/");

		// No version set, should return None
		assert_eq!(request.version(), None);

		// Should use provided default
		assert_eq!(request.version_or("fallback"), "fallback");
	}

	#[tokio::test]
	async fn test_middleware_invalid_version() {
		let versioning = URLPathVersioning::new()
			.with_default_version("1.0")
			.with_allowed_versions(vec!["1.0", "2.0"]);

		let middleware = VersioningMiddleware::new(versioning);
		let handler = Arc::new(DummyHandler);

		// Test with invalid version (not in allowed list)
		let request = create_test_request("/v3/users/");
		let result = middleware.process(request, handler.clone()).await;

		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_api_version_methods() {
		let version = ApiVersion("2.0".to_string());

		assert_eq!(version.as_str(), "2.0");
		assert_eq!(version.to_string(), "2.0");
	}
}
