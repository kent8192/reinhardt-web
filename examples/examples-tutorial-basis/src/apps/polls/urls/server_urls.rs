//! Server-side URL configuration for the polls application.
//!
//! The polls app exposes every dynamic data path through `#[server_fn]`
//! (registered in `src/config/urls.rs`), so this router is intentionally
//! empty — there are no native-only `#[get]/#[post]` views to mount. The
//! function is kept around because:
//!
//! 1. **Symmetry with `users`** — every app in the tutorial declares both
//!    a `client_router` and a `server_urls` submodule, even when the
//!    server side has nothing to register today. New polls-app HTTP
//!    endpoints (a CSV export, an RSS feed, …) drop into this function
//!    without touching the aggregator.
//! 2. **Inventory presence** — `#[url_patterns(InstalledApp::polls,
//!    mode = server)]` emits a `UrlPatternsRegistration` even for an
//!    empty router, which keeps the polls app visible to framework
//!    tooling that walks the inventory (path-coverage checks, route
//!    introspection, etc.).
//! 3. **Discoverability** — readers grepping for the per-app server
//!    surface find this file and a clear "no endpoints today" rationale
//!    instead of guessing whether the omission is intentional.
//!
//! The `#[url_patterns]` macro derives the path prefix from
//! `InstalledApp::polls` (= `"polls"`), so even if endpoints are added
//! later the aggregating `config/urls.rs` does not need an explicit
//! `.mount("/polls/", ...)` call.
use crate::config::apps::InstalledApp;
use reinhardt::ServerRouter;
use reinhardt::url_patterns;
#[url_patterns(InstalledApp::polls, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}
