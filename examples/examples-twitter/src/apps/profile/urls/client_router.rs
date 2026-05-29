//! Client-side routing for the profile application (SPA pages).
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The page components are
//! wasm-only, so the route registration becomes a no-op on native.

use reinhardt::ClientRouter;

#[cfg(wasm)]
use {
	crate::core::client::pages::{profile_edit_page, profile_page},
	reinhardt::ClientPath,
	uuid::Uuid,
};

/// Register the profile client routes onto the client router.
///
/// On native this is a no-op: the page components only exist on wasm.
pub fn client_url_patterns(c: ClientRouter) -> ClientRouter {
	#[cfg(wasm)]
	{
		c.route_path(
			"profile_edit",
			"/profile/{user_id}/edit",
			|ClientPath(user_id): ClientPath<Uuid>| profile_edit_page(user_id),
		)
		.route_path(
			"profile_detail",
			"/profile/{user_id}",
			|ClientPath(user_id): ClientPath<Uuid>| profile_page(user_id),
		)
	}
	#[cfg(not(wasm))]
	{
		c
	}
}
