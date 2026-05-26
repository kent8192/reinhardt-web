//! Server-side URL patterns for the users application.
//!
//! Defines no HTTP endpoints of its own — authentication is exposed via
//! server functions registered in `crate::config::urls::routes`. This empty
//! aggregator exists for discoverability and symmetry with `polls`.

use reinhardt::ServerRouter;

pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}
