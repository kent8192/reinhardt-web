//! Activity and ViewTransition WASM integration tests.
//!
//! **Run with**: `wasm-pack test --headless --chrome`

#![cfg(wasm)]

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_pages::component::{
	ActivityBoundary, IntoPage, Page, PageElement, ViewTransitionBoundary, start_view_transition,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn hidden_activity_boundary_keeps_content_in_rendered_markup() {
	let html = ActivityBoundary::hidden()
		.content(|| PageElement::new("button").child("Keep state").into_page())
		.render()
		.render_to_string();

	assert!(html.contains("data-rh-activity=\"hidden\""));
	assert!(html.contains("hidden=\"hidden\""));
	assert!(html.contains("Keep state"));
}

#[wasm_bindgen_test]
fn view_transition_boundary_marks_named_subtree() {
	let html = ViewTransitionBoundary::new()
		.name("panel")
		.content(|| Page::text("Panel"))
		.render()
		.render_to_string();

	assert!(html.contains("data-rh-view-transition=\"boundary\""));
	assert!(html.contains("data-rh-view-transition-name=\"panel\""));
	assert!(html.contains("view-transition-name: panel;"));
}

#[wasm_bindgen_test]
async fn start_view_transition_runs_update_on_wasm() {
	let ran = Rc::new(RefCell::new(false));
	let handle = start_view_transition({
		let ran = Rc::clone(&ran);
		move || {
			*ran.borrow_mut() = true;
		}
	});

	for _ in 0..5 {
		if *ran.borrow() {
			break;
		}
		gloo_timers::future::TimeoutFuture::new(20).await;
	}

	assert!(*ran.borrow());
	assert!(
		handle.is_started() || handle.is_unsupported() || handle.error().is_some(),
		"unexpected ViewTransition status: {:?}",
		handle.status()
	);

	if handle.is_started() {
		assert!(handle.transition().is_some());
		handle
			.skip_transition()
			.expect("view transition can be skipped");
	}
}
