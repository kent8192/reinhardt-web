pub mod http;
pub mod http2;
pub mod rate_limit;
pub mod shutdown;
pub mod timeout;

#[cfg(feature = "graphql")]
pub mod graphql;

#[cfg(feature = "websocket")]
pub mod websocket;

pub use http::{serve, serve_with_shutdown, HttpServer};
pub use http2::{serve_http2, serve_http2_with_shutdown, Http2Server};
pub use rate_limit::{RateLimitConfig, RateLimitHandler, RateLimitStrategy};
pub use shutdown::{shutdown_signal, with_shutdown, ShutdownCoordinator};
pub use timeout::TimeoutHandler;

#[cfg(feature = "graphql")]
pub use graphql::{graphql_handler, GraphQLHandler};

#[cfg(feature = "websocket")]
pub use websocket::{serve_websocket, BroadcastManager, WebSocketHandler, WebSocketServer};

// Re-export types needed for server trait
pub use reinhardt_http::{Request, Response};

/// Common server trait that all server types implement
pub trait ServerHandler: Send + Sync {
    type Error;
    fn handle(
        &self,
        request: Request,
    ) -> impl std::future::Future<Output = Result<Response, Self::Error>> + Send;
}
