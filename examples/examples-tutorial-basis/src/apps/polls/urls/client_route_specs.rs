//! Native-side client route metadata for route aggregation and reversing.

use reinhardt::ClientRouter;
use reinhardt::pages::component::Page;

/// Client route names and paths without WASM component bodies.
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("index", "/", Page::empty)
		.route("question_new", "/polls/new/", Page::empty)
		.route(
			"choice_new",
			"/polls/{question_id}/choices/new/",
			Page::empty,
		)
		.route(
			"choice_edit",
			"/polls/{question_id}/choices/{choice_id}/edit/",
			Page::empty,
		)
		.route(
			"choice_delete",
			"/polls/{question_id}/choices/{choice_id}/delete/",
			Page::empty,
		)
		.route("detail", "/polls/{question_id}/", Page::empty)
		.route("question_edit", "/polls/{question_id}/edit/", Page::empty)
		.route(
			"question_delete",
			"/polls/{question_id}/delete/",
			Page::empty,
		)
		.route("results", "/polls/{question_id}/results/", Page::empty)
		.not_found(Page::empty)
}

/// Reverse a named polls client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_url_patterns()
		.reverse(name, params)
		.unwrap_or_else(|error| panic!("failed to reverse polls client route `{name}`: {error}"))
}
