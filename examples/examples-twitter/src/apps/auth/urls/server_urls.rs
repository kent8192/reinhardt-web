//! Server-side URL patterns for the auth application.
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The `#[server_fn]` macro emits
//! a marker module on both targets (#4711), so `s.server_fn(<fn>::marker)`
//! compiles cross-target; on wasm the no-op `ServerRouter` discards each call.

use reinhardt::ServerRouter;

// `server_fn` is an inherent no-op on the wasm `ServerRouter`; on native it is
// provided by this extension trait, which only exists on native.
#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;

use crate::apps::auth::shared::server_fn::{current_user, login, logout, register};

/// Register the auth server functions onto the server router.
///
/// On wasm this is a no-op: the markers still exist (#4711) but the no-op
/// `ServerRouter::server_fn` absorbs and discards each registration.
pub fn server_url_patterns(s: ServerRouter) -> ServerRouter {
	s.server_fn(login::marker)
		.server_fn(register::marker)
		.server_fn(logout::marker)
		.server_fn(current_user::marker)
}
