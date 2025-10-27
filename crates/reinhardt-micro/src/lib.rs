//! # Reinhardt Micro
//!
//! A lightweight microservice framework for Rust, providing the minimal subset of Reinhardt
//! functionality needed for building simple APIs and microservices.
//!
//! ## Philosophy
//!
//! Reinhardt Micro is designed to solve the "over-engineering" problem commonly found in
//! monolithic frameworks like Django. It provides:
//!
//! - **Minimal dependencies**: Only include what you need
//! - **Fast compilation**: Fewer crates means faster builds
//! - **Small binaries**: Optimized for microservices and serverless
//! - **FastAPI-inspired ergonomics**: Function-based endpoints with type-safe parameter extraction
//!
//! ## Middleware Configuration Helpers
//!
//! Reinhardt Micro provides builder-style middleware configuration:
//!
//! ```rust,no_run
//! use reinhardt_micro::{App, CorsConfig, RateLimitConfig, CompressionConfig, LoggingConfig, MetricsConfig};
//! use std::time::Duration;
//!
//! # async fn example() {
//! let app = App::new()
//!     .with_cors(CorsConfig::permissive())
//!     .with_rate_limit(RateLimitConfig::lenient())
//!     .with_compression(CompressionConfig::for_json())
//!     .with_timeout(Duration::from_secs(30))
//!     .with_logging(LoggingConfig::verbose())
//!     .with_metrics(MetricsConfig::with_endpoint("/metrics"));
//! # }
//! ```
//!
//! Available middleware helper methods:
//! - `with_cors()`: Quick CORS configuration
//! - `with_rate_limit()`: Simple rate limiting setup
//! - `with_compression()`: Response compression
//! - `with_timeout()`: Request timeout handling
//! - `with_logging()`: Structured logging configuration
//! - `with_metrics()`: Metrics collection
//!
//! ## Planned Features
//!
//! ### Additional Utility Functions
//!
//! Planned utility additions:
//!
//! ```rust,ignore
//! use reinhardt_micro::utils::*;
//!
//! // Response builders
//! ok_json(data);              // 200 OK with JSON
//! created_json(data);         // 201 Created with JSON
//! no_content();               // 204 No Content
//! bad_request(message);       // 400 Bad Request
//! unauthorized(message);      // 401 Unauthorized
//! not_found(message);         // 404 Not Found
//! internal_error(message);    // 500 Internal Server Error
//!
//! // Request helpers
//! extract_bearer_token(&request);  // Extract JWT from Authorization header
//! parse_query_params::<T>(&request); // Parse query string into type T
//! validate_content_type(&request, "application/json");
//!
//! // Testing utilities
//! test_request(method, path, body);  // Create test request
//! assert_json_response(response, expected); // Assert JSON response
//! ```
//!
//! For detailed implementation status, see GitHub issues with the `enhancement` label.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use reinhardt_micro::App;
//!
//! #[tokio::main]
//! async fn main() {
//!     fn hello() {
//!         println!("Hello, World!");
//!     }
//!
//!     fn get_user() {
//!         println!("Get user endpoint");
//!     }
//!
//!     // Create a minimal app
//!     let app = App::new()
//!         .route("/", hello)
//!         .route("/users/:id", get_user);
//!
//!     // Run the server
//!     app.serve("127.0.0.1:8000").await.unwrap();
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `routing` (default): Basic routing functionality
//! - `params` (default): Type-safe parameter extraction (Path, Query, Json, etc.)
//! - `di` (default): Dependency injection system
//! - `schema` (default): OpenAPI schema generation
//! - `database`: ORM integration (optional)
//!
//! ## When to use Reinhardt Micro vs Full Reinhardt
//!
//! Use **Reinhardt Micro** when:
//! - Building simple REST APIs or microservices
//! - You need fast compilation and small binaries
//! - You don't need admin panel, ORM, or complex authentication
//! - You prefer function-based endpoints over class-based views
//!
//! Use **Full Reinhardt** when:
//! - Building complex applications with many features
//! - You need Django-style admin panel, ORM, migrations, etc.
//! - You're migrating from Django and want familiar patterns
//! - You need all the batteries included

pub use reinhardt_apps::{Error, Request, Response, Result};

#[cfg(feature = "params")]
pub use reinhardt_params::{Cookie, Form, Header, Json, Path, Query};

