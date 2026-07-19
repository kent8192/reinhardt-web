#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::component::{Component, IntoPage, Page};
use reinhardt_pages::prelude::{
	ActionButton as PreludeActionButton, ActionResultPanel as PreludeActionResultPanel,
	ResourcePanel as PreludeResourcePanel,
};
use reinhardt_pages::reactive::{ReactiveScope, ResourceState, use_action, use_resource};
use reinhardt_pages::ssr::SsrRenderer;
use reinhardt_pages::ui::{ActionButton, ActionResultPanel, ResourcePanel};
use reinhardt_pages::{PageElement, deps};

fn assert_prelude_exports() {
	fn assert_component<T: Component>() {}

	assert_component::<PreludeActionButton<u32, String, u32>>();
	assert_component::<PreludeActionResultPanel<u32, String>>();
	assert_component::<PreludeResourcePanel<u32, String>>();
}

#[cfg(all(native, feature = "testing"))]
use std::cell::{Cell, RefCell};
#[cfg(all(native, feature = "testing"))]
use std::rc::Rc;

#[cfg(all(native, feature = "testing"))]
use reinhardt_pages::testing::component::{Role, render};

#[test]
fn action_button_renders_semantic_idle_button() {
	ReactiveScope::run(|| {
		let action = use_action(|_: u32| async { Ok::<u32, String>(1) });
		let button = ActionButton::new(action, 7_u32, Page::text("Save"))
			.attr("aria-label", "Save project")
			.attr("type", "submit");

		let html = button.render().render_to_string();

		assert_eq!(
			html,
			r#"<button type="button" aria-label="Save project">Save</button>"#
		);
	});
}

#[test]
fn action_button_forwards_custom_attrs_and_filters_managed_attrs() {
	ReactiveScope::run(|| {
		let action = use_action(|_: u32| async { Ok::<u32, String>(1) });
		let button = ActionButton::new(action, 7_u32, Page::text("Save"))
			.attr("class", "primary")
			.attr("data-kind", "save")
			.attr("type", "submit")
			.attr("disabled", "false")
			.attr("aria-busy", "false");

		assert_eq!(
			button.render().render_to_string(),
			r#"<button type="button" class="primary" data-kind="save">Save</button>"#
		);
	});
}

#[cfg(all(native, feature = "testing"))]
#[test]
fn action_button_dispatches_fixed_payload() {
	let dispatched = Rc::new(RefCell::new(Vec::new()));
	let dispatched_for_action = Rc::clone(&dispatched);
	let screen = render(move || {
		let action = use_action(move |payload: u32| {
			dispatched_for_action.borrow_mut().push(payload);
			async move { std::future::pending::<Result<u32, String>>().await }
		});
		ActionButton::new(action, 7_u32, Page::text("Save")).render()
	});

	screen.get_by_role(Role::Button, "Save").click();

	assert_eq!(&*dispatched.borrow(), &[7]);
}

#[cfg(all(native, feature = "testing"))]
#[test]
fn action_button_builds_payload_at_dispatch_time_and_guards_pending_clicks() {
	let factory_calls = Rc::new(Cell::new(0));
	let dispatched = Rc::new(RefCell::new(Vec::new()));
	let factory_calls_for_render = Rc::clone(&factory_calls);
	let dispatched_for_action = Rc::clone(&dispatched);
	let screen = render(move || {
		let action = use_action(move |payload: u32| {
			dispatched_for_action.borrow_mut().push(payload);
			async move { std::future::pending::<Result<u32, String>>().await }
		});
		ActionButton::new_with(
			action,
			move || {
				factory_calls_for_render.set(factory_calls_for_render.get() + 1);
				9_u32
			},
			Page::text("Save"),
		)
		.render()
	});

	let button = screen.get_by_role(Role::Button, "Save");
	button.click();
	button.click();

	assert_eq!(factory_calls.get(), 1);
	assert_eq!(&*dispatched.borrow(), &[9]);
}

