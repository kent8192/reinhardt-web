#![cfg(not(target_arch = "wasm32"))]

use futures_util::StreamExt;
use reinhardt_core::types::page::SuspenseNode;
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
async fn buffered_suspense_emits_resolved_content_directly() {
	let mut renderer = SsrRenderer::new();
	let html = renderer
		.render_page_with_view_head_to_string(suspense_resource_view())
		.await;

	assert!(html.contains("resolved"));
	assert!(html.contains(r#"data-rh-suspense="resolved""#));
	assert!(!html.contains("rh-suspense-start:rh-suspense-0"));
	assert!(!html.contains(r#"data-rh-suspense="pending""#));
	assert!(!html.contains(r#"data-rh-suspense-chunk="rh-suspense-0""#));
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
	let mut stream = renderer
		.render_page_with_view_head(suspense_resource_view())
		.await;

	let _shell = stream.next().await.unwrap();
	let replacement = stream.next().await.unwrap().into_string();
	assert!(replacement.contains(r#"<script nonce="nonce-123">"#));
}

#[tokio::test]
async fn suspense_replacement_escapes_boundary_id_for_script() {
	let boundary_id = "x</script><script>alert(1)</script>";
	let view = Page::reactive(move || {
		let resource = use_resource(|| async { Ok::<_, String>("safe-content".to_string()) }, ());
		let tracked_key = resource.ssr_key().unwrap().to_string();
		let pending_resource = resource.clone();
		let content_resource = resource.clone();

		Page::Suspense(SuspenseNode::new_with_tracked_resources(
			Some(boundary_id.to_string()),
			vec![tracked_key],
			move || pending_resource.is_loading(),
			|| PageElement::new("span").child("fallback").into_page(),
			move || match content_resource.get() {
				ResourceState::Success(value) => {
					PageElement::new("strong").child(value).into_page()
				}
				ResourceState::Loading => PageElement::new("em").child("loading").into_page(),
				ResourceState::Error(error) => PageElement::new("em").child(error).into_page(),
			},
		))
	});

	let mut renderer = SsrRenderer::new();
	let mut stream = renderer.render_page_with_view_head(view).await;
	let _shell = stream.next().await.unwrap();
	let replacement = stream.next().await.unwrap().into_string();

	assert_eq!(replacement.matches("</script>").count(), 1);
	assert!(replacement.contains(r#"<\/script>"#));
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
async fn streamed_replacement_preserves_nested_suspense_markers() {
	let view = Page::reactive(|| {
		let outer = use_resource(
			|| async {
				tokio::time::sleep(Duration::from_millis(5)).await;
				Ok::<_, String>("outer".to_string())
			},
			(),
		);
		let outer_content = outer.clone();

		SuspenseBoundary::new()
			.fallback(|| PageElement::new("span").child("outer-fallback").into_page())
			.track(outer)
			.content(move || match outer_content.get() {
				ResourceState::Success(_) => Page::reactive(|| {
					let inner = use_resource(
						|| async {
							tokio::time::sleep(Duration::from_millis(20)).await;
							Ok::<_, String>("inner-resolved".to_string())
						},
						(),
					);
					let inner_content = inner.clone();

					SuspenseBoundary::new()
						.fallback(|| PageElement::new("span").child("inner-fallback").into_page())
						.track(inner)
						.content(move || match inner_content.get() {
							ResourceState::Success(value) => {
								PageElement::new("strong").child(value).into_page()
							}
							ResourceState::Loading => {
								PageElement::new("em").child("inner-loading").into_page()
							}
							ResourceState::Error(error) => {
								PageElement::new("em").child(error).into_page()
							}
						})
						.into_page()
				}),
				ResourceState::Loading => PageElement::new("em").child("outer-loading").into_page(),
				ResourceState::Error(error) => PageElement::new("em").child(error).into_page(),
			})
			.into_page()
	});

	let mut renderer = SsrRenderer::new();
	let mut stream = renderer.render_page_with_view_head(view).await;
	let shell = stream.next().await.unwrap().into_string();
	let outer_replacement = stream.next().await.unwrap().into_string();
	let inner_replacement = stream.next().await.unwrap().into_string();

	assert!(shell.contains("outer-fallback"));
	assert!(outer_replacement.contains("inner-fallback"));
	assert!(outer_replacement.contains("rh-suspense-start:rh-suspense-1"));
	assert!(inner_replacement.contains(r#"data-rh-suspense-chunk="rh-suspense-1""#));
	assert!(inner_replacement.contains("inner-resolved"));
}

#[tokio::test]
async fn suspense_boundary_ids_reset_between_reused_stream_renders() {
	let mut renderer = SsrRenderer::new();

	let mut first = renderer
		.render_page_with_view_head(delayed_suspense_resource_view(
			Duration::from_millis(5),
			"first",
		))
		.await;
	let first_shell = first.next().await.unwrap().into_string();

	let mut second = renderer
		.render_page_with_view_head(delayed_suspense_resource_view(
			Duration::from_millis(5),
			"second",
		))
		.await;
	let second_shell = second.next().await.unwrap().into_string();

	assert!(first_shell.contains("rh-suspense-start:rh-suspense-0"));
	assert!(second_shell.contains("rh-suspense-start:rh-suspense-0"));
	assert!(!second_shell.contains("rh-suspense-start:rh-suspense-1"));
}

#[tokio::test]
async fn custom_pending_suspense_renders_fallback_on_ssr() {
	let view = Page::Suspense(SuspenseNode::new(
		Some("custom-pending".to_string()),
		|| true,
		|| {
			PageElement::new("span")
				.child("custom-fallback")
				.into_page()
		},
		|| {
			PageElement::new("strong")
				.child("custom-content")
				.into_page()
		},
	));

	let mut renderer = SsrRenderer::new();
	let html = renderer.render_page_with_view_head_to_string(view).await;

	assert!(html.contains("custom-fallback"));
	assert!(!html.contains("custom-content"));
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
