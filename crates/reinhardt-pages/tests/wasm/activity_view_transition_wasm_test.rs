//! Activity and ViewTransition WASM integration tests.
//!
//! **Run with**: `wasm-pack test --headless --chrome`

#![cfg(wasm)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use reinhardt_core::types::page::EventType;
use reinhardt_pages::component::{
	ActivityBoundary, IntoPage, Page, PageElement, PageExt, ViewTransitionBoundary,
	cleanup_reactive_nodes, start_view_transition,
};
use reinhardt_pages::deps;
use reinhardt_pages::dom::Element;
use reinhardt_pages::reactive::{ReactiveScope, Signal, hooks::use_retained_layout_effect};
use wasm_bindgen::JsCast;
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
fn reactive_activity_mode_updates_wrapper_without_recreating_content() {
	cleanup_reactive_nodes();
	ReactiveScope::run(|| {
		let document = web_sys::window().unwrap().document().unwrap();
		if let Some(prev) = document.get_element_by_id("activity-root") {
			prev.remove();
		}

		let target = document.create_element("div").unwrap();
		target.set_id("activity-root");
		document.body().unwrap().append_child(&target).unwrap();

		let visible = Signal::new(true);
		let visible_for_view = visible.clone();
		let handler_calls = Rc::new(Cell::new(0));
		let handler_calls_for_view = Rc::clone(&handler_calls);
		Page::reactive(move || {
			let handler_calls = Rc::clone(&handler_calls_for_view);
			ActivityBoundary::default()
				.visible_when(visible_for_view.get())
				.content({
					let handler_calls = Rc::clone(&handler_calls);
					move || {
						let handler_calls = Rc::clone(&handler_calls);
						PageElement::new("input")
							.attr("id", "activity-owned-input")
							.attr("value", "initial")
							.on(
								EventType::Input,
								Arc::new(move |_| {
									let signal = Signal::new(1_i32);
									assert_eq!(signal.get(), 1);
									handler_calls.set(handler_calls.get() + 1);
								}),
							)
							.into_page()
					}
				})
				.into_page()
		})
		.mount(&Element::new(target.clone()))
		.expect("activity mounts");

		let wrapper = target
			.query_selector("[data-rh-activity]")
			.unwrap()
			.expect("activity wrapper");
		let input = document
			.get_element_by_id("activity-owned-input")
			.unwrap()
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		input.set_value("user typed");

		visible.set(false);
		assert_eq!(
			wrapper.get_attribute("data-rh-activity").as_deref(),
			Some("hidden")
		);
		assert_eq!(wrapper.get_attribute("hidden").as_deref(), Some("hidden"));
		input
			.dispatch_event(&web_sys::Event::new("input").unwrap())
			.unwrap();
		assert_eq!(handler_calls.get(), 1);
		assert_eq!(
			document
				.get_element_by_id("activity-owned-input")
				.unwrap()
				.dyn_into::<web_sys::HtmlInputElement>()
				.unwrap()
				.value(),
			"user typed"
		);

		visible.set(true);
		assert_eq!(
			wrapper.get_attribute("data-rh-activity").as_deref(),
			Some("visible")
		);
		assert_eq!(wrapper.get_attribute("hidden"), None);
		assert_eq!(wrapper.get_attribute("aria-hidden"), None);
		assert_eq!(
			document
				.get_element_by_id("activity-owned-input")
				.unwrap()
				.dyn_into::<web_sys::HtmlInputElement>()
				.unwrap()
				.value(),
			"user typed"
		);

		cleanup_reactive_nodes();
		target.remove();
	});
}

#[wasm_bindgen_test]
fn reactive_activity_mode_retains_the_original_render_store() {
	cleanup_reactive_nodes();
	ReactiveScope::run(|| {
		let document = web_sys::window().unwrap().document().unwrap();
		let target = document.create_element("div").unwrap();
		document.body().unwrap().append_child(&target).unwrap();
		let visible = Signal::new(true);
		let visible_for_view = visible.clone();
		let renders = Rc::new(Cell::new(0));
		let renders_for_view = Rc::clone(&renders);
		let initial_owner = Rc::new(RefCell::new(None::<std::rc::Weak<()>>));
		let initial_owner_for_view = Rc::clone(&initial_owner);

		Page::reactive(move || {
			let render_index = renders_for_view.get();
			renders_for_view.set(render_index + 1);
			let initial_owner = Rc::clone(&initial_owner_for_view);
			ActivityBoundary::default()
				.visible_when(visible_for_view.get())
				.content(move || {
					let owner = Rc::new(());
					if render_index == 0 {
						*initial_owner.borrow_mut() = Some(Rc::downgrade(&owner));
					}
					let retained_owner = Rc::clone(&owner);
					use_retained_layout_effect(
						move || {
							let _owner = Rc::clone(&retained_owner);
							None::<fn()>
						},
						deps![],
					);
					PageElement::new("input").into_page()
				})
				.into_page()
		})
		.mount(&Element::new(target.clone()))
		.expect("activity mounts");
		let owner = initial_owner
			.borrow()
			.clone()
			.expect("initial render owner");

		assert!(owner.upgrade().is_some());
		visible.set(false);
		assert!(owner.upgrade().is_some());

		cleanup_reactive_nodes();
		target.remove();
	});
}

#[wasm_bindgen_test]
fn reactive_activity_mode_keeps_nested_reactive_content_live() {
	cleanup_reactive_nodes();
	ReactiveScope::run(|| {
		let document = web_sys::window().unwrap().document().unwrap();
		let target = document.create_element("div").unwrap();
		document.body().unwrap().append_child(&target).unwrap();
		let visible = Signal::new(true);
		let message = Signal::new("initial".to_owned());
		let visible_for_view = visible.clone();
		let message_for_view = message.clone();

		Page::reactive(move || {
			let message_for_content = message_for_view.clone();
			ActivityBoundary::default()
				.visible_when(visible_for_view.get())
				.content(move || {
					let message_for_reactive = message_for_content.clone();
					Page::reactive(move || Page::text(message_for_reactive.get()))
				})
				.into_page()
		})
		.mount(&Element::new(target.clone()))
		.expect("activity mounts");

		visible.set(false);
		message.set("updated".to_owned());
		assert_eq!(target.text_content().as_deref(), Some("updated"));

		cleanup_reactive_nodes();
		target.remove();
	});
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
