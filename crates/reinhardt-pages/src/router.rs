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
//!
//! # Migration to `urls::ClientRouter`
//!
//! As of `0.1.0-rc.27`, `Router` and its related types are
//! `#[deprecated]`. New code should use
//! `reinhardt_urls::routers::ClientRouter` instead. The migration
//! map for the most common patterns is:
//!
//! Note: in the table below, `urls::*` abbreviates
//! `reinhardt_urls::routers::*`.
//!
//! ```text
//! pages::Router               -> urls::ClientRouter
//! pages::Route                -> urls::ClientRoute
//! pages::RouteMatch           -> urls::ClientRouteMatch
//! pages::PathPattern          -> urls::ClientPathPattern
//! pages::PathParams<T>        -> urls::Path<T>
//! pages::NavigationSubscription -> urls::NavigationSubscription
//! pages::RouterError          -> urls::RouterError
//! pages::PathError            -> urls::PathError
//! ClientLauncher::router(F)   -> ClientLauncher::router_client(F)
//! ```
//!
//! `Link` and `RouterOutlet` remain in this module and are NOT
//! deprecated — they are rendering primitives with no migration target.
//!
//! See `kent8192/reinhardt-web#4234` for the full design.

mod components;
mod core;
mod handler;
mod history;
mod params;
mod pattern;

pub use components::{Link, Redirect, RouterOutlet, guard, guard_or};
#[allow(deprecated)] // (Refs #4234) Re-exporting deprecated symbols intentionally.
pub use core::{NavigationSubscription, PathError, Route, RouteMatch, Router, RouterError};
pub use history::{HistoryState, NavigationType};
// `setup_popstate_listener` is wasm-only — see `history` module docs.
#[cfg(wasm)]
pub use history::setup_popstate_listener;
#[allow(deprecated)] // (Refs #4234) Re-exporting deprecated `PathParams` intentionally.
pub use params::{FromPath, ParamContext, PathParams};
#[allow(deprecated)] // (Refs #4234) Re-exporting deprecated `PathPattern` intentionally.
pub use pattern::{PathParam, PathPattern};