#[test]
fn action_result_panel_renders_idle_content_and_empty_missing_slots() {
	ReactiveScope::run(|| {
		let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });
		let panel = ActionResultPanel::new(action).idle(|| Page::text("ready"));

		assert_eq!(panel.render().render_to_string(), "ready");
		assert_eq!(
			ActionResultPanel::new(action).render().render_to_string(),
			""
		);
	});
}

#[test]
fn action_result_panel_replaces_slots_and_converts_into_page() {
	ReactiveScope::run(|| {
		let action = use_action(|_: ()| async { Ok::<String, String>("done".to_string()) });
		let page = ActionResultPanel::new(action)
			.idle(|| Page::text("first"))
			.idle(|| Page::text("second"))
			.into_page();

		assert_eq!(page.render_to_string(), "second");
	});
}

#[test]
fn resource_panel_renders_each_resource_state_without_a_wrapper() {
	ReactiveScope::run(|| {
		let resource = use_resource(
			|| async { Ok::<Vec<String>, String>(Vec::new()) },
			reinhardt_pages::deps![],
		);
		let panel = ResourcePanel::new(resource)
			.loading(|| Page::text("loading"))
			.empty_if(Vec::is_empty)
			.empty(|_| Page::text("empty"))
			.success(|items| Page::text(format!("success:{}", items.len())))
			.error(|error| Page::text(format!("error:{error}")));
		let page = panel.render();

		resource.set(ResourceState::Loading);
		assert_eq!(page.render_to_string(), "loading");
		resource.set(ResourceState::Success(Vec::new()));
		assert_eq!(page.render_to_string(), "empty");
		resource.set(ResourceState::Success(vec!["one".to_string()]));
		assert_eq!(page.render_to_string(), "success:1");
		resource.set(ResourceState::Error("failed".to_string()));
		assert_eq!(page.render_to_string(), "error:failed");
	});
}

#[test]
fn resource_panel_without_empty_slot_uses_success_for_empty_values() {
	ReactiveScope::run(|| {
		let resource = use_resource(
			|| async { Ok::<Vec<String>, String>(Vec::new()) },
			reinhardt_pages::deps![],
		);
		let page = ResourcePanel::new(resource)
			.empty(|_| Page::text("unused"))
			.success(|items| Page::text(format!("success:{}", items.len())))
			.render();

		resource.set(ResourceState::Success(Vec::new()));
		assert_eq!(page.render_to_string(), "success:0");
	});
}

#[test]
fn resource_panel_from_latest_uses_resource_fallback() {
	ReactiveScope::run(|| {
		let resource = use_resource(|| async { Ok::<String, String>(String::new()) }, deps![]);
		let action = use_action(|_: ()| async { Ok::<String, String>("action".to_string()) });
		let page = ResourcePanel::from_latest(resource.latest_after(action))
			.success(|value| Page::text(value.clone()))
			.render();

		resource.set(ResourceState::Success("resource".to_string()));
		assert_eq!(page.render_to_string(), "resource");
	});
}

struct ResourcePanelSsrView;

impl Component for ResourcePanelSsrView {
	fn render(&self) -> Page {
		let resource = use_resource(
			|| async { Ok::<_, String>("server-value".to_string()) },
			deps![],
		);
		ResourcePanel::new(resource)
			.loading(|| Page::text("loading"))
			.success(|value| PageElement::new("strong").child(value).into_page())
			.error(|error| Page::text(error.clone()))
			.render()
	}

	fn name() -> &'static str {
		"ResourcePanelSsrView"
	}
}

#[tokio::test]
async fn resource_panel_uses_existing_ssr_resource_resolution() {
	assert_prelude_exports();

	let mut renderer = SsrRenderer::new();
	let html = renderer.render_page_to_string(&ResourcePanelSsrView).await;

	assert_eq!(html.matches("<strong>server-value</strong>").count(), 1);
	assert_eq!(html.matches("loading").count(), 0);
}
