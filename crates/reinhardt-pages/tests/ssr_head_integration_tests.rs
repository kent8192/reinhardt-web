//! SSR and Head integration tests
//!
//! Tests the integration between SsrRenderer and View's Head system.
//!
//! Success Criteria:
//! 1. render_page_with_view_head extracts and uses View's head
//! 2. View's head elements are ADDITIVE to SsrOptions (not replacing)
//! 3. View's title takes precedence over SsrOptions title
//! 4. When no View head, SsrOptions head is used
//! 5. Both sources' elements are present in final output
//!
//! Test Categories:
//! - View Head Only: 2 tests
//! - SsrOptions Head Only: 2 tests
//! - Merged Head (additive): 4 tests
//! - Edge Cases: 2 tests
//!
//! Total: 10 tests

#[cfg(not(target_arch = "wasm32"))]
mod ssr_tests {
	use reinhardt_pages::component::{ElementView, Head, IntoView, LinkTag, MetaTag};
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
		let view = ElementView::new("div")
			.child("Content")
			.into_view()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<title>View Title</title>"));
	}

	/// Tests that render_page_with_view_head includes View's meta tags.
	#[rstest]
	fn test_render_page_with_view_head_includes_view_meta() {
		let view_head = Head::new().meta(MetaTag::new("description", "View description"));
		let view = ElementView::new("div")
			.child("Content")
			.into_view()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<meta name=\"description\" content=\"View description\""));
	}

	// ============================================================================
	// SsrOptions Head Only Tests
	// ============================================================================

	/// Tests that when View has no head, SsrOptions title is used.
	#[rstest]
	#[allow(deprecated)] // Testing deprecated API for backwards compatibility
	fn test_no_view_head_uses_ssr_options_title() {
		let view = ElementView::new("div").child("Content").into_view();

		let mut renderer = SsrRenderer::with_options(
			reinhardt_pages::ssr::SsrOptions::new().title("Options Title"),
		);
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<title>Options Title</title>"));
	}

	/// Tests that SsrOptions meta tags are always included.
	#[rstest]
	#[allow(deprecated)] // Testing deprecated API for backwards compatibility
	fn test_ssr_options_meta_always_included() {
		let view = ElementView::new("div").child("Content").into_view();

		let mut renderer = SsrRenderer::with_options(
			reinhardt_pages::ssr::SsrOptions::new().meta("author", "Test Author"),
		);
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<meta name=\"author\" content=\"Test Author\">"));
	}

	// ============================================================================
	// Merged Head (Additive) Tests
	// ============================================================================

	/// Tests that View's title takes precedence over SsrOptions title.
	#[rstest]
	#[allow(deprecated)] // Testing deprecated API for backwards compatibility
	fn test_view_title_takes_precedence() {
		let view_head = Head::new().title("View Title");
		let view = ElementView::new("div")
			.child("Content")
			.into_view()
			.with_head(view_head);

		let mut renderer = SsrRenderer::with_options(
			reinhardt_pages::ssr::SsrOptions::new().title("Options Title"),
		);
		let html = renderer.render_page_with_view_head(view);

		// View title should be present
		assert!(html.contains("<title>View Title</title>"));
		// Options title should NOT be present
		assert!(!html.contains("Options Title"));
	}

	/// Tests that SsrOptions meta tags and View meta tags are both present (additive).
	#[rstest]
	#[allow(deprecated)] // Testing deprecated API for backwards compatibility
	fn test_meta_tags_are_additive() {
		let view_head = Head::new().meta(MetaTag::new("description", "View desc"));
		let view = ElementView::new("div")
			.child("Content")
			.into_view()
			.with_head(view_head);

		let mut renderer = SsrRenderer::with_options(
			reinhardt_pages::ssr::SsrOptions::new().meta("author", "Test Author"),
		);
		let html = renderer.render_page_with_view_head(view);

		// Both meta tags should be present
		assert!(html.contains("<meta name=\"author\" content=\"Test Author\">"));
		assert!(html.contains("<meta name=\"description\" content=\"View desc\""));
	}

	/// Tests that SsrOptions CSS and View CSS are both present (additive).
	#[rstest]
	#[allow(deprecated)] // Testing deprecated API for backwards compatibility
	fn test_css_links_are_additive() {
		let view_head = Head::new().link(LinkTag::new("stylesheet", "/view-style.css"));
		let view = ElementView::new("div")
			.child("Content")
			.into_view()
			.with_head(view_head);

		let mut renderer = SsrRenderer::with_options(
			reinhardt_pages::ssr::SsrOptions::new().css("/options-style.css"),
		);
		let html = renderer.render_page_with_view_head(view);

		// Both CSS links should be present
		assert!(html.contains("href=\"/options-style.css\""));
		assert!(html.contains("href=\"/view-style.css\""));
	}

	/// Tests that order is preserved: SsrOptions elements first, then View elements.
	#[rstest]
	#[allow(deprecated)] // Testing deprecated API for backwards compatibility
	fn test_order_ssr_options_before_view() {
		let view_head = Head::new().meta(MetaTag::new("view-meta", "view"));
		let view = ElementView::new("div")
			.child("Content")
			.into_view()
			.with_head(view_head);

		let mut renderer = SsrRenderer::with_options(
			reinhardt_pages::ssr::SsrOptions::new().meta("options-meta", "options"),
		);
		let html = renderer.render_page_with_view_head(view);

		// Find positions
		let options_pos = html.find("options-meta").expect("options-meta not found");
		let view_pos = html.find("view-meta").expect("view-meta not found");

		// SsrOptions should come before View
		assert!(
			options_pos < view_pos,
			"SsrOptions meta should appear before View meta"
		);
	}

	// ============================================================================
	// Edge Case Tests
	// ============================================================================

	/// Tests that empty View head doesn't break rendering.
	#[rstest]
	fn test_empty_view_head_renders_correctly() {
		let view_head = Head::new(); // Empty head
		let view = ElementView::new("div")
			.child("Content")
			.into_view()
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
			meta { name: "description", content: "Macro description" }
		});

		let view = ElementView::new("div")
			.child("Hello")
			.into_view()
			.with_head(page_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head(view);

		assert!(html.contains("<title>Macro Title</title>"));
		assert!(html.contains("<meta name=\"description\" content=\"Macro description\""));
		assert!(html.contains("<div>Hello</div>"));
	}
}
