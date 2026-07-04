#![cfg(not(target_arch = "wasm32"))]

use reinhardt_pages::component::{Component, IntoPage, Page, PageElement};
use reinhardt_pages::reactive::{ResourceState, use_resource, use_resource_with_key};
use reinhardt_pages::ssr::{SsrOptions, SsrRenderer};
use std::time::Duration;

fn resource_view() -> Page {
	Page::reactive(|| {
		let resource = use_resource(|| async { Ok::<_, String>("server-value".to_string()) }, ());

		match resource.get() {
			ResourceState::Success(value) => PageElement::new("div").child(value).into_page(),
			ResourceState::Loading => PageElement::new("div").child("loading").into_page(),
			ResourceState::Error(error) => PageElement::new("div").child(error).into_page(),
		}
	})
}

struct ResourceComponent;

impl Component for ResourceComponent {
	fn render(&self) -> Page {
		let resource = use_resource(
			|| async { Ok::<_, String>("component-server-value".to_string()) },
			(),
		);

		match resource.get() {
			ResourceState::Success(value) => PageElement::new("section").child(value).into_page(),
			ResourceState::Loading => PageElement::new("section").child("loading").into_page(),
			ResourceState::Error(error) => PageElement::new("section").child(error).into_page(),
		}
	}

	fn name() -> &'static str {
		"ResourceComponent"
	}
}

#[tokio::test]
async fn ssr_resolved_resource_serializes_state() {
	let mut renderer = SsrRenderer::new();
	let html = renderer
		.render_page_with_view_head_to_string(resource_view())
		.await;

	assert!(html.contains(r#""resources""#));
	assert!(html.contains("server-value"));
	assert!(!html.contains(">loading<"));
}

#[tokio::test]
async fn ssr_replays_component_render_for_top_level_resource() {
	let component = ResourceComponent;
	let mut renderer = SsrRenderer::new();
	let html = renderer.render_page_to_string(&component).await;

	assert!(html.contains("component-server-value"));
	assert!(!html.contains(">loading<"));
}

#[tokio::test]
async fn ssr_resource_error_serializes_state() {
	let view = Page::reactive(|| {
		let resource = use_resource(
			|| async { Err::<String, _>("server-error".to_string()) },
			(),
		);

		match resource.get() {
			ResourceState::Success(value) => PageElement::new("div").child(value).into_page(),
			ResourceState::Loading => PageElement::new("div").child("loading").into_page(),
			ResourceState::Error(error) => PageElement::new("div").child(error).into_page(),
		}
	});

	let mut renderer = SsrRenderer::new();
	let html = renderer.render_page_with_view_head_to_string(view).await;

	assert!(html.contains("server-error"));
	assert!(!html.contains(">loading<"));
}

#[tokio::test]
async fn ssr_resource_timeout_leaves_loading_unserialized() {
	let view = Page::reactive(|| {
		let resource = use_resource(
			|| async {
				tokio::time::sleep(Duration::from_secs(60)).await;
				Ok::<_, String>("delayed".to_string())
			},
			(),
		);

		match resource.get() {
			ResourceState::Success(value) => PageElement::new("div").child(value).into_page(),
			ResourceState::Loading => PageElement::new("div").child("loading").into_page(),
			ResourceState::Error(error) => PageElement::new("div").child(error).into_page(),
		}
	});

	let mut renderer =
		SsrRenderer::with_options(SsrOptions::new().resource_timeout(Duration::from_millis(1)));
	let html = renderer.render_page_with_view_head_to_string(view).await;

	assert!(html.contains(">loading<"));
	assert!(!html.contains("delayed"));
	assert!(!html.contains(r#""rh-res-0""#));
}

#[tokio::test]
async fn explicit_resource_key_is_serialized() {
	let view = Page::reactive(|| {
		let resource = use_resource_with_key(
			"polls.detail.42",
			|| async { Ok::<_, String>("question".to_string()) },
			(),
		);

		match resource.get() {
			ResourceState::Success(value) => PageElement::new("div").child(value).into_page(),
			ResourceState::Loading => PageElement::new("div").child("loading").into_page(),
			ResourceState::Error(error) => PageElement::new("div").child(error).into_page(),
		}
	});

	let mut renderer = SsrRenderer::new();
	let html = renderer.render_page_with_view_head_to_string(view).await;

	assert!(html.contains("polls.detail.42"));
	assert!(html.contains("question"));
}
