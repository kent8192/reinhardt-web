//! Exception handling extension point for dispatch errors.
//!
//! This module provides the [`ExceptionHandler`] hook and
//! [`ExceptionHandlingHandler`], the adapter that applies it to a handler chain.
//!
//! Without an installed handler every error is converted by
//! `impl From<Error> for Response`, which omits internal details and emits a
//! plain-text body. Installing a handler replaces that conversion so an
//! application can present a fixed error shape to its clients.

use async_trait::async_trait;
use std::sync::Arc;

use crate::{Error, Handler, Request, Response, Result};

/// A strategy for turning a dispatch error into an HTTP response.
///
/// Install an implementation with `with_exception_handler` on the router, the
/// server, or a middleware chain. Every error the framework produces while
/// serving a request then flows through this method instead of the default
/// `impl From<Error> for Response` conversion.
///
/// # Responsibility
///
/// Installing a handler transfers responsibility for the response body and
/// headers to the handler. The default conversion never exposes internal
/// details and sets `Content-Type: text/plain; charset=utf-8` together with
/// `X-Content-Type-Options: nosniff`; a custom handler provides none of these
/// unless it sets them itself. Interpolating `Display` output of the error into
/// a response body can disclose internal paths and credentials.
///
/// # Panics
///
/// A panicking handler is not caught here. The request's connection task fails
/// and the server process stays alive.
///
/// # Examples
///
/// ```
/// use async_trait::async_trait;
/// use hyper::StatusCode;
/// use reinhardt_http::{Error, ExceptionHandler, Request, Response};
///
/// struct JsonErrors;
///
/// #[async_trait]
/// impl ExceptionHandler for JsonErrors {
///     async fn handle_exception(&self, _request: &Request, error: Error) -> Response {
///         let status = StatusCode::from_u16(error.status_code())
///             .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
///         Response::new(status).with_body("error")
///     }
/// }
/// ```
#[async_trait]
pub trait ExceptionHandler: Send + Sync + 'static {
	/// Builds the response for `error`.
	///
	/// `request` carries the method, URI, version, headers, path parameters,
	/// query parameters and extensions of the original request, which is
	/// exactly what [`Request::clone_for_di`] preserves. Its body is empty
	/// because the original request body has already been consumed by the
	/// handler that produced the error.
	async fn handle_exception(&self, request: &Request, error: Error) -> Response;
}

/// Applies an [`ExceptionHandler`] to the errors produced by an inner handler.
///
/// Wrap the innermost handler of a chain with this adapter to route its errors
/// through the installed handler. The adapter always yields `Ok`, so an outer
/// wrapper that converts errors cannot observe them.
///
/// This adapter sees only errors that reach it as `Err`. Errors raised by
/// middleware inside a [`MiddlewareChain`](crate::MiddlewareChain) are converted
/// by that chain, which applies its own installed handler instead.
///
/// # Examples
///
/// ```
/// use async_trait::async_trait;
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, StatusCode, Version};
/// use reinhardt_http::{
///     Error, ExceptionHandler, ExceptionHandlingHandler, Handler, Request, Response,
/// };
/// use std::sync::Arc;
///
/// struct FailingHandler;
///
/// #[async_trait]
/// impl Handler for FailingHandler {
///     async fn handle(&self, _request: Request) -> reinhardt_http::Result<Response> {
///         Err(Error::NotFound("no route".to_string()))
///     }
/// }
///
/// struct TeapotErrors;
///
/// #[async_trait]
/// impl ExceptionHandler for TeapotErrors {
///     async fn handle_exception(&self, _request: &Request, _error: Error) -> Response {
///         Response::new(StatusCode::IM_A_TEAPOT)
///     }
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// let handler = ExceptionHandlingHandler::new(
///     Arc::new(FailingHandler),
///     Arc::new(TeapotErrors),
/// );
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
/// let response = handler.handle(request).await.unwrap();
/// assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
/// # }
/// ```
pub struct ExceptionHandlingHandler {
	inner: Arc<dyn Handler>,
	exception_handler: Arc<dyn ExceptionHandler>,
}

impl ExceptionHandlingHandler {
	/// Creates an adapter that converts `inner`'s errors with `exception_handler`.
	pub fn new(inner: Arc<dyn Handler>, exception_handler: Arc<dyn ExceptionHandler>) -> Self {
		Self {
			inner,
			exception_handler,
		}
	}
}

#[async_trait]
impl Handler for ExceptionHandlingHandler {
	async fn handle(&self, request: Request) -> Result<Response> {
		// `Request` is not `Clone` because it owns parsed-body state. Capture the
		// context with `clone_for_di`, which copies method, URI, version, headers,
		// path parameters and query parameters, and shares the extensions store
		// (auth state, DI context) through an internal `Arc`. This cost is paid
		// only where a custom handler is installed, because this adapter is only
		// constructed in that case.
		let context = request.clone_for_di();
		match self.inner.handle(request).await {
			Ok(response) => Ok(response),
			Err(error) => Ok(self
				.exception_handler
				.handle_exception(&context, error)
				.await),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Version};
	use rstest::rstest;
	use std::sync::Mutex;

	/// Marker type stored in the request's DI context.
	#[derive(Debug, PartialEq, Eq)]
	struct MarkerContext(&'static str);

	/// What an exception handler observed about the request it was given.
	#[derive(Debug, PartialEq, Eq)]
	struct ObservedRequest {
		method: Method,
		path: String,
		request_id: Option<String>,
		item_id: Option<String>,
		di_marker: Option<&'static str>,
	}

	/// Exception handler that records the request context it receives.
	struct ObservingHandler {
		observed: Arc<Mutex<Vec<ObservedRequest>>>,
	}

	#[async_trait]
	impl ExceptionHandler for ObservingHandler {
		async fn handle_exception(&self, request: &Request, error: Error) -> Response {
			let di_marker = request
				.get_di_context::<MarkerContext>()
				.map(|marker| marker.0);
			self.observed.lock().unwrap().push(ObservedRequest {
				method: request.method.clone(),
				path: request.uri.path().to_string(),
				request_id: request.get_header("x-request-id"),
				item_id: request.path_params.get("id").cloned(),
				di_marker,
			});

			// Echo the error so the assertion can prove the error travelled through.
			Response::new(StatusCode::IM_A_TEAPOT).with_body(error.to_string())
		}
	}

	/// Handler that always fails with the error produced by `factory`.
	///
	/// `Error` is not `Clone`, so the factory builds a fresh value per call.
	struct FailingHandler {
		factory: fn() -> Error,
	}

	#[async_trait]
	impl Handler for FailingHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Err((self.factory)())
		}
	}

