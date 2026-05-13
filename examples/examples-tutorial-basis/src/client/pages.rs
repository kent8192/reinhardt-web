//! Page components
//!
//! This module re-exports page-level components for the polling application.
//! Each page function returns a View that can be rendered.

use reinhardt::pages::component::Page;

/// Index page - List all polls
pub fn index_page() -> Page {
	crate::client::components::polls::polls_index()
}

/// Poll detail page - Show question and voting form
pub fn polls_detail_page(question_id: i64) -> Page {
	crate::client::components::polls::polls_detail(question_id)
}

/// Poll results page - Show voting results
pub fn polls_results_page(question_id: i64) -> Page {
	crate::client::components::polls::polls_results(question_id)
}

/// New question page - Create a new poll question
pub fn question_new_page() -> Page {
	crate::client::components::polls::question_new()
}

/// Edit question page - Update an existing poll question (author only)
pub fn question_edit_page(question_id: i64) -> Page {
	crate::client::components::polls::question_edit(question_id)
}

/// Delete question confirmation page - Author-only deletion
pub fn question_delete_page(question_id: i64) -> Page {
	crate::client::components::polls::question_delete_confirm(question_id)
}

/// New choice page - Add a choice to an existing question (Phase 3)
pub fn choice_new_page(question_id: i64) -> Page {
	crate::client::components::polls::choice_new(question_id)
}

/// Edit choice page - Update a choice's text (Phase 3)
pub fn choice_edit_page(choice_id: i64) -> Page {
	crate::client::components::polls::choice_edit(choice_id)
}

/// Delete choice confirmation page - Confirm before deletion (Phase 3)
pub fn choice_delete_page(choice_id: i64) -> Page {
	crate::client::components::polls::choice_delete_confirm(choice_id)
}

/// Login page - Username + password form
pub fn login_page() -> Page {
	crate::client::components::users::login_form()
}

/// Logout page - Single-button session termination
pub fn logout_page() -> Page {
	crate::client::components::users::logout_form()
}
