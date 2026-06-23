//! Page wrappers for the polls application.
//!
//! The shared site navigation is composed at this layer so route handlers stay
//! focused on selecting the right polls component.

use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

/// Index page - List all polls.
pub fn index_page() -> Page {
	with_nav(super::components::polls_index())
}

/// Poll detail page - Show question and voting form.
pub fn polls_detail_page(question_id: i64) -> Page {
	with_nav(super::components::polls_detail(question_id))
}

/// Poll results page - Show voting results.
pub fn polls_results_page(question_id: i64) -> Page {
	with_nav(super::components::polls_results(question_id))
}

/// New question page - Create a new poll question.
pub fn question_new_page() -> Page {
	with_nav(super::components::question_new())
}

/// Edit question page - Update an existing poll question.
pub fn question_edit_page(question_id: i64) -> Page {
	with_nav(super::components::question_edit(question_id))
}

/// Delete question confirmation page.
pub fn question_delete_page(question_id: i64) -> Page {
	with_nav(super::components::question_delete_confirm(question_id))
}

/// New choice page - Add a choice to an existing question.
pub fn choice_new_page(question_id: i64) -> Page {
	with_nav(super::components::choice_new(question_id))
}

/// Edit choice page - Update a choice's text.
pub fn choice_edit_page(question_id: i64, choice_id: i64) -> Page {
	with_nav(super::components::choice_edit(question_id, choice_id))
}

/// Delete choice confirmation page.
pub fn choice_delete_page(question_id: i64, choice_id: i64) -> Page {
	with_nav(super::components::choice_delete_confirm(
		question_id,
		choice_id,
	))
}
