//! Integration tests for main paths of routing x middleware composition and
//! signal emission.
//!
//! These tests cover composition scenarios that the existing integration test
//! files do not exercise:
//!
//! * Routing 404 path observed by a middleware layer.
//! * Middleware short-circuiting before reaching the router.
//! * Multiple middleware modifying both request headers and the response
//!   status in a single composition.
//! * Middleware observing and transforming a view error.
//! * Signals emitted when the wrapped handler is composed with a middleware
//!   chain.
//! * Signals emitted when the view returns an internal error.

use async_trait::async_trait;
use bytes::Bytes;
use http::{HeaderName, HeaderValue, Method, StatusCode};
use reinhardt_core::signals::{request_finished, request_started};
use reinhardt_dispatch::handler::BaseHandler;
use reinhardt_dispatch::middleware::MiddlewareChain;
use reinhardt_http::Handler;
use reinhardt_http::{Request, Response};
use reinhardt_middleware::Middleware;
use reinhardt_urls::prelude::Router;
use reinhardt_urls::routers::{DefaultRouter, Route};
use rstest::rstest;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View handler that returns 200 OK with a fixed body.
struct OkHandler;

#[async_trait]
impl Handler for OkHandler {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		Ok(Response::new(StatusCode::OK).with_body(Bytes::from("OK")))
	}
}

/// View handler that returns an internal error.
struct InternalErrorHandler;

#[async_trait]
impl Handler for InternalErrorHandler {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		Err(reinhardt_core::exception::Error::Internal(
			"view failure".to_string(),
		))
	}
}

/// Middleware that records the downstream response status into a shared slot.
struct StatusRecorderMiddleware {
	observed: Arc<AtomicUsize>,
}

#[async_trait]
impl Middleware for StatusRecorderMiddleware {
	async fn process(
		&self,
		request: Request,
		handler: Arc<dyn Handler>,
	) -> reinhardt_core::exception::Result<Response> {
		let response = handler.handle(request).await?;
		self.observed
			.store(response.status.as_u16() as usize, Ordering::SeqCst);
		Ok(response)
	}
}

/// Middleware that short-circuits the chain by returning a fixed response
/// without invoking the wrapped handler.
struct ShortCircuitMiddleware {
	status: StatusCode,
	body: Bytes,
}

#[async_trait]
impl Middleware for ShortCircuitMiddleware {
	async fn process(
		&self,
		_request: Request,
		_handler: Arc<dyn Handler>,
	) -> reinhardt_core::exception::Result<Response> {
		// Deliberately do NOT call `handler.handle(...)` to verify
		// short-circuit semantics through the rest of the chain.
		Ok(Response::new(self.status).with_body(self.body.clone()))
	}
}

/// Middleware that inserts a request header before delegating downstream.
struct InsertHeaderMiddleware {
	name: &'static str,
	value: &'static str,
}

#[async_trait]
impl Middleware for InsertHeaderMiddleware {
	async fn process(
		&self,
		mut request: Request,
		handler: Arc<dyn Handler>,
	) -> reinhardt_core::exception::Result<Response> {
		request.headers.insert(
			HeaderName::from_static(self.name),
			HeaderValue::from_static(self.value),
		);
		handler.handle(request).await
	}
}

/// Middleware that overwrites the response status with `Accepted`.
struct ForceAcceptedMiddleware;

#[async_trait]
impl Middleware for ForceAcceptedMiddleware {
	async fn process(
		&self,
		request: Request,
		handler: Arc<dyn Handler>,
	) -> reinhardt_core::exception::Result<Response> {
		let mut response = handler.handle(request).await?;
		response.status = StatusCode::ACCEPTED;
		Ok(response)
	}
}

/// Middleware that transforms an `Internal` error into a 503 response.
struct ErrorToServiceUnavailableMiddleware;

