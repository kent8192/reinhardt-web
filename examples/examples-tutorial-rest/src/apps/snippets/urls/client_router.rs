//! Client-side (SPA) router stub for the snippets app.
//!
//! This is a REST-only tutorial example — there is no WASM frontend — so
//! `client_url_patterns` deliberately returns an empty `ClientRouter`.
//! The stub exists only to satisfy the non-`standalone` `#[routes]` macro
//! in `src/config/urls.rs`, which walks every installed app's
//! `urls::client_url_resolvers` module to build the cross-target
//! `ResolvedUrls` accessor surface.
//!
//! Removing this file would force `src/config/urls.rs` back to
//! `#[routes(standalone)]`, which is the right choice for projects that
//! do not use `installed_apps!` (the macro's documented purpose for
//! `standalone`). This example does use `installed_apps!`, so we keep
//! plain `#[routes]` and stub the client side instead.

use reinhardt::ClientRouter;
use reinhardt::url_patterns;

use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::snippets, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
}
