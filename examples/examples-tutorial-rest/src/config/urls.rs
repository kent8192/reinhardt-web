//! URL configuration for examples-tutorial-rest project
//!
//! The `routes` function defines all URL patterns for this project.
//!
//! The `/api/` prefix is a literal path (no `{...}` parameters), which
//! satisfies the rc.24 guard that panics if `ServerRouter::mount()` receives
//! a prefix containing path parameters.
//!
//! `#[routes(server_only)]` is used because this project consumes
//! `installed_apps!` (see `src/config/apps.rs`) but is REST-only — it has
//! no client or WebSocket surface. `server_only` (Issue #4509) instructs
//! the routes macro to skip the per-app client/ws resolver lookups.

use reinhardt::prelude::*;
use reinhardt::routes;

#[routes(server_only)]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}
