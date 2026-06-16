//! Page components
//!
//! This module re-exports page-level components for the polling application.
//! Each page function returns a View that can be rendered.
//!
//! The shared site navigation (`nav_bar`) is composed at this layer so every
//! routed page receives the same header without each component reimplementing
//! it. Body components in `client::components::polls` / `::users` stay focused
//! on page-specific markup.

use reinhardt::pages::client_page;
use reinhardt::pages::component::Page;

#[cfg(client)]
use crate::client::components::nav::with_nav;

/// Index page - List all polls
#[client_page]
pub fn index_page() -> Page {
	with_nav(crate::apps::polls::client::components::polls_index())
}

/// Poll detail page - Show question and voting form
#[client_page]
pub fn polls_detail_page(question_id: i64) -> Page {
	with_nav(crate::apps::polls::client::components::polls_detail(
		question_id,
	))
}

/// Poll results page - Show voting results
#[client_page]
pub fn polls_results_page(question_id: i64) -> Page {
	with_nav(crate::apps::polls::client::components::polls_results(
		question_id,
	))
}

/// New question page - Create a new poll question
#[client_page]
pub fn question_new_page() -> Page {
	with_nav(crate::apps::polls::client::components::question_new())
}

/// Edit question page - Update an existing poll question (author only)
#[client_page]
pub fn question_edit_page(question_id: i64) -> Page {
	with_nav(crate::apps::polls::client::components::question_edit(
		question_id,
	))
}

/// Delete question confirmation page - Author-only deletion
#[client_page]
pub fn question_delete_page(question_id: i64) -> Page {
	with_nav(crate::apps::polls::client::components::question_delete_confirm(question_id))
}

/// New choice page - Add a choice to an existing question (Phase 3)
#[client_page]
pub fn choice_new_page(question_id: i64) -> Page {
	with_nav(crate::apps::polls::client::components::choice_new(
		question_id,
	))
}

/// Edit choice page - Update a choice's text (Phase 3)
#[client_page]
pub fn choice_edit_page(question_id: i64, choice_id: i64) -> Page {
	with_nav(crate::apps::polls::client::components::choice_edit(
		question_id,
		choice_id,
	))
}

/// Delete choice confirmation page - Confirm before deletion (Phase 3)
#[client_page]
pub fn choice_delete_page(question_id: i64, choice_id: i64) -> Page {
	with_nav(crate::apps::polls::client::components::choice_delete_confirm(question_id, choice_id))
}

/// Login page - Username + password form
#[client_page]
pub fn login_page() -> Page {
	with_nav(crate::apps::users::client::components::login_form())
}

/// Logout page - Single-button session termination
#[client_page]
pub fn logout_page() -> Page {
	with_nav(crate::apps::users::client::components::logout_form())
}

/// Sign-up page - Create a new account
#[client_page]
pub fn signup_page() -> Page {
	with_nav(crate::apps::users::client::components::signup_form())
}
