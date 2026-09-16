//! Exception handling and conversion for HTTP requests
//!
//! This module provides functionality to convert exceptions into HTTP responses,
//! similar to Django's `django.core.handlers.exception`.

use async_trait::async_trait;
use bytes::Bytes;
use hyper::StatusCode;
use reinhardt_http::{ExceptionHandler as HttpExceptionHandler, Request, Response};
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use tracing::{error, warn};

use crate::DispatchError;
use crate::build_error_response;

/// Result type for exception handlers
pub type ExceptionResult = Result<Response, DispatchError>;

/// A compatibility hook for handling the dispatch-specific error categories.
///
/// This trait preserves the public contract exposed by `reinhardt-dispatch`
/// before the framework-wide HTTP exception hook was introduced. New server,
/// router, and middleware APIs use [`reinhardt_http::ExceptionHandler`]; pass a
/// legacy implementation through [`adapt_exception_handler`] when it must be
/// installed through one of those APIs.
#[async_trait]
pub trait ExceptionHandler: Send + Sync {
	/// Handle a dispatch error and convert it to a response.
	async fn handle_exception(&self, request: &Request, error: DispatchError) -> Response;
}

/// Convert an internal dispatch error into the unified framework error used by
/// [`HttpExceptionHandler`].
pub(crate) fn dispatch_error_to_exception(
	error: DispatchError,
) -> reinhardt_core::exception::Error {
	match error {
		DispatchError::Middleware(message)
		| DispatchError::View(message)
		| DispatchError::Internal(message) => reinhardt_core::exception::Error::Internal(message),
		DispatchError::UrlResolution(message) => {
			reinhardt_core::exception::Error::NotFound(message)
		}
		DispatchError::Http(message) => reinhardt_core::exception::Error::Http(message),
	}
}

/// Convert a framework error into the legacy dispatch error categories used by
/// [`BaseHandler::handle_request`](crate::BaseHandler::handle_request).
pub(crate) fn exception_to_dispatch_error(
	error: reinhardt_core::exception::Error,
) -> DispatchError {
	match error {
		reinhardt_core::exception::Error::NotFound(message) => {
			DispatchError::UrlResolution(message)
		}
		reinhardt_core::exception::Error::Http(message) => DispatchError::Http(message),
		error => DispatchError::View(error.to_string()),
	}
}

/// Adapts a legacy [`ExceptionHandler`] to the framework-wide HTTP hook.
///
/// `Error::NotFound` and `Error::Http` retain their corresponding legacy
/// categories. Error variants without a corresponding [`DispatchError`]
/// variant are represented as legacy view errors. The original dispatch
/// categories remain available to existing implementations, while new code
/// should implement [`reinhardt_http::ExceptionHandler`] directly.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use async_trait::async_trait;
/// use hyper::StatusCode;
/// use reinhardt_core::exception::Error;
/// use reinhardt_dispatch::{adapt_exception_handler, DispatchError, ExceptionHandler};
/// use reinhardt_http::{Request, Response};
///
/// struct MyDispatchErrors;
///
/// #[async_trait]
/// impl ExceptionHandler for MyDispatchErrors {
///     async fn handle_exception(&self, _request: &Request, error: DispatchError) -> Response {
///         let status = match error {
///             DispatchError::UrlResolution(_) => StatusCode::NOT_FOUND,
///             _ => StatusCode::INTERNAL_SERVER_ERROR,
///         };
///         Response::new(status)
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let legacy: Arc<dyn ExceptionHandler> = Arc::new(MyDispatchErrors);
///     let http_handler = adapt_exception_handler(legacy);
///     let request = Request::builder().uri("/missing").build().unwrap();
///     let response = http_handler
///         .handle_exception(&request, Error::NotFound("route not found".into()))
///         .await;
///     assert_eq!(response.status, StatusCode::NOT_FOUND);
/// }
/// ```
pub fn adapt_exception_handler(
	handler: Arc<dyn ExceptionHandler>,
) -> Arc<dyn HttpExceptionHandler> {
	Arc::new(LegacyExceptionHandlerAdapter { handler })
}

struct LegacyExceptionHandlerAdapter {
	handler: Arc<dyn ExceptionHandler>,
}

#[async_trait]
impl HttpExceptionHandler for LegacyExceptionHandlerAdapter {
	async fn handle_exception(
		&self,
		request: &Request,
		error: reinhardt_core::exception::Error,
	) -> Response {
		self.handler
			.handle_exception(request, exception_to_dispatch_error(error))
			.await
	}
}

/// Default exception handler implementation
///
/// Converts exceptions to appropriate HTTP error responses.
pub struct DefaultExceptionHandler;

