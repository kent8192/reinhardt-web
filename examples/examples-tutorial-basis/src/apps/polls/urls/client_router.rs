//! Client-side routing for the polls SPA.

use crate::apps::polls::client::components;
use reinhardt::ClientRouter;

/// Client-side routing for the polls SPA.
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.component(components::polls_index::polls_index)
		.component(components::question_new::question_new)
		.component(components::choice_new::choice_new)
		.component(components::choice_edit::choice_edit)
		.component(components::choice_delete::choice_delete)
		.component(components::polls_detail::polls_detail)
		.component(components::question_edit::question_edit)
		.component(components::question_delete::question_delete)
		.component(components::polls_results::polls_results)
		.not_found(|| components::error_page("Page not found"))
}

/// Reverse a named polls client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_url_patterns()
		.reverse(name, params)
		.unwrap_or_else(|error| panic!("failed to reverse polls client route `{name}`: {error}"))
}
