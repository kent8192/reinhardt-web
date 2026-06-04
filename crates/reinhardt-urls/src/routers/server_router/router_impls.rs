//! Trait impls for [`ServerRouter`] that don't fit the builder / dispatch split.
//!
//! Includes `Debug`, `Default`, the `Handler` impl that turns the router
//! into an HTTP entry point, and the `RegisterViewSet` adapter used by
//! `ViewSetBuilder`.

use super::ServerRouter;
#[cfg(feature = "viewsets")]
use super::types::ViewRoute;
use async_trait::async_trait;
use reinhardt_http::{Error, Handler, MiddlewareChain, Request, Response, Result};
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

/// Handler that always returns a pre-built response.
///
/// Used internally to route framework-level error responses (404/405)
/// through the middleware chain for post-processing. (#3234)
struct FixedResponseHandler(Response);

#[async_trait]
impl Handler for FixedResponseHandler {
	async fn handle(&self, _request: Request) -> Result<Response> {
		Ok(self.0.clone())
	}
}

/// Handler implementation for ServerRouter
#[async_trait]
impl Handler for ServerRouter {
	async fn handle(&self, mut req: Request) -> Result<Response> {
		let path = req.uri.path();
		let method = &req.method;

		// Resolve route with HTTP method for matchit routing
		let route_match = match self.resolve(path, method) {
			Some(m) => m,
			None => {
				// Route not found for this method
				// Check if path exists for any other method to determine 404 vs 405
				let error = if self.path_exists_for_any_method(path) {
					Error::MethodNotAllowed(format!("Method {} not allowed for {}", method, path))
				} else {
					Error::NotFound(format!("No route for {} {}", method, path))
				};

				// If router has middleware, route the error response through the
				// middleware chain so post-processing (e.g., security headers) is
				// applied to framework-level 404/405 responses. (#3234)
				let own_middleware = self.build_middleware_with_exclusions();
				if own_middleware.is_empty() {
					return Err(error);
				}

				let response = Response::from(error);
				let handler: Arc<dyn Handler> = Arc::new(FixedResponseHandler(response));
				let chain = own_middleware
					.iter()
					.fold(MiddlewareChain::new(handler), |chain, mw| {
						chain.with_middleware(mw.clone())
					});
				return chain.handle(req).await;
			}
		};

		// Set path parameters in request
		for (key, value) in route_match.params {
			req.set_path_param(key, value);
		}

		// Set DI context if available
		if let Some(di_ctx) = &route_match.di_context {
			req.set_di_context(di_ctx.clone());
		}

		// Apply middleware stack using MiddlewareChain
		if route_match.middleware_stack.is_empty() {
			// No middleware, execute handler directly
			route_match.handler.handle(req).await
		} else {
			// Build middleware chain
			let chain = route_match.middleware_stack.iter().fold(
				MiddlewareChain::new(route_match.handler.clone()),
				|chain, mw| chain.with_middleware(mw.clone()),
			);

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
