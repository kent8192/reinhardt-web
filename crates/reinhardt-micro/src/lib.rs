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

#[cfg(feature = "di")]
pub use reinhardt_di::Depends;

#[cfg(feature = "database")]
pub use reinhardt_orm as orm;

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{Error, Request, Response, Result};

    #[cfg(feature = "params")]
    pub use reinhardt_params::{Cookie, Form, Header, Json, Path, Query};

    #[cfg(feature = "di")]
    pub use reinhardt_di::Depends;

    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};
}

use reinhardt_routers::{path as route_path, DefaultRouter, Router};
use reinhardt_server::serve as http_serve;
use reinhardt_types::Handler;
use std::net::SocketAddr;
use std::sync::Arc;

/// Application builder for creating micro services
pub struct App {
    router: DefaultRouter,
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
            router: DefaultRouter::new(),
        }
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
    pub fn route_handler(mut self, path: &str, handler: Arc<dyn Handler>) -> Self {
        self.router.add_route(route_path(path, handler));
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

        // DefaultRouter implements Handler, wrap it and serve
        http_serve(socket_addr, Arc::new(self.router))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let _app = App::new();
    }

    #[test]
    fn test_app_default() {
        let app = App::default();
        assert_eq!(std::mem::size_of_val(&app), 0); // Empty struct
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
