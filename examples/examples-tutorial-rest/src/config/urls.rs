//! URL configuration for examples-tutorial-rest project
//!
//! The `routes` function defines all URL patterns for this project.
//!
//! REST-only project: `server_only` (Issue #4509) skips the unused client
//! and WebSocket resolver lookups so the macro does not require
//! `client_url_resolvers` or `ws_url_resolvers` modules per app, while
//! preserving the `ResolvedUrls::snippets()` accessor (which
//! `#[routes(standalone)]` would suppress). This restores PR #4508's
//! intended layout — plain `#[routes]` for REST-only apps — without stubs.

use reinhardt::prelude::*;
use reinhardt::routes;

#[routes(server_only)]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}
