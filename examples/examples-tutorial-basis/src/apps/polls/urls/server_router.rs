//! Server-side URL configuration for the polls application.
//!
//! The polls app exposes its dynamic data path through `#[server_fn]`
//! handlers. Register them here so the per-app server surface lives next
//! to the app's models, client router, and handler bodies.

use crate::apps::polls::server_fn::{
	create_choice, create_question, delete_choice, delete_question, get_question_detail,
	get_question_results, get_questions, submit_vote, update_choice, update_question, vote,
};
use reinhardt::ServerRouter;
use reinhardt::pages::server_fn::ServerFnRouterExt;

pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
		.server_fn(get_questions::marker)
		.server_fn(get_question_detail::marker)
		.server_fn(get_question_results::marker)
		.server_fn(vote::marker)
		.server_fn(submit_vote::marker)
		.server_fn(create_question::marker)
		.server_fn(update_question::marker)
		.server_fn(delete_question::marker)
		.server_fn(create_choice::marker)
		.server_fn(update_choice::marker)
		.server_fn(delete_choice::marker)
}
