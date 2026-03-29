//! Client-Side Router for reinhardt-pages
//!
//! This module provides a client-side routing system compatible with reinhardt-urls.
//! It supports the History API for navigation without full page reloads.
//!
//! ## Features
//!
//! - **Path Pattern Matching**: Django-style URL patterns (`/users/{id}/`)
//! - **History API Integration**: `pushState` and `popstate` handling
//! - **Named Routes**: Reverse URL lookup by route name
//! - **Reactive Navigation**: Signal-based current route tracking
//! - **Route Guards**: Optional authentication/authorization checks
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::router::{Router, Link, RouterOutlet};
//! use std::sync::Arc;
//!
//! // Create a router
//! let router = Arc::new(Router::new()
//!     .route("/", home_page)
//!     .route("/users/", user_list)
//!     .route("/users/{id}/", user_detail)
//!     .named_route("user_detail", "/users/{id}/", user_detail));
//!
//! // Create a router outlet to render current route
//! let outlet = RouterOutlet::new(router.clone());
//!
//! // Navigate programmatically
//! router.push("/users/42/");
//!
//! // Reverse URL lookup
//! let url = router.reverse("user_detail", &[("id", "42")]);
//! ```

mod components;
mod core;
mod handler;
mod history;
mod params;
mod pattern;

pub use components::{Link, Redirect, RouterOutlet, guard, guard_or};
pub use core::{PathError, Route, RouteMatch, Router, RouterError};
pub use history::{HistoryState, NavigationType, setup_popstate_listener};
pub use params::{FromPath, ParamContext, PathParams};
pub use pattern::{PathParam, PathPattern};
