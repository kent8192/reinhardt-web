use std::cell::Cell;
use std::rc::Rc;

use reinhardt_core::page::IntoPage;
use reinhardt_core::reactive::Signal;
use reinhardt_core::types::page::{DeferredNode, Page, PageElement, SuspenseNode};

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

#[tokio::test]
async fn optional_and_async_queries_cover_role_label_and_placeholder() {
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

	assert_eq!(
		screen
			.query_by_role(Role::Button, "Submit")
			.expect("button should be found")
			.tag_name(),
		"button"
	);
	assert!(screen.query_by_role(Role::Button, "Missing").is_none());
	assert_eq!(
		screen
			.query_by_label("Email")
			.expect("label should be found")
			.tag_name(),
		"input"
	);
	assert!(screen.query_by_label("Missing").is_none());
	assert_eq!(
		screen
			.query_by_placeholder("name@example.com")
			.expect("placeholder should be found")
			.tag_name(),
		"input"
	);
	assert!(screen.query_by_placeholder("Missing").is_none());
	assert_eq!(screen.find_by_label("Email").await.tag_name(), "input");
	assert_eq!(
		screen
			.try_find_by_placeholder("name@example.com")
			.await
			.unwrap()
			.tag_name(),
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
fn suspense_nodes_render_the_active_branch() {
	let pending_screen = render(Page::Suspense(SuspenseNode::new(
		Some("pending-boundary".to_string()),
		|| true,
		|| PageElement::new("p").child("Loading").into_page(),
		|| PageElement::new("p").child("Ready").into_page(),
	)));
	let resolved_screen = render(Page::Suspense(SuspenseNode::new(
		Some("resolved-boundary".to_string()),
		|| false,
		|| PageElement::new("p").child("Loading").into_page(),
		|| PageElement::new("p").child("Ready").into_page(),
	)));

	assert_eq!(pending_screen.get_by_text("Loading").tag_name(), "p");
	assert!(pending_screen.query_by_text("Ready").is_none());
	assert_eq!(resolved_screen.get_by_text("Ready").tag_name(), "p");
	assert!(resolved_screen.query_by_text("Loading").is_none());
}

#[test]
fn deferred_nodes_render_content_branch() {
	let screen = render(Page::Deferred(DeferredNode::new(
		"deferred-content",
		|| PageElement::new("p").child("Deferred fallback").into_page(),
		|| PageElement::new("p").child("Deferred content").into_page(),
	)));

	assert_eq!(screen.get_by_text("Deferred content").tag_name(), "p");
	assert!(screen.query_by_text("Deferred fallback").is_none());
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

#[tokio::test]
async fn detached_element_reads_return_error() {
	let show = Signal::new(true);
	let show_for_render = show.clone();
	let screen = render(Page::reactive(move || {
		if show_for_render.get() {
			PageElement::new("button").child("Save").into_page()
		} else {
			Page::Empty
		}
	}));
	let button = screen.get_by_role(Role::Button, "Save");

	show.set(false);
	screen.settle().await;

	assert_eq!(button.try_text(), Err(EventError::DetachedElement));
	assert_eq!(button.try_tag_name(), Err(EventError::DetachedElement));
	assert_eq!(button.try_value(), Err(EventError::DetachedElement));
}

#[test]
fn missing_event_handler_returns_error() {
	let screen = render(PageElement::new("button").child("Save"));

	assert_eq!(
		screen.get_by_role(Role::Button, "Save").try_click(),
		Err(EventError::MissingHandler)
	);
}
