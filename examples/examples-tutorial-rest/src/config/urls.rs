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
//! neither client (`#[url_patterns(..., mode = client)]`) nor WebSocket
//! (`#[url_patterns(..., mode = ws)]`) surface. `server_only` (Issue
//! #4509) instructs the routes macro to skip the per-app
//! `client_url_resolvers` / `ws_url_resolvers` module lookups so the
//! `snippets` app needs no stub modules — which is what PR #4508's
//! `client_router.rs` and `ws_urls.rs` stubs (plus the `websockets`
//! feature opt-in) were working around. The `crate::urls::url_prelude`
//! module and the per-app `ResolvedUrls::<app>()` accessor — which the
//! `standalone` flag would suppress — remain available because
//! `server_only` only gates client/ws emission, not server.

use reinhardt::prelude::*;
use reinhardt::routes;

#[routes(server_only)]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}
