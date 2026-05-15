//! Client-side routing for the polls SPA.
//!
//! Routes are declared with `#[url_patterns(InstalledApp::polls, mode = client)]`,
//! which auto-registers the router via inventory. The WASM entry point
//! consumes this builder through `ClientLauncher::router_client(...)`.
//!
//! Path parameters use the typed `ClientPath<T>` extractor — there is no
//! thread-local router and no `with_router` helper.

use reinhardt::ClientPath;
use reinhardt::ClientRouter;
use reinhardt::pages::component::Page;
use reinhardt::pages::page;
use reinhardt::url_patterns;

use crate::client::pages::{
	choice_delete_page, choice_edit_page, choice_new_page, index_page, polls_detail_page,
	polls_results_page, question_delete_page, question_edit_page, question_new_page,
};
use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::polls, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("/", index_page)
		.route("/polls/new/", question_new_page)
		.route_path(
			"/polls/{question_id}/choices/new/",
			|ClientPath(question_id): ClientPath<i64>| choice_new_page(question_id),
		)
		.route_path(
			"/polls/choices/{choice_id}/edit/",
			|ClientPath(choice_id): ClientPath<i64>| choice_edit_page(choice_id),
		)
		.route_path(
			"/polls/choices/{choice_id}/delete/",
			|ClientPath(choice_id): ClientPath<i64>| choice_delete_page(choice_id),
		)
		.route_path(
			"/polls/{question_id}/",
			|ClientPath(question_id): ClientPath<i64>| polls_detail_page(question_id),
		)
		.route_path(
			"/polls/{question_id}/edit/",
			|ClientPath(question_id): ClientPath<i64>| question_edit_page(question_id),
		)
		.route_path(
			"/polls/{question_id}/delete/",
			|ClientPath(question_id): ClientPath<i64>| question_delete_page(question_id),
		)
		.route_path(
			"/polls/{question_id}/results/",
			|ClientPath(question_id): ClientPath<i64>| polls_results_page(question_id),
		)
		.not_found(|| error_page("Page not found"))
}

/// Error page used as the `not_found` fallback.
fn error_page(message: &str) -> Page {
	let message = message.to_string();
	page!(|message: String| {
		div {
			class: "container mt-5",
			div {
				class: "alert alert-danger",
				{ message }
			}
			a {
				href: "/",
				class: "btn btn-primary",
				"Back to Home"
			}
		}
	})(message)
}
