//! URL configuration for examples-twitter project
//!
//! This project uses reinhardt-pages with Server Functions for API communication.
//! Each app defines unified routes (server + client) in `urls.rs`, which are mounted here.

use reinhardt::pages::component::Page;
use reinhardt::UnifiedRouter;

// Import app URL modules
use crate::apps::{auth, dm, profile, relationship, tweet};

/// Build URL patterns for the application
///
/// This project uses:
/// - Server Functions (`#[server_fn]`) for API communication
/// - Client routing for SPA navigation
///
/// Each app's `routes()` function returns a `UnifiedRouter<Page>` with both
/// server and client routes defined.
///
/// Note: `#[routes]` macro is not compatible with `UnifiedRouter<Page>` as it
/// expects `ServerRouter`. For Pages projects, routes are registered via
/// `PagesRegistration` in server configuration.
pub fn routes() -> UnifiedRouter<Page> {
	UnifiedRouter::new()
		// Mount each app's unified routes
		.mount_unified("/", auth::urls::routes())
		.mount_unified("/", tweet::urls::routes())
		.mount_unified("/", profile::urls::routes())
		.mount_unified("/", relationship::urls::routes())
		.mount_unified("/", dm::urls::routes())
}
