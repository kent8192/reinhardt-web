//! Async closure-based, method-aware routes for test stub servers.
//!
//! Capture shared state in closures to implement fake upstreams and webhooks.
//! Request recording and call-count assertions remain under the test's control.
//! This module is native-only (P0) through `reinhardt::test::stub`.

use reinhardt_http::{Handler, Request, Response, ViewResult};
use reinhardt_urls::routers::ServerRouter;
use std::collections::HashSet;
use std::future::Future;

use hyper::Method;

/// Builds a test stub server router from capturing async closures.
///
/// Each route receives a raw [`Request`], including `request.path_params`.
/// Method mismatches and unknown paths use the framework's 405 and 404 handling,
/// including router middleware and exception handling. For application endpoints
/// with extractors and named routes, use HTTP method macros and
/// [`ServerRouter::endpoint`] instead.
///
/// # Examples
///
/// This example builds a router without starting a server. Pass the result to
/// [`crate::test_server_guard`] to serve it in a test.
///
/// ```rust
/// use reinhardt_http::Response;
/// use reinhardt_testkit::{ServerRouter, stub::StubRouter};
/// use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
///
/// let calls = Arc::new(AtomicUsize::new(0));
/// let captured_calls = Arc::clone(&calls);
/// let router: ServerRouter = StubRouter::new()
///     .post("/webhook", move |_request| {
///         let calls = Arc::clone(&captured_calls);
///         async move {
///             calls.fetch_add(1, Ordering::SeqCst);
///             Ok(Response::ok().with_body("accepted"))
///         }
///     })
///     .get("/health", |_request| async { Ok(Response::ok()) })
///     .into();
/// assert_eq!(calls.load(Ordering::SeqCst), 0);
/// ```
pub struct StubRouter {
	router: ServerRouter,
	routes: HashSet<(String, Method)>,
}

impl StubRouter {
	/// Creates an empty stub router.
	pub fn new() -> Self {
		Self {
			router: ServerRouter::new(),
			routes: HashSet::new(),
		}
	}

	/// Registers an async closure for a path and HTTP method.
	///
	/// # Panics
	///
	/// Panics immediately if this exact `(path, method)` is already registered.
	pub fn route<F, Fut>(mut self, path: &str, method: Method, handler: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = ViewResult<Response>> + Send + 'static,
	{
		assert!(
			self.routes.insert((path.to_owned(), method.clone())),
			"duplicate stub route: {method} {path}"
		);
		self.router = self
			.router
			.handler_for_method(path, method, ClosureHandler(handler));
		self
	}

	/// Registers a GET route. Panics on a duplicate path and method.
	pub fn get<F, Fut>(self, path: &str, handler: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = ViewResult<Response>> + Send + 'static,
	{
		self.route(path, Method::GET, handler)
	}

	/// Registers a POST route. Panics on a duplicate path and method.
	pub fn post<F, Fut>(self, path: &str, handler: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = ViewResult<Response>> + Send + 'static,
	{
		self.route(path, Method::POST, handler)
	}

	/// Registers a PUT route. Panics on a duplicate path and method.
	pub fn put<F, Fut>(self, path: &str, handler: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = ViewResult<Response>> + Send + 'static,
	{
		self.route(path, Method::PUT, handler)
	}

	/// Registers a PATCH route. Panics on a duplicate path and method.
	pub fn patch<F, Fut>(self, path: &str, handler: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = ViewResult<Response>> + Send + 'static,
	{
		self.route(path, Method::PATCH, handler)
	}

	/// Registers a DELETE route. Panics on a duplicate path and method.
	pub fn delete<F, Fut>(self, path: &str, handler: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = ViewResult<Response>> + Send + 'static,
	{
		self.route(path, Method::DELETE, handler)
	}

	/// Converts the stub into a router usable with [`crate::test_server_guard`].
	pub fn into_server_router(self) -> ServerRouter {
		self.router
	}
}

impl Default for StubRouter {
	fn default() -> Self {
		Self::new()
	}
}

impl From<StubRouter> for ServerRouter {
	fn from(stub: StubRouter) -> Self {
		stub.into_server_router()
	}
}

struct ClosureHandler<F>(F);

#[async_trait::async_trait]
impl<F, Fut> Handler for ClosureHandler<F>
where
	F: Fn(Request) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = ViewResult<Response>> + Send + 'static,
{
	async fn handle(&self, request: Request) -> ViewResult<Response> {
		(self.0)(request).await
	}
}
