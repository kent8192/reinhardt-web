//! URL routing and proxy utilities for Reinhardt framework.
//!
//! This crate provides URL routing, pattern matching, and proxy functionality
//! for the Reinhardt web framework. It is a unified interface over the following
//! internal crates:
//!
//! - `reinhardt-routers`: URL routing and pattern matching with middleware support
//! - `reinhardt-routers-macros`: Compile-time URL validation macros
//! - `reinhardt-proxy`: Lazy relationship loading for ORM
//!
//! ## Features
//!
//! ### Route Middleware Support
//!
//! Per-route middleware configuration is now available. You can attach middleware
//! to specific routes or route groups:
//!
//! ```rust,ignore
//! use reinhardt_routers::{UnifiedRouter, RouteGroup};
//! use reinhardt_middleware::LoggingMiddleware;
//! use hyper::Method;
//! use std::sync::Arc;
//! # use reinhardt_apps::{Request, Response, Result};
//!
//! # async fn handler(_req: Request) -> Result<Response> { Ok(Response::ok()) }
//! # async fn users_handler(_req: Request) -> Result<Response> { Ok(Response::ok()) }
//! # async fn settings_handler(_req: Request) -> Result<Response> { Ok(Response::ok()) }
//! let router = UnifiedRouter::new()
//!     .function("/public", Method::GET, handler)
//!     .function("/protected", Method::GET, handler)
//!         .with_route_middleware(Arc::new(LoggingMiddleware)) // Route-specific middleware
//!     .mount("/admin", RouteGroup::new()
//!         .with_middleware(Arc::new(LoggingMiddleware)) // Group middleware
//!         .function("/users", Method::GET, users_handler)
//!         .function("/settings", Method::GET, settings_handler)
//!         .build());
//! ```
//!
//! **Features**:
//! - Per-route middleware configuration
//! - Route group middleware with inheritance
//! - Middleware composition and chaining
//! - Proper execution order: global → group → route → handler
//!
//! See `reinhardt-routers` crate documentation for detailed usage and examples.

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "routers")]
#[cfg_attr(docsrs, doc(cfg(feature = "routers")))]
pub use reinhardt_routers as routers;

#[cfg(feature = "routers-macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "routers-macros")))]
pub use reinhardt_routers_macros as routers_macros;

#[cfg(feature = "proxy")]
#[cfg_attr(docsrs, doc(cfg(feature = "proxy")))]
pub use reinhardt_proxy as proxy;

// Re-export commonly used types from routers
#[cfg(feature = "routers")]
#[cfg_attr(docsrs, doc(cfg(feature = "routers")))]
pub mod prelude {
	pub use reinhardt_routers::{
		PathPattern, Route, RouteGroup, Router, UnifiedRouter, clear_script_prefix,
		get_script_prefix, set_script_prefix,
	};
}
