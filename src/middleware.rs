//! Middleware module.
//!
//! This module provides middleware for request/response processing.
//! It is available on native targets (P0: native-only) whenever a middleware
//! component feature or a preset enables the middleware dependency. Individual
//! features such as `middleware-cors` do not require the umbrella `middleware`
//! feature.
//!
//! # Examples
//!
//! ```rust
//! # #[cfg(feature = "middleware-cors")]
//! # {
//! use reinhardt::middleware::CorsMiddleware;
//!
//! let middleware = CorsMiddleware::permissive();
//! # }
//! ```

pub use reinhardt_middleware::*;
