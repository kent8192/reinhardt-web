//! Middleware system for request/response processing pipeline.

use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};
use reinhardt_types::{Handler, Middleware};
use std::sync::Arc;

/// A middleware chain that composes multiple middleware components with a handler.
pub struct MiddlewareChain {
    handler: Arc<dyn Handler>,
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Creates a new middleware chain with the given handler.
    pub fn new(handler: Arc<dyn Handler>) -> Self {
        Self {
            handler,
            middlewares: Vec::new(),
        }
    }

    /// Adds a middleware to the chain.
    pub fn add_middleware(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// Builds the final handler by composing all middleware.
    pub fn build(self) -> Arc<dyn Handler> {
        let mut handler = self.handler;

        for middleware in self.middlewares.into_iter().rev() {
            handler = Arc::new(MiddlewareHandler {
                middleware,
                next: handler,
            });
        }

        handler
    }
}

/// Internal handler that wraps a middleware with its next handler.
struct MiddlewareHandler {
    middleware: Arc<dyn Middleware>,
    next: Arc<dyn Handler>,
}

#[async_trait::async_trait]
impl Handler for MiddlewareHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        self.middleware.process(request, self.next.clone()).await
    }
}
