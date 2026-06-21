//! URL configuration for the polls application.
//!
//! This module is intentionally target-neutral. Native builds aggregate the
//! server-function routes from `server::urls`, while both native and WASM
//! builds expose the client route table and reverse helpers.

use reinhardt::{ClientPath, ClientRouter, ServerRouter};

use super::pages;

/// Server-side app router.
pub fn server_url_patterns() -> ServerRouter {
	#[cfg(server)]
	{
		super::server::urls::server_url_patterns()
	}
	#[cfg(not(server))]
	{
		ServerRouter::new()
	}
}

/// Client-side routing for the polls SPA.
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("index", "/", pages::index_page)
		.route("question_new", "/polls/new/", pages::question_new_page)
		.route_path(
			"choice_new",
			"/polls/{question_id}/choices/new/",
			|ClientPath(question_id): ClientPath<i64>| pages::choice_new_page(question_id),
		)
		.route_path(
			"choice_edit",
			"/polls/{question_id}/choices/{choice_id}/edit/",
			|ClientPath(question_id): ClientPath<i64>, ClientPath(choice_id): ClientPath<i64>| {
				pages::choice_edit_page(question_id, choice_id)
			},
		)
		.route_path(
			"choice_delete",
			"/polls/{question_id}/choices/{choice_id}/delete/",
			|ClientPath(question_id): ClientPath<i64>, ClientPath(choice_id): ClientPath<i64>| {
				pages::choice_delete_page(question_id, choice_id)
			},
		)
		.route_path(
			"detail",
			"/polls/{question_id}/",
			|ClientPath(question_id): ClientPath<i64>| pages::polls_detail_page(question_id),
		)
		.route_path(
			"question_edit",
			"/polls/{question_id}/edit/",
			|ClientPath(question_id): ClientPath<i64>| pages::question_edit_page(question_id),
		)
		.route_path(
			"question_delete",
			"/polls/{question_id}/delete/",
			|ClientPath(question_id): ClientPath<i64>| pages::question_delete_page(question_id),
		)
		.route_path(
			"results",
			"/polls/{question_id}/results/",
			|ClientPath(question_id): ClientPath<i64>| pages::polls_results_page(question_id),
		)
		.not_found(|| pages::error_page("Page not found"))
}

/// Reverse a named polls client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_url_patterns()
		.reverse(name, params)
		.unwrap_or_else(|error| panic!("failed to reverse polls client route `{name}`: {error}"))
}
