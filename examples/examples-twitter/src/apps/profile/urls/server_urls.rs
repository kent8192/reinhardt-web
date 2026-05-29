//! Server-side URL patterns for the profile application.
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The `server_fn` markers are
//! native-only, so the registration becomes a no-op on wasm.

use reinhardt::ServerRouter;

#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(native)]
use crate::apps::profile::shared::server_fn::{fetch_profile, update_profile, update_profile_form};

/// Register the profile server functions onto the server router.
///
/// On wasm this is a no-op: the server-fn markers do not exist on that target,
/// and the no-op `ServerRouter` discards the result anyway.
pub fn server_url_patterns(s: ServerRouter) -> ServerRouter {
	#[cfg(native)]
	{
		s.server_fn(fetch_profile::marker)
			.server_fn(update_profile::marker)
			.server_fn(update_profile_form::marker)
	}
	#[cfg(not(native))]
	{
		s
	}
}
