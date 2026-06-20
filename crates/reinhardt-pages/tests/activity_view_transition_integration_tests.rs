#![cfg(not(target_arch = "wasm32"))]
//! Activity and ViewTransition boundary integration tests for SSR/native.

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_pages::component::{
	ActivityBoundary, ActivityMode, IntoPage, Page, PageElement, ViewTransitionBoundary,
	ViewTransitionStatus, start_view_transition,
};

#[test]
fn activity_visible_renders_content_without_hidden_attributes() {
	let html = ActivityBoundary::visible()
		.content(|| {
			PageElement::new("main")
				.child("Visible content")
				.into_page()
		})
		.render()
		.render_to_string();

	assert!(html.contains("data-rh-activity=\"visible\""));
	assert!(html.contains("data-rh-state-preserved=\"true\""));
	assert!(html.contains("Visible content"));
	assert!(!html.contains("hidden=\"hidden\""));
	assert!(!html.contains("aria-hidden=\"true\""));
}

#[test]
fn activity_hidden_preserves_subtree_while_hidden() {
	let html = ActivityBoundary::hidden()
		.content(|| {
			PageElement::new("section")
				.child("Preserved content")
				.into_page()
		})
		.render()
		.render_to_string();

	assert!(html.contains("data-rh-activity=\"hidden\""));
	assert!(html.contains("data-rh-state-preserved=\"true\""));
	assert!(html.contains("hidden=\"hidden\""));
	assert!(html.contains("aria-hidden=\"true\""));
	assert!(html.contains("Preserved content"));
}

#[test]
fn activity_mode_from_visible_controls_rendered_state() {
	let hidden = ActivityBoundary::default()
		.visible_when(false)
		.content(|| Page::text("still here"));

	assert_eq!(hidden.activity_mode(), ActivityMode::Hidden);
	assert!(hidden.render().render_to_string().contains("still here"));
}

#[test]
fn view_transition_boundary_renders_stable_marker_without_browser_api() {
	let html = ViewTransitionBoundary::new()
		.content(|| {
			PageElement::new("article")
				.child("Transition content")
				.into_page()
		})
		.render()
		.render_to_string();

	assert!(html.contains("data-rh-view-transition=\"boundary\""));
	assert!(html.contains("Transition content"));
	assert!(!html.contains("view-transition-name"));
}

#[test]
fn view_transition_boundary_renders_name_style() {
	let html = ViewTransitionBoundary::new()
		.name("hero-card")
		.content(|| Page::text("Hero"))
		.render()
		.render_to_string();

	assert!(html.contains("data-rh-view-transition=\"boundary\""));
	assert!(html.contains("data-rh-view-transition-name=\"hero-card\""));
	assert!(html.contains("style=\"view-transition-name: hero-card;\""));
	assert!(html.contains("Hero"));
}

#[test]
fn view_transition_boundary_sanitizes_name_before_inline_style() {
	let html = ViewTransitionBoundary::new()
		.name("123; color: red")
		.content(|| Page::text("Hero"))
		.render()
		.render_to_string();

	assert!(html.contains("data-rh-view-transition-name=\"rh-vt-123__color__red\""));
	assert!(html.contains("style=\"view-transition-name: rh-vt-123__color__red;\""));
	assert!(!html.contains("color: red"));
	assert!(!html.contains("123;"));
}

#[test]
fn native_start_view_transition_runs_update_and_reports_unsupported() {
	let ran = Rc::new(RefCell::new(false));
	let handle = start_view_transition({
		let ran = Rc::clone(&ran);
		move || {
			*ran.borrow_mut() = true;
		}
	});

	assert!(*ran.borrow());
	assert_eq!(handle.status(), &ViewTransitionStatus::Unsupported);
	assert!(handle.is_unsupported());
	assert!(!handle.is_started());
}
