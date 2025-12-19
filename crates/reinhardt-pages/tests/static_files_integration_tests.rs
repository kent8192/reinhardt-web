//! Static Files Integration Tests
//!
//! Tests for static file URL generation and SSR component integration.
//!
//! Success Criteria:
//! 1. Static URL generation works correctly (✓ Implemented in reinhardt-static)
//! 2. Hash-based cache busting functions properly (✓ Implemented in reinhardt-static)
//! 3. Manifest-based URLs are correctly generated (✓ Implemented in reinhardt-static)
//! 4. Integration with SSR renderer is seamless
//!
//! Note: Category 1 (URL Generation) tests are in reinhardt-static crate.
//! This file focuses on Category 2 (SSR Component Integration).

use reinhardt_pages::component::{Component, IntoView, View};
use reinhardt_pages::ssr::{SsrOptions, SsrRenderer};
use reinhardt_static::TemplateStaticConfig;
use std::collections::HashMap;

// ============================================================================
// Category 2: SSR Component Integration (8 tests)
// ============================================================================

/// Test: Basic static URL in SSR render options
#[test]
fn test_ssr_with_basic_static_url() {
	let static_config = TemplateStaticConfig::new("/static/".to_string());

	let options = SsrOptions::new()
		.title("Test Page")
		.css(static_config.resolve_url("css/style.css"));

	let mut renderer = SsrRenderer::with_options(options);

	struct SimpleComponent;
	impl Component for SimpleComponent {
		fn render(&self) -> View {
			View::element("div").child("Hello, World!").into_view()
		}

		fn name() -> &'static str {
			"SimpleComponent"
		}
	}

	let html = renderer.render_page(&SimpleComponent);

	// Debug: Print generated HTML
	eprintln!("Generated HTML:\n{}", html);

	// Verify CSS link is in HTML
	assert!(html.contains("<link"), "HTML should contain link tag");
	assert!(
		html.contains("href=\"/static/css/style.css\""),
		"HTML should contain correct CSS URL.\nGenerated HTML:\n{}",
		html
	);
	assert!(
		html.contains("rel=\"stylesheet\""),
		"Link should be stylesheet"
	);
}

/// Test: Multiple CSS and JS static URLs
#[test]
fn test_ssr_with_multiple_static_urls() {
	let static_config = TemplateStaticConfig::new("/static/".to_string());

	let options = SsrOptions::new()
		.title("Test Page")
		.css(static_config.resolve_url("css/reset.css"))
		.css(static_config.resolve_url("css/main.css"))
		.js(static_config.resolve_url("js/vendor.js"))
		.js(static_config.resolve_url("js/app.js"));

	let mut renderer = SsrRenderer::with_options(options);

	struct TestComponent;
	impl Component for TestComponent {
		fn render(&self) -> View {
			View::element("h1").child("Test").into_view()
		}

		fn name() -> &'static str {
			"TestComponent"
		}
	}

	let html = renderer.render_page(&TestComponent);

	// Verify all CSS links
	assert!(
		html.contains("href=\"/static/css/reset.css\""),
		"HTML should contain reset.css"
	);
	assert!(
		html.contains("href=\"/static/css/main.css\""),
		"HTML should contain main.css"
	);

	// Verify all JS scripts
	assert!(
		html.contains("src=\"/static/js/vendor.js\""),
		"HTML should contain vendor.js"
	);
	assert!(
		html.contains("src=\"/static/js/app.js\""),
		"HTML should contain app.js"
	);

	// Verify order: CSS should come before JS in head
	let css_pos = html.find("css/reset.css").unwrap();
	let js_pos = html.find("js/vendor.js").unwrap();
	assert!(
		css_pos < js_pos,
		"CSS links should appear before JS scripts"
	);
}

/// Test: Manifest-based hashed URLs in SSR
#[test]
fn test_ssr_with_manifest_urls() {
	let mut manifest = HashMap::new();
	manifest.insert(
		"css/app.css".to_string(),
		"css/app.abc123def456.css".to_string(),
	);
	manifest.insert(
		"js/bundle.js".to_string(),
		"js/bundle.789xyz.js".to_string(),
	);

	let static_config = TemplateStaticConfig::new("/static/".to_string()).with_manifest(manifest);

	let options = SsrOptions::new()
		.title("Manifest Test")
		.css(static_config.resolve_url("css/app.css"))
		.js(static_config.resolve_url("js/bundle.js"));

	let mut renderer = SsrRenderer::with_options(options);

	struct ManifestComponent;
	impl Component for ManifestComponent {
		fn render(&self) -> View {
			View::element("p").child("Manifest test").into_view()
		}

		fn name() -> &'static str {
			"ManifestComponent"
		}
	}

	let html = renderer.render_page(&ManifestComponent);

	// Verify hashed filenames are used
	assert!(
		html.contains("href=\"/static/css/app.abc123def456.css\""),
		"HTML should contain hashed CSS filename"
	);
	assert!(
		html.contains("src=\"/static/js/bundle.789xyz.js\""),
		"HTML should contain hashed JS filename"
	);

	// Verify original filenames are NOT used
	assert!(
		!html.contains("css/app.css\""),
		"HTML should not contain original CSS filename"
	);
	assert!(
		!html.contains("js/bundle.js\""),
		"HTML should not contain original JS filename"
	);
}