/// Utility functions for building HTTP responses
pub mod utils;

#[cfg(feature = "di")]
pub use reinhardt_di::Depends;

#[cfg(feature = "database")]
pub use reinhardt_orm as orm;

// Re-export endpoint macros for FastAPI-style function-based endpoints
pub use reinhardt_macros::{delete, endpoint, get, patch, post, put, use_injection};

/// Built-in middleware shortcuts for common use cases
pub mod middleware {
    pub use reinhardt_middleware::{
        // Compression
        BrotliMiddleware,
        // CORS
        CorsMiddleware,
        // Security
        CsrfMiddleware,
        GZipMiddleware,
        // HTTPS
        HttpsRedirectMiddleware,
        // Logging
        LoggingMiddleware,
        // Middleware trait
        Middleware,
        // Request tracking
        RequestIdMiddleware,
        SecurityMiddleware,
        TracingMiddleware,
    };
}

/// Middleware configuration helpers
pub mod middleware_config;

/// Utility functions for common request operations
pub mod utils;

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{Error, Request, Response, Result};

    #[cfg(feature = "params")]
    pub use reinhardt_params::{Cookie, Form, Header, Json, Path, Query};

    #[cfg(feature = "di")]
    pub use reinhardt_di::Depends;

    // Re-export endpoint macros
    pub use reinhardt_macros::{delete, endpoint, get, patch, post, put, use_injection};

    // Re-export utils
    pub use super::utils::*;

    // Re-export middleware configs
    pub use super::middleware_config::*;

    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};
}

use reinhardt_middleware::Middleware;
use reinhardt_routers::{path as route_path, DefaultRouter, Router};
use reinhardt_server::serve as http_serve;
use reinhardt_types::Handler;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Re-export configuration types for convenience
pub use middleware_config::{
    CompressionConfig, CorsConfig, LoggingConfig, MetricsConfig, RateLimitConfig, TimeoutConfig,
};

/// Application builder for creating micro services
pub struct App {
    router: Arc<Mutex<DefaultRouter>>,
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl App {
    /// Create a new application
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::App;
    ///
    /// let app = App::new();
    // App is now ready to have routes added
    /// ```
    pub fn new() -> Self {
        Self {
            router: Arc::new(Mutex::new(DefaultRouter::new())),
            middlewares: Vec::new(),
        }
    }

    /// Add CORS middleware with custom configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::{App, CorsConfig};
    ///
    /// let app = App::new()
    ///     .with_cors(CorsConfig::permissive());
    /// ```
    pub fn with_cors(mut self, config: CorsConfig) -> Self {
        use reinhardt_middleware::CorsMiddleware;
        self.middlewares.push(Arc::new(CorsMiddleware::new(config)));
        self
    }

    /// Add rate limiting middleware with custom configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::{App, RateLimitConfig};
    ///
    /// let app = App::new()
    ///     .with_rate_limit(RateLimitConfig::lenient());
    /// ```
    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        use reinhardt_middleware::RateLimitMiddleware;
        self.middlewares.push(Arc::new(RateLimitMiddleware::new(config)));
        self
    }

    /// Add compression middleware with custom configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::{App, CompressionConfig};
    ///
    /// let app = App::new()
    ///     .with_compression(CompressionConfig::for_json());
    /// ```
    pub fn with_compression(mut self, config: CompressionConfig) -> Self {
        use reinhardt_middleware::GZipMiddleware;
        self.middlewares.push(Arc::new(GZipMiddleware::new(config)));
        self
    }

    /// Add timeout middleware
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use reinhardt_micro::App;
    ///
    /// let app = App::new()
    ///     .with_timeout(Duration::from_secs(30));
    /// ```
    pub fn with_timeout(self, _duration: Duration) -> Self {
        // Note: Timeout middleware implementation is pending in reinhardt-middleware
        // For now, this is a no-op placeholder
        self
    }

    /// Add logging middleware with custom configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::{App, LoggingConfig};
    ///
    /// let app = App::new()
    ///     .with_logging(LoggingConfig::verbose());
    /// ```
    pub fn with_logging(mut self, _config: LoggingConfig) -> Self {
        use reinhardt_middleware::LoggingMiddleware;
        self.middlewares.push(Arc::new(LoggingMiddleware::new()));
        self
    }