#[async_trait]
impl HttpExceptionHandler for DefaultExceptionHandler {
	async fn handle_exception(
		&self,
		_request: &Request,
		error: reinhardt_core::exception::Error,
	) -> Response {
		// Internal error details are logged server-side but never exposed
		// in HTTP response bodies to prevent information disclosure.
		if error.status_code() >= 500 {
			error!("Dispatch error: {}", error);
		} else {
			warn!("Dispatch error: {}", error);
		}
		let status =
			StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
		let client_message = match status {
			StatusCode::BAD_REQUEST => "Bad Request",
			StatusCode::UNAUTHORIZED => "Unauthorized",
			StatusCode::FORBIDDEN => "Forbidden",
			StatusCode::NOT_FOUND => "Not Found",
			StatusCode::METHOD_NOT_ALLOWED => "Method Not Allowed",
			StatusCode::CONFLICT => "Conflict",
			_ => "Internal Server Error",
		};

		build_error_response(status, client_message)
	}
}

#[async_trait]
impl ExceptionHandler for DefaultExceptionHandler {
	async fn handle_exception(&self, request: &Request, error: DispatchError) -> Response {
		HttpExceptionHandler::handle_exception(self, request, dispatch_error_to_exception(error))
			.await
	}
}

/// Convert an exception to an HTTP response
///
/// This function wraps a handler that returns `Result<Response, DispatchError>`
/// and converts any errors into proper HTTP responses using the default exception handler.
///
/// The request's method, URI, version, and headers are preserved before
/// passing ownership to the handler, so that the exception handler retains
/// the original request context (headers, auth info) for context-aware error
/// responses.
pub async fn convert_exception_to_response<F, Fut>(handler: F, request: Request) -> Response
where
	F: FnOnce(Request) -> Fut,
	Fut: Future<Output = Result<Response, DispatchError>>,
{
	// Capture the request context before consuming the request,
	// so the exception handler has access to headers and auth info.
	let method = request.method.clone();
	let uri = request.uri.clone();
	let version = request.version;
	let headers = request.headers.clone();

	match handler(request).await {
		Ok(response) => response,
		Err(error) => {
			let exception_handler = DefaultExceptionHandler;
			// Reconstruct a request with the original context for error handling
			match Request::builder()
				.method(method)
				.uri(uri.to_string())
				.version(version)
				.headers(headers)
				.body(Bytes::new())
				.build()
			{
				Ok(context_request) => {
					HttpExceptionHandler::handle_exception(
						&exception_handler,
						&context_request,
						dispatch_error_to_exception(error),
					)
					.await
				}
				Err(_) => {
					let mut response = Response::new(hyper::StatusCode::INTERNAL_SERVER_ERROR);
					response.body = Bytes::from("Internal Server Error");
					response
				}
			}
		}
	}
}

/// Trait for types that can be converted into HTTP responses
pub trait IntoResponse {
	/// Convert self into an HTTP response
	fn into_response(self) -> Response;
}

impl IntoResponse for Response {
	fn into_response(self) -> Response {
		self
	}
}

impl IntoResponse for String {
	fn into_response(self) -> Response {
		let mut response = Response::new(StatusCode::OK);
		response.body = Bytes::from(self.into_bytes());
		response
	}
}

impl IntoResponse for &str {
	fn into_response(self) -> Response {
		let mut response = Response::new(StatusCode::OK);
		response.body = Bytes::from(self.as_bytes().to_vec());
		response
	}
}

impl IntoResponse for Vec<u8> {
	fn into_response(self) -> Response {
		let mut response = Response::new(StatusCode::OK);
		response.body = Bytes::from(self);
		response
	}
}

impl IntoResponse for StatusCode {
	fn into_response(self) -> Response {
		Response::new(self)
	}
}

