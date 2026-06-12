//! Server-side URL configuration for the {{ app_name }} application.
//!
//! Per-app routers are NOT aggregated automatically — endpoints added here
//! become reachable only after `config/urls.rs` aggregates
//! `crate::apps::{{ app_name }}::urls::server_url_patterns()`.
//!
//! # Placeholder note
//!
//! The freshly generated function returns an empty `ServerRouter`. Register
//! views/endpoints and server-function markers here:
//!
//! ```rust,ignore
//! use crate::apps::{{ app_name }}::{server_fn, views};
//! use reinhardt::pages::server_fn::ServerFnRouterExt;
//!
//! ServerRouter::new()
//!     .endpoint(views::index)
//!     .server_fn(server_fn::some_fn::marker)
//! ```

use reinhardt::ServerRouter;

pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}
