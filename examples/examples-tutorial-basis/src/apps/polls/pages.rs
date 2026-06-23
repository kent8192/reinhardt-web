//! Target-neutral page entry points for the polls application.
//!
//! Native builds aggregate these functions for route metadata, while WASM
//! builds call into the polls client components and wrap them in the shared
//! site navigation.

use reinhardt::pages::component::Page;

#[cfg(client)]
use crate::client::components::nav::with_nav;

/// Index page - List all polls.
pub fn index_page() -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::polls_index())
	}
	#[cfg(not(client))]
	{
		Page::Empty
	}
}

/// Poll detail page - Show question and voting form.
pub fn polls_detail_page(question_id: i64) -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::polls_detail(
			question_id,
		))
	}
	#[cfg(not(client))]
	{
		let _ = question_id;
		Page::Empty
	}
}

/// Poll results page - Show voting results.
pub fn polls_results_page(question_id: i64) -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::polls_results(
			question_id,
		))
	}
	#[cfg(not(client))]
	{
		let _ = question_id;
		Page::Empty
	}
}

/// New question page - Create a new poll question.
pub fn question_new_page() -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::question_new())
	}
	#[cfg(not(client))]
	{
		Page::Empty
	}
}

/// Edit question page - Update an existing poll question.
pub fn question_edit_page(question_id: i64) -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::question_edit(
			question_id,
		))
	}
	#[cfg(not(client))]
	{
		let _ = question_id;
		Page::Empty
	}
}

/// Delete question confirmation page.
pub fn question_delete_page(question_id: i64) -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::question_delete_confirm(question_id))
	}
	#[cfg(not(client))]
	{
		let _ = question_id;
		Page::Empty
	}
}

/// New choice page - Add a choice to an existing question.
pub fn choice_new_page(question_id: i64) -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::choice_new(
			question_id,
		))
	}
	#[cfg(not(client))]
	{
		let _ = question_id;
		Page::Empty
	}
}

/// Edit choice page - Update a choice's text.
pub fn choice_edit_page(question_id: i64, choice_id: i64) -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::polls::client::components::choice_edit(
			question_id,
			choice_id,
		))
	}
	#[cfg(not(client))]
	{
		let _ = (question_id, choice_id);
		Page::Empty
	}
}

/// Delete choice confirmation page.
pub fn choice_delete_page(question_id: i64, choice_id: i64) -> Page {
	#[cfg(client)]
	{
		with_nav(
			crate::apps::polls::client::components::choice_delete_confirm(question_id, choice_id),
		)
	}
	#[cfg(not(client))]
	{
		let _ = (question_id, choice_id);
		Page::Empty
	}
}

/// Error page used as the `not_found` fallback.
pub fn error_page(message: &str) -> Page {
	#[cfg(client)]
	{
		let message = message.to_string();
		reinhardt::pages::page!(|message: String| {
			div {
				class: "layout-page",
				div {
					class: "alert-danger mb-4",
					{ message }
				}
				a {
					href: "/",
					class: "btn-primary",
					"Back to Home"
				}
			}
		})(message)
	}
	#[cfg(not(client))]
	{
		let _ = message;
		Page::Empty
	}
}
