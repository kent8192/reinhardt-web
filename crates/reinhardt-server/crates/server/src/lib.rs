//! HTTP server implementation for Reinhardt framework.
//!
//! This module provides HTTP/1.1 and HTTP/2 server implementations based on Hyper.
//!
//! ## Basic Usage
//!
//! ```rust,ignore
//! use reinhardt_server::{HttpServer, serve};
//! use std::net::SocketAddr;
//! use std::sync::Arc;
//!
//! let addr: SocketAddr = "127.0.0.1:8000".parse()?;
//! let handler = Arc::new(MyHandler);
//!
//! let server = HttpServer::new(handler.clone());
//!
//! // Start server
//! serve(addr, handler).await?;
//! ```
//!
//! ## Accessing Handler
//!
//! Use the `handler()` method to get a clone of the server's handler:
//!
//! ```rust,ignore
//! let server = HttpServer::new(handler);
//! let handler_clone = server.handler();  // Returns Arc<dyn Handler>
//! ```

pub mod http;
pub mod http2;
pub mod rate_limit;
pub mod shutdown;
pub mod timeout;

#[cfg(feature = "graphql")]
pub mod graphql;

#[cfg(feature = "websocket")]
pub mod websocket;

pub use http::{HttpServer, serve, serve_with_shutdown};
pub use http2::{Http2Server, serve_http2, serve_http2_with_shutdown};
pub use rate_limit::{RateLimitConfig, RateLimitHandler, RateLimitStrategy};
pub use shutdown::{ShutdownCoordinator, shutdown_signal, with_shutdown};
pub use timeout::TimeoutHandler;

#[cfg(feature = "graphql")]
pub use graphql::{GraphQLHandler, graphql_handler};

#[cfg(feature = "websocket")]
pub use websocket::{BroadcastManager, WebSocketHandler, WebSocketServer, serve_websocket};

// Re-export types needed for server trait
pub use reinhardt_core::http::{Request, Response};

/// Common server trait that all server types implement
pub trait ServerHandler: Send + Sync {
	type Error;
	fn handle(
		&self,
		request: Request,
	) -> impl std::future::Future<Output = Result<Response, Self::Error>> + Send;
}
