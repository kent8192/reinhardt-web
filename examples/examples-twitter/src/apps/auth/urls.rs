//! URL configuration for auth application
//!
//! Defines unified routes for authentication with both server and client routing.
#[cfg(wasm)]
use crate::apps::auth::client::components::{login_form, register_form};
#[cfg(native)]
use crate::apps::auth::shared::server_fn::{current_user, login, logout, register};
use reinhardt::UnifiedRouter;
#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;
/// Unified routes for auth application (client + server)
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new()
		.server(|s| {
			#[cfg(native)]
			{
				s.server_fn(login::marker)
					.server_fn(register::marker)
					.server_fn(logout::marker)
					.server_fn(current_user::marker)
			}
			#[cfg(wasm)]
			s
		})
		.client(|c| {
			#[cfg(wasm)]
			{
				c.route("/login", || login_form())
					.route("/register", || register_form())
			}
			#[cfg(native)]
			c
		})
}
