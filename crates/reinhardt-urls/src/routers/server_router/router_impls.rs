//! Trait impls for [`ServerRouter`] that don't fit the builder / dispatch split.
//!
//! Includes `Debug`, `Default`, the `Handler` impl that turns the router
//! into an HTTP entry point, and the `RegisterViewSet` adapter used by
//! `ViewSetBuilder`.

use super::ServerRouter;
#[cfg(feature = "viewsets")]
use super::types::ViewRoute;
use async_trait::async_trait;
use reinhardt_http::{
	Error, ExceptionHandlingHandler, Handler, MiddlewareChain, Request, Response, Result,
};
use std::sync::Arc;

impl std::fmt::Debug for ServerRouter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut debug = f.debug_struct("ServerRouter");
		debug
			.field("prefix", &self.prefix)
			.field("namespace", &self.namespace)
			.field("routes", &self.routes.len());
		#[cfg(feature = "viewsets")]
		debug.field("viewsets", &self.viewsets.len());
		debug
			.field("functions", &self.functions.len())
			.field("views", &self.views.len())
			.field("children", &self.children.len())
			.field("middleware", &self.middleware.len())
			.finish_non_exhaustive()
	}
}

impl Default for ServerRouter {
	fn default() -> Self {
		Self::new()
	}
}

/// The kind of routing failure produced when no route matches a request.
#[derive(Clone, Copy)]
enum RoutingErrorKind {
	NotFound,
	MethodNotAllowed,
}

fn routing_error(kind: RoutingErrorKind, method: &str, path: &str) -> Error {
	match kind {
		RoutingErrorKind::MethodNotAllowed => {
			Error::MethodNotAllowed(format!("Method {method} not allowed for {path}"))
		}
		RoutingErrorKind::NotFound => Error::NotFound(format!("No route for {method} {path}")),
	}
}

/// Handler that returns the routing error for the current request.
///
/// Keeping the error as an `Err` until the middleware chain reaches this
/// handler lets middleware short-circuit an unmatched request without first
/// invoking the exception handler for a response that will be discarded.
struct RoutingErrorHandler {
	kind: RoutingErrorKind,
	method: String,
	path: String,
}

#[async_trait]
impl Handler for RoutingErrorHandler {
	async fn handle(&self, _request: Request) -> Result<Response> {
		Err(routing_error(self.kind, &self.method, &self.path))
	}
}

