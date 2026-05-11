//! URL configuration for examples-tutorial-basis project
//!
//! The `routes` function defines the top-level project router. Per-app routes
//! are registered separately by `#[url_patterns(InstalledApp::<app>, mode = ...)]`
//! attributes on the app's URL functions (see
//! `apps/polls/urls/server_urls.rs::server_url_patterns`), so this file no
//! longer needs to mount them explicitly.

use reinhardt::UnifiedRouter;
#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;
#[cfg(native)]
use reinhardt::routes;

// Import server_fn marker modules (snake_case + ::marker)
#[cfg(native)]
use crate::server_fn::polls::{
	get_question_detail, get_question_results, get_questions, get_vote_form_metadata, submit_vote,
	vote,
};

#[cfg_attr(native, routes(standalone))]
pub fn routes() -> UnifiedRouter {
	// Server: register server functions. The polls app router is auto-mounted
	// via `#[url_patterns(InstalledApp::polls, mode = server)]`.
	#[cfg(native)]
	let router = UnifiedRouter::new().server(|s| {
		s.server_fn(get_questions::marker)
			.server_fn(get_question_detail::marker)
			.server_fn(get_question_results::marker)
			.server_fn(vote::marker)
			.server_fn(get_vote_form_metadata::marker)
			.server_fn(submit_vote::marker)
	});

	// Client: empty router (polls routes are server-only)
	#[cfg(wasm)]
	let router = UnifiedRouter::new();

	router
}
