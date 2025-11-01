//! # Reinhardt Dispatch
//!
//! HTTP request dispatching and handler system for Reinhardt framework.
//!
//! This module provides the core request handling functionality,
//! equivalent to Django's `django.core.handlers` and `django.dispatch`.
//!
//! ## Overview
//!
//! The dispatch system handles:
//! - HTTP request handling and routing
//! - Middleware chain execution
//! - View dispatching
//! - Exception handling
//! - Signal emission for request lifecycle events
//!
//! ## Architecture
//!
//! ```text
//! Request → BaseHandler → Middleware Chain → URL Resolver → View → Response
//!                ↓                                            ↓
//!           Signals                                      Signals
//!       (request_started)                          (request_finished)
//! ```
//!
//! ## Examples
//!
//! ### Basic Request Handling with URL Routing
//!
//! ```rust
//! use reinhardt_dispatch::BaseHandler;
//! use reinhardt_routers::{DefaultRouter, Router, path};
//! use reinhardt_http::{Request, Response};
//! use reinhardt_types::Handler;
//! use std::sync::Arc;
//! use hyper::{Method, Uri, Version, HeaderMap, StatusCode};
//! use bytes::Bytes;
//! use async_trait::async_trait;
//!
//! // Define a simple handler
//! struct HelloHandler;
//!
//! #[async_trait]
//! impl Handler for HelloHandler {
//!     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
//!         Ok(Response::ok().with_body("Hello, World!"))
//!     }
//! }
//!
//! # tokio_test::block_on(async {
//! // Create a router and register routes
//! let mut router = DefaultRouter::new();
//! let route = path("/", Arc::new(HelloHandler)).with_name("index");
//! router.add_route(route);
//!
//! // Create handler with router
//! let handler = BaseHandler::with_router(Arc::new(router));
//!
//! // Create a request
//! let request = Request::new(
//!     Method::GET,
//!     Uri::from_static("/"),
//!     Version::HTTP_11,
//!     HeaderMap::new(),
//!     Bytes::new(),
//! );
//!
//! // Handle request
//! let response = handler.handle_request(request).await.unwrap();
//! assert_eq!(response.status, StatusCode::OK);
//! # });
//! ```
//!
//! ### With Middleware
//!
//! ```rust
//! use reinhardt_dispatch::{BaseHandler, MiddlewareChain};
//! use reinhardt_routers::{DefaultRouter, Router, path};
//! use reinhardt_types::{Handler, Middleware};
//! use reinhardt_http::{Request, Response};
//! use std::sync::Arc;
//! use async_trait::async_trait;
//!
//! // Example middleware
//! struct LoggingMiddleware;
//!
//! #[async_trait]
//! impl Middleware for LoggingMiddleware {
//!     async fn process(&self, request: Request, next: Arc<dyn Handler>) -> reinhardt_exception::Result<Response> {
//!         println!("Request: {} {}", request.method, request.path());
//!         let response = next.handle(request).await?;
//!         println!("Response: {}", response.status);
//!         Ok(response)
//!     }
//! }
//!
//! // Example handler
//! struct ApiHandler;
//!
//! #[async_trait]
//! impl Handler for ApiHandler {
//!     async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
//!         Ok(Response::ok().with_json(&serde_json::json!({"status": "ok"})).unwrap())
//!     }
//! }
//!
//! # tokio_test::block_on(async {
//! // Setup router
//! let mut router = DefaultRouter::new();
//! let api_route = path("/api", Arc::new(ApiHandler)).with_name("api");
//! router.add_route(api_route);
//!
//! // Create base handler with router
//! let base_handler: Arc<dyn Handler> = Arc::new(BaseHandler::with_router(Arc::new(router)));
//!
//! // Wrap with middleware
//! let handler = MiddlewareChain::new(base_handler)
//!     .add_middleware(Arc::new(LoggingMiddleware))
//!     .build();
//!
//! // Use the handler
//! let request = Request::new(
//!     hyper::Method::GET,
//!     hyper::Uri::from_static("/api"),
//!     hyper::Version::HTTP_11,
//!     hyper::HeaderMap::new(),
//!     bytes::Bytes::new(),
//! );
//!
//! let response = handler.handle(request).await.unwrap();
//! assert_eq!(response.status, hyper::StatusCode::OK);
//! # });
//! ```

pub mod dispatcher;
pub mod exception;
pub mod handler;
pub mod middleware;

// Re-exports
pub use dispatcher::Dispatcher;
pub use exception::{ExceptionHandler, convert_exception_to_response};
pub use handler::BaseHandler;
pub use middleware::MiddlewareChain;

use thiserror::Error;

/// Errors that can occur during request dispatching
#[derive(Debug, Error)]
pub enum DispatchError {
	/// Middleware configuration error
	#[error("Middleware error: {0}")]
	Middleware(String),

	/// View execution error
	#[error("View error: {0}")]
	View(String),

	/// URL resolution error
	#[error("URL resolution error: {0}")]
	UrlResolution(String),

	/// HTTP error
	#[error("HTTP error: {0}")]
	Http(String),

	/// Internal error
	#[error("Internal error: {0}")]
	Internal(String),
}
