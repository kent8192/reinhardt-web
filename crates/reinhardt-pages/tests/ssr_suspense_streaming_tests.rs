#![cfg(not(target_arch = "wasm32"))]

use futures_util::StreamExt;
use reinhardt_pages::component::suspense::SuspenseBoundary;
use reinhardt_pages::component::{IntoPage, Page, PageElement};
use reinhardt_pages::reactive::{ResourceState, use_resource};
use reinhardt_pages::ssr::{SsrChunk, SsrOptions, SsrRenderer, SsrStream};
use std::time::Duration;

fn suspense_resource_view() -> Page {
	Page::reactive(|| {
		let resource = use_resource(
			|| async {
				tokio::time::sleep(Duration::from_millis(5)).await;
				Ok::<_, String>("resolved".to_string())
			},
			(),
		);
		let content_resource = resource.clone();

		SuspenseBoundary::new()
			.fallback(|| PageElement::new("span").child("fallback").into_page())
			.track(resource)
			.content(move || match content_resource.get() {
				ResourceState::Success(value) => {
					PageElement::new("strong").child(value).into_page()
				}
				ResourceState::Loading => PageElement::new("em").child("loading").into_page(),
				ResourceState::Error(error) => PageElement::new("em").child(error).into_page(),
			})
			.into_page()
	})
}

fn slow_suspense_resource_view() -> Page {
	delayed_suspense_resource_view(Duration::from_secs(60), "too-late")
}

fn delayed_suspense_resource_view(delay: Duration, value: &'static str) -> Page {
	Page::reactive(move || {
		let resource = use_resource(
			move || async move {
				tokio::time::sleep(delay).await;
				Ok::<_, String>(value.to_string())
			},
			(),
		);
		let content_resource = resource.clone();

		SuspenseBoundary::new()
			.fallback(|| PageElement::new("span").child("fallback").into_page())
			.track(resource)
			.content(move || match content_resource.get() {
				ResourceState::Success(value) => {
					PageElement::new("strong").child(value).into_page()
				}
				ResourceState::Loading => PageElement::new("em").child("loading").into_page(),
				ResourceState::Error(error) => PageElement::new("em").child(error).into_page(),
			})
			.into_page()
	})
}

#[tokio::test]
async fn suspense_streaming_emits_fallback_and_replacement() {
	let mut renderer = SsrRenderer::new();
	let html = renderer
		.render_page_with_view_head_to_string(suspense_resource_view())
		.await;

	assert!(html.contains("rh-suspense-start:rh-suspense-0"));
	assert!(html.contains(r#"data-rh-suspense="pending""#));
	assert!(html.contains("fallback"));
	assert!(html.contains(r#"data-rh-suspense-chunk="rh-suspense-0""#));
	assert!(html.contains("resolved"));
}

#[tokio::test]
async fn suspense_stream_emits_shell_replacement_and_closing_chunks() {
	let mut renderer = SsrRenderer::new();
	let mut stream = renderer
		.render_page_with_view_head(suspense_resource_view())
		.await;

	let shell = stream.next().await.unwrap().into_string();
	let replacement = stream.next().await.unwrap().into_string();
	let closing = stream.next().await.unwrap().into_string();

	assert!(shell.contains(r#"data-rh-suspense="pending""#));
	assert!(replacement.contains(r#"data-rh-suspense-chunk="rh-suspense-0""#));
	assert!(replacement.contains("resolved"));
	assert!(closing.contains(r#"<script id="ssr-state" type="application/json">"#));
	assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn suspense_stream_returns_shell_before_resource_resolves() {
	let mut renderer = SsrRenderer::new();
	let mut stream = tokio::time::timeout(
		Duration::from_millis(50),
		renderer.render_page_with_view_head(delayed_suspense_resource_view(
			Duration::from_millis(200),
			"resolved-later",
		)),
	)
	.await
	.expect("render_page should return before the boundary resource resolves");

	let shell = tokio::time::timeout(Duration::from_millis(50), stream.next())
		.await
		.expect("shell chunk should be available immediately")
		.expect("shell chunk")
		.into_string();

	assert!(shell.contains(r#"data-rh-suspense="pending""#));
	assert!(shell.contains("fallback"));
	assert!(!shell.contains("resolved-later"));

	let replacement = tokio::time::timeout(Duration::from_secs(1), stream.next())
		.await
		.expect("replacement chunk should arrive after the resource resolves")
		.expect("replacement chunk")
		.into_string();

	assert!(replacement.contains(r#"data-rh-suspense-chunk="rh-suspense-0""#));
	assert!(replacement.contains("resolved-later"));
}

#[tokio::test]
async fn suspense_replacement_uses_script_nonce() {
	let mut renderer = SsrRenderer::with_options(SsrOptions::new().script_nonce("nonce-123"));
	let html = renderer
		.render_page_with_view_head_to_string(suspense_resource_view())
		.await;

	assert!(html.contains(r#"<script nonce="nonce-123">"#));
}

#[tokio::test]
async fn suspense_timeout_keeps_fallback_without_replacement() {
	let mut renderer =
		SsrRenderer::with_options(SsrOptions::new().resource_timeout(Duration::from_millis(1)));
	let html = renderer
		.render_page_with_view_head_to_string(slow_suspense_resource_view())
		.await;

	assert!(html.contains(r#"data-rh-suspense="pending""#));
	assert!(html.contains("fallback"));
	assert!(!html.contains(r#"data-rh-suspense-chunk="rh-suspense-0""#));
	assert!(!html.contains("too-late"));
}

#[tokio::test]
async fn collecting_ssr_stream_produces_full_html() {
	let stream = SsrStream::from_chunks([
		SsrChunk::Html("<!DOCTYPE html>".to_string()),
		SsrChunk::Html("<html></html>".to_string()),
	]);

	assert_eq!(
		stream.collect_string().await,
		"<!DOCTYPE html><html></html>"
	);
}