macro_rules! dispatch_router_request {
	($router:expr, $req:expr) => {{
		let mut req = $req;
		let exception_handler = $router.exception_handler.clone().or_else(|| {
			req.extensions
				.get::<Arc<dyn reinhardt_http::ExceptionHandler>>()
		});
		if let Some(handler) = &exception_handler {
			req.install_exception_handler(Arc::clone(handler));
		}
		let path = req.uri.path();
		let method = &req.method;

		// Resolve route with HTTP method for matchit routing
		let route_match = match $router.resolve(path, method) {
			Some(m) => m,
			None => {
				// Route not found for this method
				// Check if path exists for any other method to determine 404 vs 405
				let error_kind = if $router.path_exists_for_any_method(path) {
					RoutingErrorKind::MethodNotAllowed
				} else {
					RoutingErrorKind::NotFound
				};

				let path = path.to_owned();
				let method = method.clone();

				// The route match normally installs the route's DI context below.
				// Unmatched requests have no `RouteMatch`, so install the router
				// context before an exception handler or router middleware observes
				// the request.
				if let Some(di_ctx) = &$router.di_context {
					req.set_di_context(di_ctx.clone());
				}

				// If router has middleware, route the error response through the
				// middleware chain so post-processing (e.g., security headers) is
				// applied to framework-level 404/405 responses. (#3234)
				let own_middleware = $router.build_middleware_with_exclusions();
				if own_middleware.is_empty() {
					// An installed handler answers the request directly. Without
					// one the error stays an `Err`, which is what callers of a
					// middleware-free router rely on.
					let error = routing_error(error_kind, method.as_ref(), &path);
					return match exception_handler.as_ref() {
						Some(exception_handler) => {
							let context = req.clone_for_di();
							context
								.extensions
								.insert(reinhardt_http::ExceptionHandlerInvoked);
							Ok(exception_handler.handle_exception(&context, error).await)
						}
						None => Err(error),
					};
				}

				// Keep the routing error as an `Err` so the chain's exception
				// handler runs only when middleware calls the inner handler. This
				// preserves #3234 post-processing while avoiding duplicate custom
				// responses when middleware rejects the request first.
				let handler: Arc<dyn Handler> = Arc::new(RoutingErrorHandler {
					kind: error_kind,
					method: method.to_string(),
					path,
				});
				let chain = own_middleware
					.iter()
					.fold(MiddlewareChain::new(handler), |chain, mw| {
						chain.with_middleware(mw.clone())
					});
				// A middleware in this chain can fail too (a CSRF or permission
				// rejection on an unmatched path), and that error must use the same
				// handler as the 404/405 body above.
				let chain = match exception_handler.as_ref() {
					Some(exception_handler) => {
						chain.with_exception_handler(Arc::clone(exception_handler))
					}
					None => chain,
				};
				return chain.handle(req).await;
			}
		};

		if let Some(params) = route_match.params {
			req.set_path_params(params);
		} else if exception_handler.is_some() || !req.path_params.is_empty() {
			req.set_path_params(Default::default());
		}

		// Set DI context if available
		if let Some(di_ctx) = &route_match.di_context {
			req.set_di_context(di_ctx.clone());
		}

		// Apply middleware stack using MiddlewareChain
		if route_match.middleware_stack.is_empty() {
			if let Some(exception_handler) = exception_handler.as_ref() {
				return ExceptionHandlingHandler::new(
					Arc::clone(route_match.handler),
					Arc::clone(exception_handler),
				)
				.handle(req)
				.await;
			}
			if let Some(requestless_handler) = route_match.requestless_sync_handler {
				return requestless_handler.handle_requestless_sync();
			}
			if let Some(sync_handler) = route_match.sync_handler {
				return sync_handler.handle_sync(req);
			}

			// No middleware, execute the trait object directly. Calling through
			// `Arc<dyn Handler>` would add the blanket `Arc<T>` async-trait box.
			route_match.handler.as_ref().handle(req).await
		} else {
			let route_handler: Arc<dyn Handler> = match exception_handler.as_ref() {
				Some(exception_handler) => Arc::new(ExceptionHandlingHandler::new(
					Arc::clone(route_match.handler),
					Arc::clone(exception_handler),
				)),
				None => Arc::clone(route_match.handler),
			};
			// The chain also converts errors raised by middleware itself, so it
			// needs the handler independently of the adapter above.
			let chain =
				MiddlewareChain::with_middlewares(route_handler, route_match.middleware_stack);
			let chain = match exception_handler.as_ref() {
				Some(exception_handler) => {
					chain.with_exception_handler(Arc::clone(exception_handler))
				}
				None => chain,
			};

			// Execute chain
			chain.handle(req).await
		}
	}};
}

impl ServerRouter {
	fn try_dispatch_exact_requestless_sync(
		&self,
		path: &str,
		method: &hyper::Method,
	) -> Option<Result<Response>> {
		if !self.children.is_empty() || !self.middleware.is_empty() || self.di_context.is_some() {
			return None;
		}

		let remaining_path = Self::strip_prefix_normalized(&self.prefix, path)?;
		let route_handler = self
			.compiled_routes()
			.exact_for_method(method)?
			.get(remaining_path.as_ref())?;

		if !route_handler.middleware.is_empty() || !route_handler.param_names.is_empty() {
			return None;
		}

		route_handler
			.requestless_sync_handler
			.as_ref()
			.map(|handler| handler.handle_requestless_sync())
	}

