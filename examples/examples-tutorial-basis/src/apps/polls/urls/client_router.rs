//! Client-side routing for the polls SPA.
//!
//! The router applies the `polls` namespace to every named route.
//! Each route is registered with a stable name so that page components
//! can resolve URLs through the URL reverser instead of formatting
//! path strings inline.
use crate::client::pages::{
	choice_delete_page, choice_edit_page, choice_new_page, index_page, polls_detail_page,
	polls_results_page, question_delete_page, question_edit_page, question_new_page,
};
use reinhardt::ClientPath;
use reinhardt::ClientRouter;
use reinhardt::pages::component::Page;
use reinhardt::pages::page;

pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("index", "/", index_page)
		.route("question_new", "/polls/new/", question_new_page)
		.route_path(
			"choice_new",
			"/polls/{question_id}/choices/new/",
			|ClientPath(question_id): ClientPath<i64>| choice_new_page(question_id),
		)
		.route_path(
			"choice_edit",
			"/polls/{question_id}/choices/{choice_id}/edit/",
			|ClientPath(question_id): ClientPath<i64>, ClientPath(choice_id): ClientPath<i64>| {
				choice_edit_page(question_id, choice_id)
			},
		)
		.route_path(
			"choice_delete",
			"/polls/{question_id}/choices/{choice_id}/delete/",
			|ClientPath(question_id): ClientPath<i64>, ClientPath(choice_id): ClientPath<i64>| {
				choice_delete_page(question_id, choice_id)
			},
		)
		.route_path(
			"detail",
			"/polls/{question_id}/",
			|ClientPath(question_id): ClientPath<i64>| polls_detail_page(question_id),
		)
		.route_path(
			"question_edit",
			"/polls/{question_id}/edit/",
			|ClientPath(question_id): ClientPath<i64>| question_edit_page(question_id),
		)
		.route_path(
			"question_delete",
			"/polls/{question_id}/delete/",
			|ClientPath(question_id): ClientPath<i64>| question_delete_page(question_id),
		)
		.route_path(
			"results",
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
			class: "layout-page",
			div {
				class: "alert-danger mb-4",
				{ { message } }
			}
			a {
				href: "/",
				class: "btn-primary",
				"Back to Home"
			}
		}
	})(message)
}
