//! URL configuration for relationship application
//!
//! Defines unified routes for user relationships (follow/unfollow). The
//! server-side builder body lives in the `server_urls` submodule, so this
//! aggregator stays free of `#[cfg]` branches (issue #4569).
//!
//! This app currently only has server-side routes; client-side relationship
//! management is handled through profile components.

use reinhardt::UnifiedRouter;

pub mod server_urls;

/// Unified routes for relationship application (server only)
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().server(server_urls::server_url_patterns)
}