impl<T: IntoResponse, E: fmt::Display> IntoResponse for Result<T, E> {
	fn into_response(self) -> Response {
		match self {
			Ok(value) => value.into_response(),
			Err(error) => {
				// Log the error details server-side only; never expose in response body
				error!("Error converting to response: {}", error);
				build_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_http::ExceptionHandler as HttpExceptionHandler;
	use rstest::rstest;
	use std::sync::Arc;

	fn build_request() -> Request {
		Request::builder()
			.method(hyper::Method::GET)
			.uri("/")
			.version(hyper::Version::HTTP_11)
			.headers(hyper::HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	// ==========================================================================
	// Information Disclosure Prevention Tests (#439)
	// ==========================================================================

	#[tokio::test]
	async fn test_internal_error_does_not_expose_details() {
		// Arrange
		let handler = DefaultExceptionHandler;
		let request = build_request();
		let error = dispatch_error_to_exception(DispatchError::Internal(
			"database pool exhausted at /src/db/pool.rs:99".to_string(),
		));

		// Act
		let response = HttpExceptionHandler::handle_exception(&handler, &request, error).await;

		// Assert: generic message only, no internal details
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
		assert_eq!(body, "Internal Server Error");
		assert!(!body.contains("database"));
		assert!(!body.contains(".rs:"));
	}

	#[tokio::test]
	async fn test_middleware_error_does_not_expose_details() {
		// Arrange
		let handler = DefaultExceptionHandler;
		let request = build_request();
		let error = dispatch_error_to_exception(DispatchError::Middleware(
			"JWT decode failed: invalid signature for key abc123".to_string(),
		));

		// Act
		let response = HttpExceptionHandler::handle_exception(&handler, &request, error).await;

		// Assert: generic message only, no internal details
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
		assert_eq!(body, "Internal Server Error");
		assert!(!body.contains("JWT"));
		assert!(!body.contains("abc123"));
	}

	#[tokio::test]
	async fn test_view_error_does_not_expose_details() {
		// Arrange
		let handler = DefaultExceptionHandler;
		let request = build_request();
		let error = dispatch_error_to_exception(DispatchError::View(
			"template rendering panicked at /src/views/admin.rs:42".to_string(),
		));

		// Act
		let response = HttpExceptionHandler::handle_exception(&handler, &request, error).await;

		// Assert: generic message only, no internal details
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
		assert_eq!(body, "Internal Server Error");
		assert!(!body.contains("panicked"));
		assert!(!body.contains(".rs:"));
	}

	#[tokio::test]
	async fn test_url_resolution_returns_not_found() {
		// Arrange
		let handler = DefaultExceptionHandler;
		let request = build_request();
		let error = dispatch_error_to_exception(DispatchError::UrlResolution(
			"no route matched".to_string(),
		));

		// Act
		let response = HttpExceptionHandler::handle_exception(&handler, &request, error).await;

		// Assert
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::NOT_FOUND);
		assert_eq!(body, "Not Found");
	}

	#[tokio::test]
	async fn test_http_error_returns_bad_request() {
		// Arrange
		let handler = DefaultExceptionHandler;
		let request = build_request();
		let error =
			dispatch_error_to_exception(DispatchError::Http("malformed header".to_string()));

		// Act
		let response = HttpExceptionHandler::handle_exception(&handler, &request, error).await;

		// Assert
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::BAD_REQUEST);
		assert_eq!(body, "Bad Request");
	}

	#[rstest]
	#[tokio::test]
	async fn legacy_exception_handler_can_be_adapted_to_http_hook() {
		// Arrange
		struct LegacyTeapot;

		#[async_trait]
		impl ExceptionHandler for LegacyTeapot {
			async fn handle_exception(&self, _request: &Request, error: DispatchError) -> Response {
				assert!(matches!(error, DispatchError::UrlResolution(_)));
				Response::new(StatusCode::IM_A_TEAPOT)
			}
		}

		let request = build_request();
		let handler = adapt_exception_handler(Arc::new(LegacyTeapot));

		// Act
		let response = HttpExceptionHandler::handle_exception(
			handler.as_ref(),
			&request,
			reinhardt_core::exception::Error::NotFound("missing".to_owned()),
		)
		.await;

		// Assert
		assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
	}

	#[rstest]
	#[tokio::test]
	async fn legacy_exception_handler_preserves_http_error_category() {
		// Arrange
		struct LegacyHttp;

		#[async_trait]
		impl ExceptionHandler for LegacyHttp {
			async fn handle_exception(&self, _request: &Request, error: DispatchError) -> Response {
				assert!(matches!(error, DispatchError::Http(_)));
				Response::new(StatusCode::IM_A_TEAPOT)
			}
		}

		let request = build_request();
		let handler = adapt_exception_handler(Arc::new(LegacyHttp));

		// Act
		let response = HttpExceptionHandler::handle_exception(
			handler.as_ref(),
			&request,
			reinhardt_core::exception::Error::Http("malformed header".to_owned()),
		)
		.await;

		// Assert
		assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
	}

	#[test]
	fn test_into_response_for_result_err_does_not_expose_error() {
		// Arrange
		let result: Result<String, String> =
			Err("connection string: postgres://admin:pass@host/db".to_string());

		// Act
		let response = result.into_response();

		// Assert
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
		assert!(!body.contains("postgres"));
		assert!(!body.contains("admin"));
		assert_eq!(body, "Internal Server Error");
	}
}
