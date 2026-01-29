//! URL configuration for examples-twitter project
//!
//! This project uses reinhardt-pages with Server Functions for API communication.
//! Each app defines unified routes (server + client) in `urls.rs`, which are mounted here.

use reinhardt::UnifiedRouter;
use reinhardt::routes;

// Import app URL modules
use crate::apps::{auth, dm, profile, relationship, tweet};
use crate::config::middleware::create_cors_middleware;

/// Build URL patterns for the application
///
/// This project uses:
/// - Server Functions (`#[server_fn]`) for API communication
/// - Client routing for SPA navigation
/// - CORS middleware for cross-origin requests with credentials support
///
/// Each app's `routes()` function returns a `UnifiedRouter` with both
/// server and client routes defined.
#[routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new()
		// Mount each app's unified routes
		.mount_unified("/", auth::urls::routes())
		.mount_unified("/", tweet::urls::routes())
		.mount_unified("/", profile::urls::routes())
		.mount_unified("/", relationship::urls::routes())
		.mount_unified("/", dm::urls::routes())
		// Apply CORS middleware for cross-origin API access
		.with_middleware(create_cors_middleware())
}
