//! Server-side URL patterns for the relationship application.
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The `server_fn` markers are
//! native-only, so the registration becomes a no-op on wasm.

use reinhardt::ServerRouter;

#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(native)]
use crate::apps::relationship::shared::server_fn::{
	fetch_followers, fetch_following, follow_user, unfollow_user,
};

/// Register the relationship server functions onto the server router.
///
/// On wasm this is a no-op: the server-fn markers do not exist on that target,
/// and the no-op `ServerRouter` discards the result anyway.
pub fn server_url_patterns(s: ServerRouter) -> ServerRouter {
	#[cfg(native)]
	{
		s.server_fn(follow_user::marker)
			.server_fn(unfollow_user::marker)
			.server_fn(fetch_followers::marker)
			.server_fn(fetch_following::marker)
	}
	#[cfg(not(native))]
	{
		s
	}
}
