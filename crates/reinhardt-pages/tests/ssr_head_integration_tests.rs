#![cfg(not(target_arch = "wasm32"))]
//! SSR and Head integration tests
//!
//! Tests the integration between SsrRenderer and View's Head system.
//!
//! Success Criteria:
//! 1. render_page_with_view_head extracts and uses View's head
//! 2. View's head elements are properly rendered
//! 3. When no View head, basic HTML structure is maintained
//! 4. Multiple head elements are all rendered
//!
//! Test Categories:
//! - View Head Only: 2 tests
//! - No Head: 1 test
//! - Multiple Elements: 4 tests
//! - Edge Cases: 2 tests
//!
//! Total: 9 tests

#[cfg(native)]
mod ssr_tests {
	use reinhardt_pages::component::{Head, IntoPage, LinkTag, MetaTag, PageElement};
	use reinhardt_pages::head;
	use reinhardt_pages::ssr::SsrRenderer;
	use rstest::*;

	// ============================================================================
	// View Head Only Tests
	// ============================================================================

	/// Tests that render_page_with_view_head uses View's title.
	#[rstest]
	fn test_render_page_with_view_head_uses_view_title() {
		let view_head = Head::new().title("View Title");
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<title>View Title</title>"));
	}

	/// Tests that render_page_with_view_head includes View's meta tags.
	#[rstest]
	fn test_render_page_with_view_head_includes_view_meta() {
		let view_head = Head::new().meta(MetaTag::new("description", "View description"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<meta name=\"description\" content=\"View description\""));
	}

	// ============================================================================
	// No Head Tests
	// ============================================================================

	/// Tests rendering without any head elements produces no title.
	#[rstest]
	fn test_render_without_head_has_no_title() {
		let view = PageElement::new("div").child("Content").into_page();

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		// No <title> tag when no head provided
		assert!(!html.contains("<title>"));
		// But basic structure is present
		assert!(html.contains("<!DOCTYPE html>"));
		assert!(html.contains("<head>"));
		assert!(html.contains("</head>"));
	}

	// ============================================================================
	// Multiple Elements Tests
	// ============================================================================

	/// Tests multiple meta tags via Head.
	#[rstest]
	fn test_multiple_meta_tags_via_head() {
		let view_head = Head::new()
			.meta(MetaTag::new("description", "Page desc"))
			.meta(MetaTag::new("author", "Test Author"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<meta name=\"description\" content=\"Page desc\""));
		assert!(html.contains("<meta name=\"author\" content=\"Test Author\""));
	}

	/// Tests multiple CSS links via Head.
	#[rstest]
	fn test_multiple_css_links_via_head() {
		let view_head = Head::new()
			.link(LinkTag::new("stylesheet", "/style1.css"))
			.link(LinkTag::new("stylesheet", "/style2.css"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("href=\"/style1.css\""));
		assert!(html.contains("href=\"/style2.css\""));
	}

	/// Tests title combined with meta tags.
	#[rstest]
	fn test_title_with_meta_tags() {
		let view_head = Head::new()
			.title("My Page")
			.meta(MetaTag::new("description", "Page description"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<title>My Page</title>"));
		assert!(html.contains("<meta name=\"description\" content=\"Page description\""));
	}

	/// Tests exact duplicate asset hints are deduplicated during SSR.
	#[rstest]
	fn test_duplicate_asset_hints_are_deduplicated_during_ssr() {
		let view_head = Head::new()
			.preconnect("https://cdn.example.com")
			.preconnect("https://cdn.example.com")
			.preload_script("/static/app.js")
			.preload_script("/static/app.js");
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert_eq!(
			html.matches("<link rel=\"preconnect\" href=\"https://cdn.example.com\">")
				.count(),
			1
		);
		assert_eq!(
			html.matches("<link rel=\"preload\" href=\"/static/app.js\" as=\"script\">")
				.count(),
			1
		);
	}

	/// Tests exact duplicate default meta tags are deduplicated during SSR.
	#[rstest]
	fn test_default_meta_tags_are_deduplicated_during_ssr() {
		let view_head = Head::with_defaults();
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert_eq!(html.matches("<meta charset=\"UTF-8\">").count(), 1);
		assert_eq!(
			html.matches(
				"<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
			)
			.count(),
			1
		);
	}

	// ============================================================================
	// Edge Case Tests
	// ============================================================================

	/// Tests that empty View head doesn't break rendering.
	#[rstest]
	fn test_empty_view_head_renders_correctly() {
		let view_head = Head::new(); // Empty head
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		// Should still render basic HTML structure
		assert!(html.contains("<!DOCTYPE html>"));
		assert!(html.contains("<head>"));
		assert!(html.contains("</head>"));
		assert!(html.contains("<div>Content</div>"));
	}

	/// Tests rendering with head! macro generated Head.
	#[rstest]
	fn test_render_with_head_macro() {
		let page_head = head!(|| {
			title { "Macro Title" }
			meta {
				name: "description",
				content: "Macro description"
			}
		});

		let view = PageElement::new("div")
			.child("Hello")
			.into_page()
			.with_head(page_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<title>Macro Title</title>"));
		assert!(html.contains("<meta name=\"description\" content=\"Macro description\""));
		assert!(html.contains("<div>Hello</div>"));
	}
}
