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

/// Handler implementation for ServerRouter
#[async_trait]
impl Handler for ServerRouter {
	async fn handle(&self, mut req: Request) -> Result<Response> {
		let path = req.uri.path().to_owned();
		let method = req.method.clone();

		// Resolve route with HTTP method for matchit routing
		let route_match = match self.resolve(&path, &method) {
			Some(m) => m,
			None => {
				// Route not found for this method
				// Check if path exists for any other method to determine 404 vs 405
				let error_kind = if self.path_exists_for_any_method(&path) {
					RoutingErrorKind::MethodNotAllowed
				} else {
					RoutingErrorKind::NotFound
				};

				// The route match normally installs the route's DI context below.
				// Unmatched requests have no `RouteMatch`, so install the router
				// context before an exception handler or router middleware observes
				// the request.
				if let Some(di_ctx) = &self.di_context {
					req.set_di_context(di_ctx.clone());
				}

				// If router has middleware, route the error response through the
				// middleware chain so post-processing (e.g., security headers) is
				// applied to framework-level 404/405 responses. (#3234)
				let own_middleware = self.build_middleware_with_exclusions();
				if own_middleware.is_empty() {
					// An installed handler answers the request directly. Without
					// one the error stays an `Err`, which is what callers of a
					// middleware-free router rely on.
					let error = routing_error(error_kind, method.as_ref(), &path);
					return match self.exception_handler.as_ref() {
						Some(exception_handler) => {
							Ok(exception_handler.handle_exception(&req, error).await)
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
				let chain = match self.exception_handler.as_ref() {
					Some(exception_handler) => {
						chain.with_exception_handler(Arc::clone(exception_handler))
					}
					None => chain,
				};
				return chain.handle(req).await;
			}
		};

		req.path_params = route_match.params;

		// Set DI context if available
		if let Some(di_ctx) = &route_match.di_context {
			req.set_di_context(di_ctx.clone());
		}

		// Route the matched handler's errors through an installed handler before
		// the middleware chain wraps it. Without one the handler is used as-is, so
		// the default conversion is unchanged.
		let route_handler: Arc<dyn Handler> = match self.exception_handler.as_ref() {
			Some(exception_handler) => Arc::new(ExceptionHandlingHandler::new(
				route_match.handler.clone(),
				Arc::clone(exception_handler),
			)),
			None => route_match.handler.clone(),
		};

		// Apply middleware stack using MiddlewareChain
		if route_match.middleware_stack.is_empty() {
			// No middleware, execute handler directly
			route_handler.handle(req).await
		} else {
			// The chain also converts errors raised by middleware itself, so it
			// needs the handler independently of the adapter above.
			let chain =
				MiddlewareChain::with_middlewares(route_handler, route_match.middleware_stack);
			let chain = match self.exception_handler.as_ref() {
				Some(exception_handler) => {
					chain.with_exception_handler(Arc::clone(exception_handler))
				}
				None => chain,
			};

			// Execute chain
			chain.handle(req).await
		}
	}
}

/// Implement RegisterViewSet trait for ServerRouter
///
/// This allows ViewSetBuilder to directly register handlers to the router.
#[cfg(feature = "viewsets")]
impl reinhardt_views::viewsets::RegisterViewSet for ServerRouter {
	fn register_handler(&mut self, path: &str, handler: Arc<dyn Handler>) {
		self.views.push(ViewRoute {
			path: path.to_string(),
			handler,
			name: None,
			middleware: Vec::new(),
		});
	}
}