/// Test: CDN URLs in SSR
#[test]
fn test_ssr_with_cdn_urls() {
	let static_config = TemplateStaticConfig::new("https://cdn.example.com/static/".to_string());

	let options = SsrOptions::new()
		.title("CDN Test")
		.css(static_config.resolve_url("css/style.css"))
		.js(static_config.resolve_url("js/app.js"));

	let mut renderer = SsrRenderer::with_options(options);

	struct CdnComponent;
	impl Component for CdnComponent {
		fn render(&self) -> View {
			View::element("div").child("CDN test").into_view()
		}

		fn name() -> &'static str {
			"CdnComponent"
		}
	}

	let html = renderer.render_page(&CdnComponent);

	// Verify CDN URLs are used
	assert!(
		html.contains("href=\"https://cdn.example.com/static/css/style.css\""),
		"HTML should contain CDN CSS URL"
	);
	assert!(
		html.contains("src=\"https://cdn.example.com/static/js/app.js\""),
		"HTML should contain CDN JS URL"
	);
}

/// Test: Query strings and fragments in static URLs
#[test]
fn test_ssr_with_query_and_fragment_urls() {
	let static_config = TemplateStaticConfig::new("/static/".to_string());

	let options = SsrOptions::new()
		.title("Query Test")
		.css(static_config.resolve_url("css/style.css?v=1.2.3"))
		.js(static_config.resolve_url("js/app.js#main"));

	let mut renderer = SsrRenderer::with_options(options);

	struct QueryComponent;
	impl Component for QueryComponent {
		fn render(&self) -> View {
			View::element("div").child("Query test").into_view()
		}

		fn name() -> &'static str {
			"QueryComponent"
		}
	}

	let html = renderer.render_page(&QueryComponent);

	// Verify query strings and fragments are preserved
	assert!(
		html.contains("href=\"/static/css/style.css?v=1.2.3\""),
		"HTML should contain CSS URL with query string"
	);
	assert!(
		html.contains("src=\"/static/js/app.js#main\""),
		"HTML should contain JS URL with fragment"
	);
}

/// Test: Empty static config (no CSS/JS)
#[test]
fn test_ssr_without_static_files() {
	let options = SsrOptions::new().title("No Static Files");

	let mut renderer = SsrRenderer::with_options(options);

	struct EmptyStaticComponent;
	impl Component for EmptyStaticComponent {
		fn render(&self) -> View {
			View::element("div").child("No static files").into_view()
		}

		fn name() -> &'static str {
			"EmptyStaticComponent"
		}
	}

	let html = renderer.render_page(&EmptyStaticComponent);

	// Verify no CSS or JS tags
	assert!(
		!html.contains("<link rel=\"stylesheet\""),
		"HTML should not contain stylesheet link"
	);
	assert!(
		!html.contains("<script src="),
		"HTML should not contain script tag with src"
	);

	// But should still have basic HTML structure
	assert!(html.contains("<title>No Static Files</title>"));
	assert!(html.contains("<div>No static files</div>"));
}

/// Test: Relative path normalization
#[test]
fn test_ssr_with_relative_paths() {
	// Test various base URL formats
	let configs = vec![
		("/static/", "/static/css/app.css"),
		("/static", "/static/css/app.css"),
		("static/", "static/css/app.css"),
		("static", "static/css/app.css"),
	];

	for (base_url, expected_url) in configs {
		let static_config = TemplateStaticConfig::new(base_url.to_string());
		let options = SsrOptions::new().css(static_config.resolve_url("css/app.css"));

		let mut renderer = SsrRenderer::with_options(options);

		struct PathComponent;
		impl Component for PathComponent {
			fn render(&self) -> View {
				View::element("div").child("Path test").into_view()
			}

			fn name() -> &'static str {
				"PathComponent"
			}
		}

		let html = renderer.render_page(&PathComponent);

		assert!(
			html.contains(&format!("href=\"{}\"", expected_url)),
			"HTML should contain normalized URL: {} (base: {})",
			expected_url,
			base_url
		);
	}
}

/// Test: Manifest fallback to original path
#[test]
fn test_ssr_manifest_fallback() {
	let mut manifest = HashMap::new();
	manifest.insert(
		"css/known.css".to_string(),
		"css/known.hash123.css".to_string(),
	);

	let static_config = TemplateStaticConfig::new("/static/".to_string()).with_manifest(manifest);

	let options = SsrOptions::new()
		.css(static_config.resolve_url("css/known.css")) // In manifest
		.css(static_config.resolve_url("css/unknown.css")); // Not in manifest

	let mut renderer = SsrRenderer::with_options(options);

	struct FallbackComponent;
	impl Component for FallbackComponent {
		fn render(&self) -> View {
			View::element("div").child("Fallback test").into_view()
		}

		fn name() -> &'static str {
			"FallbackComponent"
		}
	}

	let html = renderer.render_page(&FallbackComponent);

	// Known file should use hashed name
	assert!(
		html.contains("href=\"/static/css/known.hash123.css\""),
		"Known file should use manifest hash"
	);

	// Unknown file should use original name
	assert!(
		html.contains("href=\"/static/css/unknown.css\""),
		"Unknown file should fallback to original path"
	);
}
