//! URL configuration for auth application
//!
//! Defines unified routes for authentication with both server and client
//! routing. The per-target builder bodies live in the `server_urls` and
//! `client_router` submodules, so this aggregator stays free of `#[cfg]`
//! branches (issue #4569).

use reinhardt::UnifiedRouter;

pub mod client_router;
pub mod server_urls;

/// Unified routes for auth application (client + server)
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new()
		.server(server_urls::server_url_patterns)
		.client(client_router::client_url_patterns)
}
