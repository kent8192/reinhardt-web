//! JSON body extraction

use async_trait::async_trait;
use reinhardt_http::Request;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug};
use std::ops::Deref;

use super::{
	ParamContext, ParamError, ParamErrorContext, ParamResult, ParamType, extract::FromRequest,
};

/// Default maximum JSON body size: 2 MiB
const DEFAULT_MAX_JSON_BODY_SIZE: usize = 2 * 1024 * 1024;

/// Extract and deserialize JSON from request body
///
/// # Example
///
/// ```rust
/// use reinhardt_di::params::Json;
/// # use serde::Deserialize;
/// #[derive(Deserialize)]
/// struct CreateUser {
///     username: String,
///     email: String,
/// }
///
/// let user_data = CreateUser {
///     username: "alice".to_string(),
///     email: "alice@example.com".to_string(),
/// };
/// let user = Json(user_data);
/// let username = &user.username;
/// let email = &user.email;
/// ```
pub struct Json<T>(pub T);

impl<T> Json<T> {
	/// Unwrap the Json and return the inner value
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_di::params::Json;
	/// use serde::Deserialize;
	///
	/// #[derive(Deserialize, Debug, PartialEq)]
	/// struct User {
	///     username: String,
	///     age: u32,
	/// }
	///
	/// let json = Json(User {
	///     username: "alice".to_string(),
	///     age: 30,
	/// });
	/// let inner = json.into_inner();
	/// assert_eq!(inner.username, "alice");
	/// assert_eq!(inner.age, 30);
	/// ```
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T> Deref for Json<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: Debug> Debug for Json<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl<T: Clone> Clone for Json<T> {
	fn clone(&self) -> Self {
		Json(self.0.clone())
	}
}

#[async_trait]
impl<T> FromRequest for Json<T>
where
	T: DeserializeOwned + Send,
{
	async fn from_request(req: &Request, ctx: &ParamContext) -> ParamResult<Self> {
		// Check Content-Type header (case-insensitive per RFC 7231)
		let content_type = req
			.headers
			.get(http::header::CONTENT_TYPE)
			.and_then(|h| h.to_str().ok())
			.unwrap_or("");

		// Allow empty Content-Type for backward compatibility,
		// but reject explicit non-JSON content types.
		// Normalize to lowercase for case-insensitive comparison.
		let ct_lower = content_type.to_lowercase();
		if !ct_lower.is_empty() && !ct_lower.contains("application/json") {
			return Err(ParamError::InvalidParameter(Box::new(
				ParamErrorContext::new(
					ParamType::Json,
					format!("Expected application/json, got {}", content_type),
				)
				.with_field("Content-Type")
				.with_expected_type::<T>(),
			)));
		}

		// Read body bytes through ParamContext's cache so that a second
		// `Json<T>` factory in the same request (e.g. resolving two DI
		// dependencies that each carry a body parameter — see #4645)
		// reuses the same bytes instead of failing with "body already
		// consumed" on the second call.
		let body_bytes = ctx.read_body_cached(req)?;

		// Enforce body size limit to prevent memory exhaustion
		if body_bytes.len() > DEFAULT_MAX_JSON_BODY_SIZE {
			return Err(ParamError::PayloadTooLarge(format!(
				"JSON body size {} bytes exceeds maximum allowed size of {} bytes",
				body_bytes.len(),
				DEFAULT_MAX_JSON_BODY_SIZE
			)));
		}

		// Deserialize JSON from body bytes with detailed error context
		serde_json::from_slice(&body_bytes).map(Json).map_err(|e| {
			let raw_value = String::from_utf8_lossy(&body_bytes).into_owned();
			ParamError::json_deserialization::<T>(e, Some(raw_value))
		})
	}
}

impl<T> super::has_inner::HasInner for Json<T> {
	type Inner = T;

	fn inner_ref(&self) -> &T {
		&self.0
	}

	fn into_inner(self) -> T {
		self.0
	}
}

// Bridge `Json<T>` to the DI container. The first factory in a request that
// resolves `Json<T>` will consume the body via `request.read_body()`; PR-2
// (Issue #4645) adds a body cache to `ParamContext` so that subsequent
// request-scoped `Json<T>` injections within the same request reuse the same
// bytes without re-consuming the underlying stream.
#[async_trait]
impl<T> crate::Injectable for Json<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
{
	async fn inject(ctx: &crate::InjectionContext) -> crate::DiResult<Self> {
		let request = ctx
			.get_http_request()
			.ok_or(crate::DiError::MissingParamContext { extractor: "Json" })?;
		let param_ctx = ctx
			.get_param_context()
			.ok_or(crate::DiError::MissingParamContext { extractor: "Json" })?;
		<Json<T> as FromRequest>::from_request(request, param_ctx)
			.await
			.map_err(crate::DiError::from_param_error)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version, header};
	use rstest::rstest;
	use serde::Deserialize;

	// Allow dead_code: fields are accessed via Deserialize derive, not directly in code
	#[allow(dead_code)]
	#[derive(Debug, Deserialize, PartialEq)]
	struct TestPayload {
		name: String,
	}

	fn build_request(content_type: Option<&str>, body: &str) -> Request {
		let mut headers = HeaderMap::new();
		if let Some(ct) = content_type {
			headers.insert(header::CONTENT_TYPE, ct.parse().unwrap());
		}
		Request::builder()
			.method(Method::POST)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::from(body.to_string()))
			.build()
			.unwrap()
	}

	#[rstest]
	#[tokio::test]
	async fn json_content_type_is_accepted() {
		// Arrange
		let req = build_request(Some("application/json"), r#"{"name":"Alice"}"#);
		let ctx = ParamContext::new();

		// Act
		let result = Json::<TestPayload>::from_request(&req, &ctx).await;

		// Assert
		assert!(result.is_ok());
		assert_eq!(result.unwrap().0.name, "Alice");
	}

	#[rstest]
	#[tokio::test]
	async fn json_content_type_with_charset_is_accepted() {
		// Arrange
		let req = build_request(Some("application/json; charset=utf-8"), r#"{"name":"Bob"}"#);
		let ctx = ParamContext::new();

		// Act
		let result = Json::<TestPayload>::from_request(&req, &ctx).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn missing_content_type_is_allowed_for_backward_compat() {
		// Arrange
		let req = build_request(None, r#"{"name":"Charlie"}"#);
		let ctx = ParamContext::new();

		// Act
		let result = Json::<TestPayload>::from_request(&req, &ctx).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn uppercase_json_content_type_is_accepted() {
		// Arrange
		let req = build_request(Some("Application/JSON"), r#"{"name":"Dave"}"#);
		let ctx = ParamContext::new();

		// Act
		let result = Json::<TestPayload>::from_request(&req, &ctx).await;

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	#[tokio::test]
	async fn form_urlencoded_content_type_is_rejected() {
		// Arrange
		let req = build_request(Some("application/x-www-form-urlencoded"), "name=Alice");
		let ctx = ParamContext::new();

		// Act
		let result = Json::<TestPayload>::from_request(&req, &ctx).await;

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(
			err_msg.contains("Expected application/json"),
			"Error should mention expected type, got: {err_msg}"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn text_plain_content_type_is_rejected() {
		// Arrange
		let req = build_request(Some("text/plain"), r#"{"name":"Eve"}"#);
		let ctx = ParamContext::new();

		// Act
		let result = Json::<TestPayload>::from_request(&req, &ctx).await;

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(
			err_msg.contains("text/plain"),
			"Error should include actual Content-Type, got: {err_msg}"
		);
	}
}