	/// Handler that always succeeds.
	struct OkHandler;

	#[async_trait]
	impl Handler for OkHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			Ok(Response::ok().with_body("ok"))
		}
	}

	/// Handler that counts how many times it ran.
	struct CountingHandler {
		calls: Arc<Mutex<usize>>,
	}

	#[async_trait]
	impl Handler for CountingHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			*self.calls.lock().unwrap() += 1;
			Err(Error::Internal("boom".to_string()))
		}
	}

	fn build_request(method: Method, uri: &str) -> Request {
		Request::builder()
			.method(method)
			.uri(uri)
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap()
	}

	#[rstest]
	#[tokio::test]
	async fn test_inner_error_is_converted_by_installed_handler() {
		// Arrange
		let observed = Arc::new(Mutex::new(Vec::new()));
		let handler = ExceptionHandlingHandler::new(
			Arc::new(FailingHandler {
				factory: || Error::NotFound("no route".to_string()),
			}),
			Arc::new(ObservingHandler {
				observed: Arc::clone(&observed),
			}),
		);
		let request = build_request(Method::GET, "/missing");

		// Act
		let response = handler.handle(request).await.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
		assert_eq!(observed.lock().unwrap().len(), 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_handler_receives_original_request_context() {
		// Arrange
		let observed = Arc::new(Mutex::new(Vec::new()));
		let handler = ExceptionHandlingHandler::new(
			Arc::new(FailingHandler {
				factory: || Error::Internal("boom".to_string()),
			}),
			Arc::new(ObservingHandler {
				observed: Arc::clone(&observed),
			}),
		);

		let mut headers = HeaderMap::new();
		headers.insert("x-request-id", "req-42".parse().unwrap());
		let mut request = Request::builder()
			.method(Method::POST)
			.uri("/api/items/7")
			.version(Version::HTTP_11)
			.headers(headers)
			.body(Bytes::from_static(b"payload"))
			.build()
			.unwrap();
		request.path_params.insert("id", "7");

		// Act
		handler.handle(request).await.unwrap();

		// Assert
		let observed = observed.lock().unwrap();
		assert_eq!(observed.len(), 1);
		assert_eq!(observed[0].method, Method::POST);
		assert_eq!(observed[0].path, "/api/items/7");
		assert_eq!(observed[0].request_id, Some("req-42".to_string()));
		assert_eq!(observed[0].item_id, Some("7".to_string()));
	}

	#[rstest]
	#[tokio::test]
	async fn test_handler_shares_request_extensions() {
		// Arrange
		let observed = Arc::new(Mutex::new(Vec::new()));
		let handler = ExceptionHandlingHandler::new(
			Arc::new(FailingHandler {
				factory: || Error::Internal("boom".to_string()),
			}),
			Arc::new(ObservingHandler {
				observed: Arc::clone(&observed),
			}),
		);
		let mut request = build_request(Method::GET, "/");
		request.set_di_context(MarkerContext("di-visible"));

		// Act
		handler.handle(request).await.unwrap();

		// Assert: the extensions store is shared with the context request
		let observed = observed.lock().unwrap();
		assert_eq!(observed.len(), 1);
		assert_eq!(observed[0].di_marker, Some("di-visible"));
	}

	#[rstest]
	#[tokio::test]
	async fn test_handler_is_not_invoked_for_successful_response() {
		// Arrange
		let observed = Arc::new(Mutex::new(Vec::new()));
		let handler = ExceptionHandlingHandler::new(
			Arc::new(OkHandler),
			Arc::new(ObservingHandler {
				observed: Arc::clone(&observed),
			}),
		);

		// Act
		let response = handler
			.handle(build_request(Method::GET, "/"))
			.await
			.unwrap();

		// Assert
		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(String::from_utf8(response.body.to_vec()).unwrap(), "ok");
		assert!(observed.lock().unwrap().is_empty());
	}

	#[rstest]
	#[tokio::test]
	async fn test_inner_handler_runs_exactly_once() {
		// Arrange
		let calls = Arc::new(Mutex::new(0_usize));
		let observed = Arc::new(Mutex::new(Vec::new()));
		let handler = ExceptionHandlingHandler::new(
			Arc::new(CountingHandler {
				calls: Arc::clone(&calls),
			}),
			Arc::new(ObservingHandler {
				observed: Arc::clone(&observed),
			}),
		);

		// Act
		handler
			.handle(build_request(Method::GET, "/"))
			.await
			.unwrap();

		// Assert
		assert_eq!(*calls.lock().unwrap(), 1);
		assert_eq!(observed.lock().unwrap().len(), 1);
	}
}
