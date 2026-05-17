//! WebSocket router stub for the snippets app.
//!
//! This is a REST-only tutorial example — there are no WebSocket consumers —
//! so `ws_url_patterns` deliberately returns an empty `WebSocketRouter`.
//! The stub exists only to satisfy the non-`standalone` `#[routes]` macro
//! in `src/config/urls.rs`, which walks every installed app's
//! `urls::ws_urls::ws_url_resolvers` module to populate `ResolvedUrls`.
//! See the discussion #3914 migration note baked into the routes macro
//! (`reinhardt-core/macros/src/routes_registration.rs`): the WS resolver
//! was hoisted under `urls/` in rc.19, and every app must expose either
//! a real `ws_url_patterns` or a stub like this one.
//!
//! `WebSocketRouter` is only exported from `reinhardt` when the
//! `websockets` feature is enabled, which is why this example's
//! `Cargo.toml` opts into that feature.

use reinhardt::WebSocketRouter;
use reinhardt::url_patterns;

use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::snippets, mode = ws)]
pub fn ws_url_patterns() -> WebSocketRouter {
	WebSocketRouter::new()
}
