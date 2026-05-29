//! Server-side URL patterns for the auth application.
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The `server_fn` markers are
//! native-only, so the registration becomes a no-op on wasm.

use reinhardt::ServerRouter;

#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(native)]
use crate::apps::auth::shared::server_fn::{current_user, login, logout, register};

/// Register the auth server functions onto the server router.
///
/// On wasm this is a no-op: the server-fn markers do not exist on that target,
/// and the no-op `ServerRouter` discards the result anyway.
pub fn server_url_patterns(s: ServerRouter) -> ServerRouter {
	#[cfg(native)]
	{
		s.server_fn(login::marker)
			.server_fn(register::marker)
			.server_fn(logout::marker)
			.server_fn(current_user::marker)
	}
	#[cfg(not(native))]
	{
		s
	}
}
