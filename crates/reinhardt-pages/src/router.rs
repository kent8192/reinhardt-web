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
//!     .route("home", "/", home_page)
//!     .route("user_list", "/users/", user_list)
//!     .route("user_detail", "/users/{id}/", user_detail));
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
mod history;
mod navigate;

/// Manouche DSL v2 spec §4.3 `FromRequest`-based page handlers.
///
/// Re-exports the building blocks for
/// `ClientRouter::page<F, P>(pattern, handler)` where
/// `P: FromRequest`. The canonical implementations live in
/// `reinhardt_urls::routers::client_router::from_request` (because
/// `ClientRouter` itself is defined there); they are re-exposed here
/// so application code can write
/// `use reinhardt_pages::router::request::FromRequest;` matching the
/// spec's namespace.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::router::request::{
///     ExtractError, FromRequest, PathParam, RouteContext,
/// };
/// use reinhardt_urls::routers::ClientRouter;
///
/// struct UserPageProps { id: PathParam<i32> }
///
/// impl FromRequest for UserPageProps {
///     fn from_request(ctx: &RouteContext) -> Result<Self, ExtractError> {
///         Ok(Self { id: PathParam::extract(ctx, "id")? })
///     }
/// }
/// ```
///
/// The legacy non-generic `PathParam` re-exported at
/// `reinhardt_pages::router::PathParam` (deprecated since
/// `0.1.0-rc.27`) is unrelated to the v2 `PathParam<T>` extractor in
/// this submodule.
pub mod request {
	pub use reinhardt_urls::routers::client_router::from_request::{
		ExtractError, FromRequest, PathParam, QueryParam, RouteContext,
	};
}

pub use components::{Link, Redirect, guard, guard_or};
pub use history::{HistoryState, NavigationType};
pub use navigate::navigate;
pub use reinhardt_urls::routers::ClientRouter;
// `setup_popstate_listener` is wasm-only — see `history` module docs.
#[cfg(wasm)]
pub use history::setup_popstate_listener;
