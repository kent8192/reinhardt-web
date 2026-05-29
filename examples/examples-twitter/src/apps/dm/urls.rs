//! URL configuration for dm application
//!
//! Defines unified routes for direct messaging with both server and client
//! routing. The per-target builder bodies live in the `server_urls` and
//! `client_router` submodules, so this aggregator stays free of `#[cfg]`
//! branches (issue #4569).
//!
//! Server functions handle REST API access. WebSocket handlers are registered
//! separately through the WebSocket middleware.

use reinhardt::UnifiedRouter;

pub mod client_router;
pub mod server_urls;

/// Unified routes for dm application (client + server)
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new()
		.server(server_urls::server_url_patterns)
		.client(client_router::client_url_patterns)
}
