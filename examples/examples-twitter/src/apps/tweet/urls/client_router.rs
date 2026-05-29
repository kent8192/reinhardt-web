//! Client-side routing for the tweet application (SPA pages).
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The page components are
//! wasm-only, so the route registration becomes a no-op on native.

use reinhardt::ClientRouter;

#[cfg(wasm)]
use crate::core::client::pages::{home_page, timeline_page};

/// Register the tweet client routes onto the client router.
///
/// On native this is a no-op: the page components only exist on wasm.
pub fn client_url_patterns(c: ClientRouter) -> ClientRouter {
	#[cfg(wasm)]
	{
		c.route("home", "/", || home_page())
			.route("timeline", "/timeline", || timeline_page())
	}
	#[cfg(not(wasm))]
	{
		c
	}
}
