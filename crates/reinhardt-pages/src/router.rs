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
//!
//! // Create a router
//! let router = Router::new()
//!     .route("/", home_page)
//!     .route("/users/", user_list)
//!     .route("/users/{id}/", user_detail)
//!     .named_route("user_detail", "/users/{id}/", user_detail);
//!
//! // Navigate programmatically
//! router.push("/users/42/");
//!
//! // Reverse URL lookup
//! let url = router.reverse("user_detail", &[("id", "42")]);
//! ```

mod components;
mod core;
mod history;
mod pattern;

pub use components::{Link, Redirect, RouterOutlet, guard, guard_or};
pub use core::{Route, RouteMatch, Router, RouterError};
pub use history::{HistoryState, NavigationType};
pub use pattern::{PathParam, PathPattern};
