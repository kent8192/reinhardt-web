//! Client-side routing for the polls SPA.
//!
//! Routes are declared with `#[url_patterns(InstalledApp::polls, mode = client)]`,
//! which auto-registers the router via inventory and applies the `polls`
//! namespace to every named route (so the route name passed to `named_route`
//! is reachable as `polls:<name>` through `ResolvedUrls`). Each route is
//! registered with a stable name so that page components can resolve URLs
//! through `ResolvedUrls::resolve_client_url(...)` instead of formatting
//! path strings inline.

use reinhardt::ClientPath;
use reinhardt::ClientRouter;
use reinhardt::pages::component::Page;
use reinhardt::pages::page;
use reinhardt::url_patterns;

use crate::client::links;
use crate::client::pages::{
	choice_delete_page, choice_edit_page, choice_new_page, index_page, polls_detail_page,
	polls_results_page, question_delete_page, question_edit_page, question_new_page,
};
use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::polls, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.named_route("index", "/", index_page)
		.named_route("question_new", "/polls/new/", question_new_page)
		.named_route_path(
			"choice_new",
			"/polls/{question_id}/choices/new/",
			|ClientPath(question_id): ClientPath<i64>| choice_new_page(question_id),
		)
		.named_route_path2(
			"choice_edit",
			"/polls/{question_id}/choices/{choice_id}/edit/",
			|ClientPath(question_id): ClientPath<i64>, ClientPath(choice_id): ClientPath<i64>| {
				choice_edit_page(question_id, choice_id)
			},
		)
		.named_route_path2(
			"choice_delete",
			"/polls/{question_id}/choices/{choice_id}/delete/",
			|ClientPath(question_id): ClientPath<i64>, ClientPath(choice_id): ClientPath<i64>| {
				choice_delete_page(question_id, choice_id)
			},
		)
		.named_route_path(
			"detail",
			"/polls/{question_id}/",
			|ClientPath(question_id): ClientPath<i64>| polls_detail_page(question_id),
		)
		.named_route_path(
			"question_edit",
			"/polls/{question_id}/edit/",
			|ClientPath(question_id): ClientPath<i64>| question_edit_page(question_id),
		)
		.named_route_path(
			"question_delete",
			"/polls/{question_id}/delete/",
			|ClientPath(question_id): ClientPath<i64>| question_delete_page(question_id),
		)
		.named_route_path(
			"results",
			"/polls/{question_id}/results/",
			|ClientPath(question_id): ClientPath<i64>| polls_results_page(question_id),
		)
		.not_found(|| error_page("Page not found"))
}

/// Error page used as the `not_found` fallback.
fn error_page(message: &str) -> Page {
	let message = message.to_string();
	let home_href = links::polls_index();
	page!(|message: String, home_href: String| {
		div {
			class: "layout-page",
			div {
				class: "alert-danger mb-4",
				{ message }
			}
			a {
				href: home_href,
				class: "btn-primary",
				"Back to Home"
			}
		}
	})(message, home_href)
}