	/// Try to dispatch a request through the synchronous route fast path.
	///
	/// Returns `None` when the matched route requires async handling or a
	/// middleware chain, or when an exception handler requires async conversion.
	/// Callers that need general routing should fall back to
	/// [`Self::dispatch`] in that case.
	pub fn try_dispatch_sync(&self, mut req: Request) -> Option<Result<Response>> {
		if self.exception_handler.is_some()
			|| req
				.extensions
				.contains::<Arc<dyn reinhardt_http::ExceptionHandler>>()
		{
			return None;
		}
		if self.children.is_empty()
			&& self.middleware.is_empty()
			&& let Some(remaining_path) =
				Self::strip_prefix_normalized(&self.prefix, req.uri.path())
			&& let Some(exact_routes) = self.compiled_routes().exact_for_method(&req.method)
			&& let Some(route_handler) = exact_routes.get(remaining_path.as_ref())
			&& route_handler.middleware.is_empty()
			&& route_handler.param_names.is_empty()
		{
			if let Some(requestless_handler) = route_handler.requestless_sync_handler.as_ref()
				&& self.di_context.is_none()
			{
				return Some(requestless_handler.handle_requestless_sync());
			}

			if let Some(sync_handler) = route_handler.sync_handler.as_ref() {
				if !req.path_params.is_empty() {
					req.path_params = Default::default();
				}
				if let Some(di_ctx) = &self.di_context {
					req.set_di_context(di_ctx.clone());
				}
				return Some(sync_handler.handle_sync(req));
			}
		}

		let path = req.uri.path();
		let method = &req.method;

		let route_match = match self.resolve(path, method) {
			Some(m) => m,
			None => {
				let error = if self.path_exists_for_any_method(path) {
					Error::MethodNotAllowed(format!("Method {} not allowed for {}", method, path))
				} else {
					Error::NotFound(format!("No route for {} {}", method, path))
				};

				if self.build_middleware_with_exclusions().is_empty() {
					return Some(Err(error));
				}
				return None;
			}
		};

		if !route_match.middleware_stack.is_empty() {
			return None;
		}

		if let Some(requestless_handler) = route_match.requestless_sync_handler {
			return Some(requestless_handler.handle_requestless_sync());
		}

		let sync_handler = route_match.sync_handler?;
		if let Some(params) = route_match.params {
			req.path_params = params;
		} else if !req.path_params.is_empty() {
			req.path_params = Default::default();
		}
		if let Some(di_ctx) = &route_match.di_context {
			req.set_di_context(di_ctx.clone());
		}

		Some(sync_handler.handle_sync(req))
	}

	/// Try to dispatch a requestless synchronous route before building a request.
	///
	/// This only succeeds for routes that need no request state: no middleware,
	/// no path parameters, no DI context, and no installed exception handler.
	/// HTTP adapters can use this after
	/// validating that the incoming request has no body.
	pub fn try_dispatch_requestless_sync(
		&self,
		path: &str,
		method: &hyper::Method,
	) -> Option<Result<Response>> {
		if self.exception_handler.is_some() {
			return None;
		}
		if let Some(response) = self.try_dispatch_exact_requestless_sync(path, method) {
			return Some(response);
		}

		let route_match = self.resolve(path, method)?;
		if !route_match.middleware_stack.is_empty()
			|| route_match.params.is_some()
			|| route_match.di_context.is_some()
		{
			return None;
		}

		let requestless_handler = route_match.requestless_sync_handler?;
		Some(requestless_handler.handle_requestless_sync())
	}

	/// Dispatch a request through this router without a trait-object handler wrapper.
	///
	/// This has the same routing behavior as the [`Handler`] implementation, but
	/// concrete callers can await the router's inherent future directly instead of
	/// going through the boxed future produced by `async_trait`.
	pub async fn dispatch(&self, req: Request) -> Result<Response> {
		dispatch_router_request!(self, req)
	}
}

/// Handler implementation for ServerRouter
#[async_trait]
impl Handler for ServerRouter {
	async fn handle(&self, req: Request) -> Result<Response> {
		dispatch_router_request!(self, req)
	}
}

/// Implement RegisterViewSet trait for ServerRouter
///
/// This allows ViewSetBuilder to directly register handlers to the router.
#[cfg(feature = "viewsets")]
impl reinhardt_views::viewsets::RegisterViewSet for ServerRouter {
	fn register_handler(&mut self, path: &str, handler: Arc<dyn Handler>) {
		self.invalidate_compiled_routes();
		self.views.push(ViewRoute {
			path: path.to_string(),
			handler,
			sync_handler: None,
			requestless_sync_handler: None,
			name: None,
			metadata: None,
			middleware: Vec::new(),
		});
	}
}
