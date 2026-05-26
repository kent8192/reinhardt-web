//! Client-side routing for the {{ app_name }} SPA.
//!
//! Route names are namespaced under `{{ app_name }}` (e.g.
//! `{{ app_name }}:index`). Pass `client_url_patterns()` explicitly to
//! `ClientLauncher::router_client(...)` in `src/client/lib.rs` (or merge
//! it with other apps' routers there) for the routes to become active.
//!
//! # Placeholder note
//!
//! The freshly generated function returns an empty `ClientRouter`. Wire
//! the placeholder page (or your real pages) once they exist:
//!
//! ```rust,ignore
//! use reinhardt::ClientPath;
//! use crate::apps::{{ app_name }}::client::pages;
//!
//! ClientRouter::new()
//!     .route("placeholder", "/", pages::placeholder_page)
//!     .route_path(
//!         "detail",
//!         "/items/{id}/",
//!         |ClientPath(id): ClientPath<i64>| pages::item_detail_page(id),
//!     )
//! ```

use reinhardt::ClientRouter;

pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
}
