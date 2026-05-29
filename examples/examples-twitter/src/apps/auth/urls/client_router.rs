//! Client-side routing for the auth application (SPA pages).
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The page components are
//! wasm-only, so the route registration becomes a no-op on native.

use reinhardt::ClientRouter;

#[cfg(wasm)]
use crate::apps::auth::client::components::{login_form, register_form};

/// Register the auth client routes onto the client router.
///
/// On native this is a no-op: the page components only exist on wasm.
pub fn client_url_patterns(c: ClientRouter) -> ClientRouter {
	#[cfg(wasm)]
	{
		c.route("login", "/login", || login_form())
			.route("register", "/register", || register_form())
	}
	#[cfg(not(wasm))]
	{
		c
	}
}
