//! Server-side URL patterns for the tweet application.
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The `server_fn` markers are
//! native-only, so the registration becomes a no-op on wasm.

use reinhardt::ServerRouter;

#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(native)]
use crate::apps::tweet::shared::server_fn::{create_tweet, delete_tweet, list_tweets};

/// Register the tweet server functions onto the server router.
///
/// On wasm this is a no-op: the server-fn markers do not exist on that target,
/// and the no-op `ServerRouter` discards the result anyway.
pub fn server_url_patterns(s: ServerRouter) -> ServerRouter {
	#[cfg(native)]
	{
		s.server_fn(create_tweet::marker)
			.server_fn(list_tweets::marker)
			.server_fn(delete_tweet::marker)
	}
	#[cfg(not(native))]
	{
		s
	}
}
