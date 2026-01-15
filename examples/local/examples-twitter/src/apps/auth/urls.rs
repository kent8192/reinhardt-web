//! URL configuration for auth application
//!
//! Defines unified routes for authentication with both server and client routing.

use reinhardt::UnifiedRouter;

#[cfg(not(target_arch = "wasm32"))]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(not(target_arch = "wasm32"))]
use crate::apps::auth::server::server_fn::{current_user, login, logout, register};

#[cfg(target_arch = "wasm32")]
use crate::apps::auth::client::components::{login_form, register_form};

/// Unified routes for auth application (client + server)
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new()
		// Server-side routes (server functions)
		.server(|s| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				s.server_fn(login::marker)
					.server_fn(register::marker)
					.server_fn(logout::marker)
					.server_fn(current_user::marker)
			}
			#[cfg(target_arch = "wasm32")]
			s
		})
		// Client-side routes (SPA)
		.client(|c| {
			#[cfg(target_arch = "wasm32")]
			{
				c.route("/login", || login_form())
					.route("/register", || register_form())
			}
			#[cfg(not(target_arch = "wasm32"))]
			c
		})
}
