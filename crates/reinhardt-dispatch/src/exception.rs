//! Exception handling and conversion for HTTP requests
//!
//! This module provides functionality to convert exceptions into HTTP responses,
//! similar to Django's `django.core.handlers.exception`.

use async_trait::async_trait;
use bytes::Bytes;
use hyper::StatusCode;
use reinhardt_core::http::{Request, Response};
use std::fmt;
use std::future::Future;
use tracing::{error, warn};

use crate::DispatchError;

/// Result type for exception handlers
pub type ExceptionResult = Result<Response, DispatchError>;

/// A trait for handling exceptions during request processing
#[async_trait]
pub trait ExceptionHandler: Send + Sync {
	/// Handle an exception and convert it to a response
	async fn handle_exception(&self, request: &Request, error: DispatchError) -> Response;
}

/// Default exception handler implementation
///
/// Converts exceptions to appropriate HTTP error responses.
pub struct DefaultExceptionHandler;

#[async_trait]
impl ExceptionHandler for DefaultExceptionHandler {
	async fn handle_exception(&self, _request: &Request, error: DispatchError) -> Response {
		let (status, message) = match &error {
			DispatchError::View(msg) => {
				warn!("View error: {}", msg);
				(StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
			}
			DispatchError::UrlResolution(msg) => {
				warn!("URL resolution error: {}", msg);
				(StatusCode::NOT_FOUND, msg.clone())
			}
			DispatchError::Middleware(msg) => {
				error!("Middleware error: {}", msg);
				(StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
			}
			DispatchError::Http(msg) => {
				warn!("HTTP error: {}", msg);
				(StatusCode::BAD_REQUEST, msg.clone())
			}
			DispatchError::Internal(msg) => {
				error!("Internal error: {}", msg);
				(StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
			}
		};

		let mut response = Response::new(status);
		response.body = Bytes::from(message.into_bytes());
		response
	}
}

/// Convert an exception to an HTTP response
///
/// This function wraps a handler that returns `Result<Response, DispatchError>`
/// and converts any errors into proper HTTP responses using the default exception handler.
pub async fn convert_exception_to_response<F, Fut>(handler: F, request: Request) -> Response
where
	F: FnOnce(Request) -> Fut,
	Fut: Future<Output = Result<Response, DispatchError>>,
{
	match handler(request).await {
		Ok(response) => response,
		Err(error) => {
			let exception_handler = DefaultExceptionHandler;
			// Create a dummy request for error handling since we consumed the original
			let dummy_request = Request::new(
				hyper::Method::GET,
				"/".parse().unwrap(),
				hyper::Version::HTTP_11,
				hyper::HeaderMap::new(),
				Bytes::new(),
			);
			exception_handler
				.handle_exception(&dummy_request, error)
				.await
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
				error!("Error converting to response: {}", error);
				let mut response = Response::new(StatusCode::INTERNAL_SERVER_ERROR);
				response.body = Bytes::from(error.to_string().into_bytes());
				response
			}
		}
	}
}