#[async_trait]
impl Middleware for ErrorToServiceUnavailableMiddleware {
	async fn process(
		&self,
		request: Request,
		handler: Arc<dyn Handler>,
	) -> reinhardt_core::exception::Result<Response> {
		match handler.handle(request).await {
			Ok(response) => Ok(response),
			Err(_) => {
				Ok(Response::new(StatusCode::SERVICE_UNAVAILABLE).with_body(Bytes::from("DOWN")))
			}
		}
	}
}

/// View handler that echoes a specific header back in the body if present.
struct EchoHeaderHandler {
	header: &'static str,
}

#[async_trait]
impl Handler for EchoHeaderHandler {
	async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
		let body = request
			.headers
			.get(self.header)
			.and_then(|v| v.to_str().ok())
			.map(|s| Bytes::copy_from_slice(s.as_bytes()))
			.unwrap_or_else(|| Bytes::from_static(b"<missing>"));
		Ok(Response::new(StatusCode::OK).with_body(body))
	}
}

/// Helper: build a request targeting `path` with an empty body.
fn make_request(path: &'static str) -> Request {
	Request::builder()
		.method(Method::GET)
		.uri(path)
		.body(Bytes::new())
		.build()
		.expect("request builder must succeed for static URIs")
}

#[rstest]
#[tokio::test]
async fn middleware_observes_router_404_status() {
	// Arrange
	let router = DefaultRouter::new();
	// Wrap the router in BaseHandler so unknown routes resolve to a 404
	// Response (raw routers return Err for missing routes).
	let base_handler: Arc<dyn Handler> = Arc::new(BaseHandler::with_router(Arc::new(router)));
	let observed = Arc::new(AtomicUsize::new(0));
	let handler = MiddlewareChain::new(base_handler)
		.add_middleware(Arc::new(StatusRecorderMiddleware {
			observed: observed.clone(),
		}))
		.expect("middleware must be accepted within default depth")
		.build();

	// Act
	let response = handler
		.handle(make_request("/missing"))
		.await
		.expect("router returns Ok(404) rather than Err for unknown routes");

	// Assert
	assert_eq!(response.status, StatusCode::NOT_FOUND);
	assert_eq!(observed.load(Ordering::SeqCst), 404);
}

#[rstest]
#[tokio::test]
async fn middleware_short_circuit_prevents_downstream_handler() {
	// Arrange
	let mut router = DefaultRouter::new();
	let downstream_invoked = Arc::new(AtomicUsize::new(0));
	struct CountingOk {
		count: Arc<AtomicUsize>,
	}
	#[async_trait]
	impl Handler for CountingOk {
		async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
			self.count.fetch_add(1, Ordering::SeqCst);
			Ok(Response::new(StatusCode::OK).with_body(Bytes::from("UNREACHABLE")))
		}
	}
	router.add_route(Route::from_handler(
		"/svc",
		Arc::new(CountingOk {
			count: downstream_invoked.clone(),
		}),
	));
	let handler = MiddlewareChain::new(Arc::new(router))
		.add_middleware(Arc::new(ShortCircuitMiddleware {
			status: StatusCode::IM_A_TEAPOT,
			body: Bytes::from_static(b"SHORT"),
		}))
		.expect("middleware must be accepted")
		.build();

	// Act
	let response = handler
		.handle(make_request("/svc"))
		.await
		.expect("short-circuit returns Ok response");

	// Assert
	assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
	assert_eq!(&response.body, &Bytes::from_static(b"SHORT"));
	assert_eq!(downstream_invoked.load(Ordering::SeqCst), 0);
}

#[rstest]
#[tokio::test]
async fn middleware_chain_composes_request_and_response_modifications() {
	// Arrange
	let mut router = DefaultRouter::new();
	router.add_route(Route::from_handler(
		"/echo",
		Arc::new(EchoHeaderHandler {
			header: "x-injected",
		}),
	));
	let handler = MiddlewareChain::new(Arc::new(router))
		.add_middleware(Arc::new(ForceAcceptedMiddleware))
		.expect("response-modifier middleware must be accepted")
		.add_middleware(Arc::new(InsertHeaderMiddleware {
			name: "x-injected",
			value: "from-middleware",
		}))
		.expect("request-modifier middleware must be accepted")
		.build();

	// Act
	let response = handler
		.handle(make_request("/echo"))
		.await
		.expect("composed chain succeeds");

	// Assert: header injection reached the view AND status override applied.
	assert_eq!(response.status, StatusCode::ACCEPTED);
	assert_eq!(&response.body, &Bytes::from_static(b"from-middleware"));
}

