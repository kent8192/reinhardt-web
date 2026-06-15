//! Server-side URL patterns for the users application.
//!
//! Authentication is exposed via `#[server_fn]` handlers. Register them
//! here so the users app owns its server surface.

use crate::apps::users::server_fn::{current_user, login, logout, register};
use reinhardt::ServerRouter;
use reinhardt::pages::server_fn::ServerFnRouterExt;

pub(super) fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
		.server_fn(login::marker)
		.server_fn(logout::marker)
		.server_fn(register::marker)
		.server_fn(current_user::marker)
}
