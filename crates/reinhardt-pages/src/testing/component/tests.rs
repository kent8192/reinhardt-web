use std::cell::Cell;
use std::rc::Rc;

use reinhardt_core::types::page::{Page, PageElement};

use super::{EventError, QueryError, Role, render};

#[test]
fn renders_page_tree_and_pretty_output() {
	let screen = render(
		PageElement::new("section")
			.attr("id", "hero")
			.child(PageElement::new("h1").child("Hello"))
			.child(Page::fragment([Page::text("Intro")])),
	);

	assert_eq!(
		screen.pretty(),
		"<section id=\"hero\">\n  <h1>\n    Hello\n  </h1>\n  Intro\n</section>\n"
	);
}

#[test]
fn pretty_text_only_screen_has_trailing_newline() {
	let screen = render(Page::text("Hello"));

	assert_eq!(screen.pretty(), "Hello\n");
}

#[test]
fn queries_by_text_role_label_and_placeholder() {
	let screen = render(
		PageElement::new("form")
			.child(
				PageElement::new("label")
					.attr("for", "email")
					.child("Email"),
			)
			.child(
				PageElement::new("input")
					.attr("id", "email")
					.attr("placeholder", "name@example.com"),
			)
			.child(PageElement::new("button").child("Submit")),
	);

	assert_eq!(screen.get_by_text("Submit").tag_name(), "button");
	assert_eq!(screen.get_by_role(Role::Button, "Submit").text(), "Submit");
	assert_eq!(screen.get_by_label("Email").tag_name(), "input");
	assert_eq!(
		screen.get_by_placeholder("name@example.com").tag_name(),
		"input"
	);
}

#[test]
fn hidden_elements_are_excluded_from_queries() {
	let screen = render(
		PageElement::new("div")
			.child(
				PageElement::new("button")
					.attr("aria-hidden", "true")
					.child("Save"),
			)
			.child(PageElement::new("button").child("Save")),
	);

	assert_eq!(screen.get_by_text("Save").tag_name(), "button");
}

#[test]
fn multiple_matches_return_query_error() {
	let screen = render(
		PageElement::new("div")
			.child(PageElement::new("button").child("Save"))
			.child(PageElement::new("button").child("Save")),
	);

	assert!(matches!(
		screen.try_get_by_text("Save"),
		Err(QueryError::MultipleMatches)
	));
}

#[test]
fn click_dispatches_native_dummy_event() {
	let clicked = Rc::new(Cell::new(false));
	let clicked_for_handler = clicked.clone();
	let screen = render(
		PageElement::new("button")
			.listener("click", move |_| clicked_for_handler.set(true))
			.child("Save"),
	);

	screen.get_by_role(Role::Button, "Save").click();

	assert!(clicked.get());
}

#[test]
fn input_updates_internal_value_before_dispatch() {
	let called = Rc::new(Cell::new(false));
	let called_for_handler = called.clone();
	let screen = render(
		PageElement::new("input")
			.attr("value", "old")
			.attr("placeholder", "Job name")
			.listener("input", move |_| called_for_handler.set(true)),
	);
	let input = screen.get_by_placeholder("Job name");

	input.input("new");

	assert!(called.get());
	assert_eq!(input.value().as_deref(), Some("new"));
}

#[test]
fn missing_event_handler_returns_error() {
	let screen = render(PageElement::new("button").child("Save"));

	assert_eq!(
		screen.get_by_role(Role::Button, "Save").try_click(),
		Err(EventError::MissingHandler)
	);
}
