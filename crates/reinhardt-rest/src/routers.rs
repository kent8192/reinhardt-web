//! REST API routers
//!
//! Re-exports router types from reinhardt-routers.
//!
//! ## Example
//!
//! ```rust
//! use crate::routers::DefaultRouter;
//!
//! let router = DefaultRouter::new();
//! // Note: To register a viewset, your type must implement the ViewSet trait
//! // router.register_viewset("users", Arc::new(YourViewSet));
//! ```

// Re-export router types from reinhardt-routers
pub use reinhardt_urls::routers::{DefaultRouter, Router};

// Re-export additional types needed for URL patterns
pub use reinhardt_urls::routers::{Route, UrlPattern};
