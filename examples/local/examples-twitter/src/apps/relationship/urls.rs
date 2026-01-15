//! URL configuration for relationship application
//!
//! Defines unified routes for user relationships (follow/unfollow).

use reinhardt::UnifiedRouter;
use reinhardt::pages::component::Page;

#[cfg(not(target_arch = "wasm32"))]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(not(target_arch = "wasm32"))]
use crate::apps::relationship::server::server_fn::{
	fetch_followers, fetch_following, follow_user, unfollow_user,
};

/// Unified routes for relationship application (server only)
///
/// This app currently only has server-side routes.
/// Client-side relationship management is handled through profile components.
pub fn routes() -> UnifiedRouter<Page> {
	UnifiedRouter::new()
		// Server-side routes (server functions)
		.server(|s| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				s.server_fn(follow_user::marker)
					.server_fn(unfollow_user::marker)
					.server_fn(fetch_followers::marker)
					.server_fn(fetch_following::marker)
			}
			#[cfg(target_arch = "wasm32")]
			s
		})
	// No client-side routes - relationship UI is embedded in profile components
}