    /// Add metrics middleware with custom configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::{App, MetricsConfig};
    ///
    /// let app = App::new()
    ///     .with_metrics(MetricsConfig::with_endpoint("/metrics"));
    /// ```
    pub fn with_metrics(mut self, config: MetricsConfig) -> Self {
        use reinhardt_middleware::MetricsMiddleware;
        self.middlewares.push(Arc::new(MetricsMiddleware::new(config)));
        self
    }
    /// Add a route to the application
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::App;
    ///
    /// fn handler() {
    ///     println!("Handler called");
    /// }
    ///
    /// let app = App::new()
    ///     .route("/", handler)
    ///     .route("/api/users", handler);
    // Routes are now registered with the app
    /// ```
    pub fn route_handler(self, path: &str, handler: Arc<dyn Handler>) -> Self {
        self.router.lock().unwrap().add_route(route_path(path, handler));
        self
    }

    /// Add a route to the application (handler-based API)
    pub fn route(self, path: &str, handler: Arc<dyn Handler>) -> Self {
        self.route_handler(path, handler)
    }
    /// Start serving the application
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_micro::App;
    ///
    /// # async fn example() -> reinhardt_micro::Result<()> {
    /// let app = App::new();
    ///
    // This would start the server on the specified address
    // Marked as no_run because it would block indefinitely
    /// app.serve("127.0.0.1:8000").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve(self, addr: &str) -> Result<()> {
        let socket_addr: SocketAddr = addr
            .parse()
            .map_err(|e| Error::ImproperlyConfigured(format!("invalid address: {}", e)))?;

        // Wrap the app (which implements Handler) and serve
        http_serve(socket_addr, Arc::new(self))
            .await
            .map_err(|e| Error::Internal(format!("server error: {}", e)))?;

        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

use async_trait::async_trait;

#[async_trait]
impl Handler for App {
    async fn handle(&self, request: Request) -> Result<Response> {
        // Create a router handler wrapper
        let router_handler = Arc::new(RouterHandler {
            router: self.router.clone(),
        });

        if self.middlewares.is_empty() {
            // No middleware, directly use router
            return router_handler.handle(request).await;
        }

        // Build middleware chain from last to first
        let mut handler: Arc<dyn Handler> = router_handler;

        for middleware in self.middlewares.iter().rev() {
            let middleware_clone = middleware.clone();
            let handler_clone = handler.clone();

            // Create a wrapper that applies this middleware
            handler = Arc::new(MiddlewareWrapper {
                middleware: middleware_clone,
                next: handler_clone,
            });
        }

        handler.handle(request).await
    }
}

/// Wrapper to call router through Mutex
struct RouterHandler {
    router: Arc<Mutex<DefaultRouter>>,
}

#[async_trait]
impl Handler for RouterHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        let router = self.router.lock().unwrap();
        router.handle(request).await
    }
}

/// Wrapper to apply a single middleware to a handler chain
struct MiddlewareWrapper {
    middleware: Arc<dyn Middleware>,
    next: Arc<dyn Handler>,
}

#[async_trait]
impl Handler for MiddlewareWrapper {
    async fn handle(&self, request: Request) -> Result<Response> {
        self.middleware.process(request, self.next.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let _app = App::new();
    }

    #[test]
    fn test_app_default() {
        let app1 = App::default();
        let app2 = App::new();
        // Both should create the same type of App with DefaultRouter
        assert_eq!(std::mem::size_of_val(&app1), std::mem::size_of_val(&app2));
    }

    use async_trait::async_trait;
    struct DummyHandler;

    #[async_trait]
    impl Handler for DummyHandler {
        async fn handle(&self, _req: Request) -> Result<Response> {
            Ok(Response::ok())
        }
    }

    #[test]
    fn test_app_route_chaining() {
        let handler = std::sync::Arc::new(DummyHandler);
        let _app = App::new()
            .route("/", handler.clone())
            .route("/api", handler);
    }

    #[tokio::test]
    #[ignore = "Network test - enable to run a real server"]
    async fn test_app_serve_runs() {
        let handler = std::sync::Arc::new(DummyHandler);
        let app = App::new().route("/", handler);
        let _ = app.serve("127.0.0.1:0").await;
    }
}
