//! Client-side routing for the polls SPA.

use reinhardt::ClientRouter;
#[cfg(not(client))]
use reinhardt::pages::component::Page;

#[cfg(client)]
use crate::apps::polls::client::components;

/// Client-side routing for the polls SPA.
pub fn client_url_patterns() -> ClientRouter {
	#[cfg(client)]
	{
		return ClientRouter::new()
			.component(components::polls_index::polls_index)
			.component(components::question_new::question_new)
			.component(components::choice_new::choice_new)
			.component(components::choice_edit::choice_edit)
			.component(components::choice_delete::choice_delete)
			.component(components::polls_detail::polls_detail)
			.component(components::question_edit::question_edit)
			.component(components::question_delete::question_delete)
			.component(components::polls_results::polls_results)
			.not_found(|| components::error_page("Page not found"));
	}

	#[cfg(not(client))]
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
