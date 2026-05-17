//! URL configuration for the snippets app.
//!
//! Each submodule carries `#[url_patterns(InstalledApp::snippets, mode = ...)]`,
//! which the framework discovers through the `url_resolvers` /
//! `client_url_resolvers` / `ws_url_resolvers` modules emitted by the macro.
//! The top-level `#[routes]` in `src/config/urls.rs` walks those modules
//! across every installed app to build the `ResolvedUrls` accessor surface.
//!
//! - `server_urls` — `#[url_patterns(..., mode = server)]` → `ServerRouter`
//! - `client_router` — `#[url_patterns(..., mode = client)]` → empty
//!   `ClientRouter` stub (REST-only example, no SPA)
//! - `ws_urls` — `#[url_patterns(..., mode = ws)]` → empty
//!   `WebSocketRouter` stub (REST-only example, no WS)
//!
//! The `client_router` and `ws_urls` stubs exist only because plain
//! `#[routes]` (without the `standalone` flag) requires every installed
//! app to expose all three resolver modules. They could be removed by
//! switching `src/config/urls.rs` back to `#[routes(standalone)]`, but
//! that flag is meant for projects that do not use `installed_apps!` —
//! and this example does use it.

pub mod client_router;
pub mod server_urls;
pub mod ws_urls;

// Re-export the macro-emitted resolver modules at the `urls` level so the
// top-level `#[routes]` macro in `src/config/urls.rs` can find them at the
// canonical flat paths `crate::apps::snippets::urls::url_resolvers` (server)
// and `crate::apps::snippets::urls::client_url_resolvers` (client). The
// `ws_url_resolvers` counterpart is referenced through the nested
// `ws_urls::ws_url_resolvers` path the routes macro already expects, so it
// does not need a flat re-export.
pub use client_router::client_url_resolvers;
pub use server_urls::url_resolvers;
