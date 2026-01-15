//! URL configuration for tweet application
//!
//! Defines unified routes for tweets with both server and client routing.

use reinhardt::UnifiedRouter;
use reinhardt::pages::component::Page;

#[cfg(not(target_arch = "wasm32"))]
use reinhardt::pages::server_fn::ServerFnRouterExt;

#[cfg(not(target_arch = "wasm32"))]
use crate::apps::tweet::server::server_fn::{create_tweet, delete_tweet, list_tweets};

#[cfg(target_arch = "wasm32")]
use crate::core::client::pages::{home_page, timeline_page};

/// Unified routes for tweet application (client + server)
pub fn routes() -> UnifiedRouter<Page> {
	UnifiedRouter::new()
		// Server-side routes (server functions)
		.server(|s| {
			#[cfg(not(target_arch = "wasm32"))]
			{
				s.server_fn(create_tweet::marker)
					.server_fn(list_tweets::marker)
					.server_fn(delete_tweet::marker)
			}
			#[cfg(target_arch = "wasm32")]
			s
		})
		// Client-side routes (SPA)
		.client(|c| {
			#[cfg(target_arch = "wasm32")]
			{
				c.route("/", || home_page())
					.route("/timeline", || timeline_page())
			}
			#[cfg(not(target_arch = "wasm32"))]
			c
		})
}
