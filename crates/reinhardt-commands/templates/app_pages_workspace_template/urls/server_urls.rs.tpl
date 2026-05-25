//! Server-side URL configuration for the {{ app_name }} application crate.

use reinhardt::ServerRouter;

#[allow(unused_imports)] // `views` will be used once endpoints are added.
use crate::views;

pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
	// Register endpoints here, e.g.:
	//     .endpoint(views::index)
}
