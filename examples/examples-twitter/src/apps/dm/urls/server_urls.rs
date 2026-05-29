//! Server-side URL patterns for the dm application.
//!
//! The per-target builder body lives here so the `urls::routes()` aggregator
//! stays free of `#[cfg]` branches (issue #4569). The `server_fn` markers are
//! native-only, so the registration becomes a no-op on wasm.

use reinhardt::ServerRouter;

#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(native)]
use crate::apps::dm::shared::server_fn::{
	create_room, get_room, list_messages, list_rooms, mark_as_read, send_message,
};

/// Register the dm server functions onto the server router.
///
/// On wasm this is a no-op: the server-fn markers do not exist on that target,
/// and the no-op `ServerRouter` discards the result anyway.
pub fn server_url_patterns(s: ServerRouter) -> ServerRouter {
	#[cfg(native)]
	{
		s.server_fn(create_room::marker)
			.server_fn(list_rooms::marker)
			.server_fn(get_room::marker)
			.server_fn(send_message::marker)
			.server_fn(list_messages::marker)
			.server_fn(mark_as_read::marker)
	}
	#[cfg(not(native))]
	{
		s
	}
}