#[rstest]
#[tokio::test]
async fn middleware_transforms_view_error_into_response() {
	// Arrange
	let mut router = DefaultRouter::new();
	router.add_route(Route::from_handler("/fail", Arc::new(InternalErrorHandler)));
	let handler = MiddlewareChain::new(Arc::new(router))
		.add_middleware(Arc::new(ErrorToServiceUnavailableMiddleware))
		.expect("error-translating middleware must be accepted")
		.build();

	// Act
	let response = handler
		.handle(make_request("/fail"))
		.await
		.expect("error-translating middleware converts Err into Ok response");

	// Assert
	assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
	assert_eq!(&response.body, &Bytes::from_static(b"DOWN"));
}

#[rstest]
#[tokio::test]
async fn signals_emit_when_handler_returns_internal_error() {
	// Arrange
	let started = Arc::new(AtomicUsize::new(0));
	let finished = Arc::new(AtomicUsize::new(0));
	let started_clone = started.clone();
	let finished_clone = finished.clone();
	request_started().connect(move |_event| {
		let counter = started_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});
	request_finished().connect(move |_event| {
		let counter = finished_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	let mut router = DefaultRouter::new();
	router.add_route(Route::from_handler("/boom", Arc::new(InternalErrorHandler)));
	let handler = BaseHandler::with_router(Arc::new(router));

	let started_before = started.load(Ordering::SeqCst);
	let finished_before = finished.load(Ordering::SeqCst);

	// Act
	let response = handler
		.handle(make_request("/boom"))
		.await
		.expect("BaseHandler converts view errors into a 500 response");

	// Assert
	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
	assert_eq!(
		started.load(Ordering::SeqCst) - started_before,
		1,
		"request_started must still fire on view error",
	);
	assert_eq!(
		finished.load(Ordering::SeqCst) - finished_before,
		1,
		"request_finished must still fire on view error",
	);
}

#[rstest]
#[tokio::test]
async fn signals_emit_when_handler_is_wrapped_by_middleware_chain() {
	// Arrange
	let started = Arc::new(AtomicUsize::new(0));
	let finished = Arc::new(AtomicUsize::new(0));
	let started_clone = started.clone();
	let finished_clone = finished.clone();
	request_started().connect(move |_event| {
		let counter = started_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});
	request_finished().connect(move |_event| {
		let counter = finished_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	let mut router = DefaultRouter::new();
	router.add_route(Route::from_handler("/ok", Arc::new(OkHandler)));
	let base_handler = Arc::new(BaseHandler::with_router(Arc::new(router)));
	let observed = Arc::new(AtomicUsize::new(0));
	let handler = MiddlewareChain::new(base_handler)
		.add_middleware(Arc::new(StatusRecorderMiddleware {
			observed: observed.clone(),
		}))
		.expect("middleware must be accepted")
		.build();

	let started_before = started.load(Ordering::SeqCst);
	let finished_before = finished.load(Ordering::SeqCst);

	// Act
	let response = handler
		.handle(make_request("/ok"))
		.await
		.expect("composed chain returns Ok");

	// Assert
	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(&response.body, &Bytes::from_static(b"OK"));
	assert_eq!(observed.load(Ordering::SeqCst), 200);
	assert_eq!(
		started.load(Ordering::SeqCst) - started_before,
		1,
		"BaseHandler wrapped by middleware chain must still emit request_started",
	);
	assert_eq!(
		finished.load(Ordering::SeqCst) - finished_before,
		1,
		"BaseHandler wrapped by middleware chain must still emit request_finished",
	);
}
