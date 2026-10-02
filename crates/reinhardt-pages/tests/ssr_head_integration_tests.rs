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
	use reinhardt_pages::component::{Head, IntoPage, LinkTag, MetaTag, Outlet, Page, PageElement};
	use reinhardt_pages::reactive::hooks::{use_head, use_page_title};
	use reinhardt_pages::ssr::SsrRenderer;
	use reinhardt_pages::{deps, head, page};
	use reinhardt_urls::routers::{ClientRouter, RouteMetadata};

	fn has_managed_head_entry(
		html: &str,
		tag: &str,
		marker_fragment: &str,
		semantic_content: &str,
	) -> bool {
		let marker_prefix = format!("<{tag} data-reinhardt-head=\"{marker_fragment}");
		html.lines()
			.any(|line| line.starts_with(&marker_prefix) && line.contains(semantic_content))
	}

	fn managed_head_entry_count(html: &str, tag: &str, semantic_content: &str) -> usize {
		let marker_prefix = format!("<{tag} data-reinhardt-head=\"");
		html.lines()
			.filter(|line| line.starts_with(&marker_prefix) && line.contains(semantic_content))
			.count()
	}

	// ============================================================================
	// View Head Only Tests
	// ============================================================================

	/// Tests that render_page_with_view_head uses View's title.
	#[tokio::test]
	async fn test_render_page_with_view_head_uses_view_title() {
		let view_head = Head::new().title("View Title");
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert!(has_managed_head_entry(
			&html,
			"title",
			"",
			"\">View Title</title>",
		));
	}

	/// Tests that render_page_with_view_head includes View's meta tags.
	#[tokio::test]
	async fn test_render_page_with_view_head_includes_view_meta() {
		let view_head = Head::new().meta(MetaTag::new("description", "View description"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"description\" content=\"View description\"",
		));
	}

	// ============================================================================
	// No Head Tests
	// ============================================================================

	/// Tests rendering without any head elements produces no title.
	#[tokio::test]
	async fn test_render_without_head_has_no_title() {
		let view = PageElement::new("div").child("Content").into_page();

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

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
	#[tokio::test]
	async fn test_multiple_meta_tags_via_head() {
		let view_head = Head::new()
			.meta(MetaTag::new("description", "Page desc"))
			.meta(MetaTag::new("author", "Test Author"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"description\" content=\"Page desc\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"author\" content=\"Test Author\"",
		));
	}

	/// Tests multiple CSS links via Head.
	#[tokio::test]
	async fn test_multiple_css_links_via_head() {
		let view_head = Head::new()
			.link(LinkTag::new("stylesheet", "/style1.css"))
			.link(LinkTag::new("stylesheet", "/style2.css"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert!(has_managed_head_entry(
			&html,
			"link",
			"",
			"rel=\"stylesheet\" href=\"/style1.css\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"link",
			"",
			"rel=\"stylesheet\" href=\"/style2.css\"",
		));
	}

	/// Tests title combined with meta tags.
	#[tokio::test]
	async fn test_title_with_meta_tags() {
		let view_head = Head::new()
			.title("My Page")
			.meta(MetaTag::new("description", "Page description"));
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert!(has_managed_head_entry(
			&html,
			"title",
			"",
			"\">My Page</title>",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"description\" content=\"Page description\"",
		));
	}

	/// Tests exact duplicate asset hints are deduplicated during SSR.
	#[tokio::test]
	async fn test_duplicate_asset_hints_are_deduplicated_during_ssr() {
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
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert_eq!(
			managed_head_entry_count(
				&html,
				"link",
				"rel=\"preconnect\" href=\"https://cdn.example.com\"",
			),
			1
		);
		assert_eq!(
			managed_head_entry_count(
				&html,
				"link",
				"rel=\"preload\" href=\"/static/app.js\" as=\"script\"",
			),
			1
		);
	}

	/// Tests exact duplicate default meta tags are deduplicated during SSR.
	#[tokio::test]
	async fn test_default_meta_tags_are_deduplicated_during_ssr() {
		let view_head = Head::with_defaults();
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert_eq!(html.matches("charset=\"UTF-8\"").count(), 1);
		assert_eq!(
			html.matches("name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"")
				.count(),
			1
		);
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"charset=\"UTF-8\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"",
		));
	}

	#[tokio::test]
	async fn implicit_default_meta_entries_remain_unmanaged() {
		let mut renderer = SsrRenderer::new();
		let html = renderer
			.render_page_with_view_head_to_string(Page::text("Content"))
			.await;

		assert_eq!(html.matches("<meta charset=\"UTF-8\">").count(), 1);
		assert_eq!(
			html.matches(
				"<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
			)
			.count(),
			1
		);
		assert!(!has_managed_head_entry(
			&html,
			"meta",
			"",
			"charset=\"UTF-8\"",
		));
		assert!(!has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"",
		));
	}

	#[tokio::test]
	async fn managed_head_entries_compose_in_structural_order() {
		// Arrange
		let view = Page::fragment([
			Page::text("before").with_head(
				Head::new()
					.title("Parent")
					.meta_description("parent")
					.base_url("https://example.test/parent/"),
			),
			Page::text("child").with_head(
				Head::new()
					.title("Child")
					.canonical("https://example.test/child")
					.base_url("https://example.test/child/"),
			),
		]);
		let mut renderer = SsrRenderer::new();

		// Act
		let html = renderer.render_page_with_view_head_to_string(view).await;

		// Assert
		assert!(has_managed_head_entry(
			&html,
			"title",
			"slot-2-title-",
			"\">Child</title>",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"slot-1-meta-",
			"name=\"description\" content=\"parent\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"link",
			"slot-2-link-",
			"rel=\"canonical\" href=\"https://example.test/child\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"base",
			"slot-2-base-",
			"href=\"https://example.test/child/\"",
		));
		assert!(!html.contains("https://example.test/parent/"));
	}

	#[tokio::test]
	async fn retained_head_hooks_compose_after_static_page_heads() {
		// Arrange
		let view = Page::fragment([
			Page::reactive(|| {
				use_head(
					|| Head::new().meta(MetaTag::new("hook", "retained")),
					deps![],
				);
				use_page_title(|| "Hook title", deps![]);
				Page::text("hook body")
			}),
			Page::text("static body").with_head(
				Head::new()
					.title("Static title")
					.meta_description("static description"),
			),
		]);
		let mut renderer = SsrRenderer::new();

		// Act
		let html = renderer.render_page_with_view_head_to_string(view).await;

		// Assert
		assert!(has_managed_head_entry(
			&html,
			"title",
			"slot-2-title-",
			"\">Hook title</title>",
		));
		assert!(!html.contains("\">Static title</title>"));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"slot-1-meta-",
			"name=\"hook\" content=\"retained\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"slot-3-meta-",
			"name=\"description\" content=\"static description\"",
		));
	}

	#[tokio::test]
	async fn exact_duplicate_uses_the_first_structural_owner_marker() {
		// Arrange
		let duplicate = Head::new().meta_description("shared description");
		let view = Page::fragment([
			Page::text("first").with_head(duplicate.clone()),
			Page::text("second").with_head(duplicate),
		]);
		let mut renderer = SsrRenderer::new();

		// Act
		let html = renderer.render_page_with_view_head_to_string(view).await;

		// Assert
		assert_eq!(
			html.matches("name=\"description\" content=\"shared description\"")
				.count(),
			1
		);
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"slot-1-meta-",
			"name=\"description\" content=\"shared description\"",
		));
		assert!(!has_managed_head_entry(
			&html,
			"meta",
			"slot-2-meta-",
			"name=\"description\" content=\"shared description\"",
		));
	}

	#[test]
	fn standalone_head_html_remains_unmarked() {
		// Arrange
		let head = Head::new().title("x");

		// Act
		let html = head.to_html();

		// Assert
		assert_eq!(html, "<title>x</title>");
		assert!(!html.contains("data-reinhardt-head"));
	}

	// ============================================================================
	// Edge Case Tests
	// ============================================================================

	/// Tests that empty View head doesn't break rendering.
	#[tokio::test]
	async fn test_empty_view_head_renders_correctly() {
		let view_head = Head::new(); // Empty head
		let view = PageElement::new("div")
			.child("Content")
			.into_page()
			.with_head(view_head);

		let mut renderer = SsrRenderer::new();
		let html = renderer.render_page_with_view_head_to_string(view).await;

		// Should still render basic HTML structure
		assert!(html.contains("<!DOCTYPE html>"));
		assert!(html.contains("<head>"));
		assert!(html.contains("</head>"));
		assert!(html.contains("<div>Content</div>"));
	}

	/// Tests rendering with head! macro generated Head.
	#[tokio::test]
	async fn test_render_with_head_macro() {
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
		let html = renderer.render_page_with_view_head_to_string(view).await;

		assert!(has_managed_head_entry(
			&html,
			"title",
			"",
			"\">Macro Title</title>",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"description\" content=\"Macro description\"",
		));
		assert!(html.contains("<div>Hello</div>"));
	}

	#[tokio::test]
	async fn client_router_metadata_heads_merge_with_leaf_head_declaration() {
		fn layout(outlet: Outlet) -> Page {
			PageElement::new("section")
				.child(outlet.into_page())
				.into_page()
		}

		fn leaf() -> Page {
			let leaf_head = head!(|| {
				title { "Leaf declaration" }
				meta {
					name: "leaf-declaration",
					content: "present"
				}
			});
			page!(#head: leaf_head, {
				main { "Leaf page" }
			})
		}

		let router = ClientRouter::new()
			.try_routes(|routes| {
				routes.layout_route("root", "/", layout, |children| {
					children.layout_route("nested", "nested/", layout, |children| {
						children.index_route("leaf", leaf)
					})
				})
			})
			.expect("route tree should register")
			.with_route_metadata(
				"root",
				RouteMetadata::new().with_head(
					Head::new()
						.title("Root route")
						.meta_description("root metadata"),
				),
			)
			.with_route_metadata(
				"nested",
				RouteMetadata::new().with_head(
					Head::new()
						.title("Nested route")
						.meta_description("nested metadata"),
				),
			)
			.with_route_metadata(
				"leaf",
				RouteMetadata::new().with_head(
					Head::new()
						.title("Leaf route")
						.meta_description("leaf metadata"),
				),
			);

		let mut renderer = SsrRenderer::new();
		let html = renderer
			.render_page_with_view_head_to_string(router.render_path("/nested/"))
			.await;

		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"description\" content=\"root metadata\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"description\" content=\"nested metadata\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"description\" content=\"leaf metadata\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"meta",
			"",
			"name=\"leaf-declaration\" content=\"present\"",
		));
		assert!(has_managed_head_entry(
			&html,
			"title",
			"",
			"\">Leaf declaration</title>",
		));
	}
}
