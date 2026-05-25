//! Server-side URL configuration for the {{ app_name }} application.
//!
//! Per-app routers are NOT aggregated automatically — endpoints added here
//! become reachable only after `config/urls.rs` wires this function in
//! (e.g. via `router.mount("/{{ app_name }}/", server_url_patterns())` or
//! `router.server(|s| s.server_fn(...))` for individual server functions).
//!
//! # Placeholder note
//!
//! The freshly generated function returns an empty `ServerRouter`. Register
//! views/endpoints here:
//!
//! ```rust,ignore
//! use crate::apps::{{ app_name }}::views;
//!
//! ServerRouter::new()
//!     .endpoint(views::index)
//! ```

use reinhardt::ServerRouter;

pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}
