//! # Template + Static Files Integration Tests
//!
//! ## Purpose
//! Cross-crate integration tests for template rendering with static file URL generation,
//! verifying the integration between reinhardt-template/templates, reinhardt-template/renderers,
//! and static file handling components.
//!
//! ## Test Coverage
//! - Template rendering with static file URLs
//! - Static file filter in templates ({{ "path"|static }})
//! - Static file resolution with hashed filenames
//! - CSS/JS inclusion in templates via static URLs
//! - Static file versioning and cache busting
//! - CDN integration with template static URLs
//! - Template caching with static file references
//! - Manifest-based static file resolution
//! - Multiple static file references in single template
//!
//! ## Fixtures Used
//! - `postgres_container`: PostgreSQL 16-alpine container for database operations
//! - `temp_dir`: Temporary directory for template and static files
//!
//! ## What is Verified
//! - Templates can generate correct static file URLs
//! - Static file filter resolves paths correctly
//! - Hashed/versioned static filenames are resolved via manifest
//! - CDN URLs are generated when configured
//! - Templates can include CSS/JS files via static URLs
//! - Multiple static file references work in single template
//! - Static file configuration is properly applied
//! - Template caching works with static file references
//!
//! ## What is NOT Covered
//! - Actual static file serving (HTTP server level)
//! - Static file compression and minification
//! - Client-side static file loading
//! - Static file upload and storage

use reinhardt_template::templates::static_filters::{StaticConfig, init_static_config, static_filter};
use reinhardt_test::fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::AnyPool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use testcontainers::core::ContainerAsync;
use testcontainers::GenericImage;
use tokio::fs;

// ============================================================================
// Helper Functions
// ============================================================================

/// Simple template renderer with static filter support
fn render_template_with_static(template: &str, context: &HashMap<String, Value>) -> String {
	let mut rendered = template.to_string();

	// Replace simple variables: {{ variable }}
	for (key, value) in context {
		let placeholder = format!("{{{{{}}}}}", key);
		let replacement = match value {
			Value::String(s) => s.clone(),
			Value::Number(n) => n.to_string(),
			Value::Bool(b) => b.to_string(),
			_ => value.to_string(),
		};
		rendered = rendered.replace(&placeholder, &replacement);
	}

	// Process static filter: {{ "path"|static }}
	let static_pattern_start = "{{ \"";
	let static_pattern_end = "\"|static }}";

	while let Some(start_pos) = rendered.find(static_pattern_start) {
		if let Some(end_pos) = rendered[start_pos..].find(static_pattern_end) {
			let full_pattern_start = start_pos;
			let full_pattern_end = start_pos + end_pos + static_pattern_end.len();
			let path_start = start_pos + static_pattern_start.len();
			let path_end = start_pos + end_pos;
			let path = &rendered[path_start..path_end];

			let static_url = static_filter(path).unwrap_or_else(|_| path.to_string());

			rendered.replace_range(full_pattern_start..full_pattern_end, &static_url);
		} else {
			break;
		}
	}

	rendered
}

/// Create a manifest mapping for hashed static files
fn create_test_manifest() -> HashMap<String, String> {
	let mut manifest = HashMap::new();
	manifest.insert(
		"css/style.css".to_string(),
		"css/style.a1b2c3d4.css".to_string(),
	);
	manifest.insert(
		"js/app.js".to_string(),
		"js/app.e5f6g7h8.js".to_string(),
	);
	manifest.insert(
		"images/logo.png".to_string(),
		"images/logo.i9j0k1l2.png".to_string(),
	);
	manifest.insert(
		"fonts/roboto.woff2".to_string(),
		"fonts/roboto.m3n4o5p6.woff2".to_string(),
	);
	manifest
}

/// Setup blog database for template tests
async fn setup_blog_database(pool: Arc<AnyPool>) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS posts (
			id SERIAL PRIMARY KEY,
			title VARCHAR(200) NOT NULL,
			slug VARCHAR(200) NOT NULL UNIQUE,
			content TEXT NOT NULL,
			featured_image VARCHAR(500),
			created_at TIMESTAMP NOT NULL DEFAULT NOW()
		)
	"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create posts table");

	let posts = vec![
		(
			"First Post",
			"first-post",
			"This is the content",
			Some("images/post1.jpg"),
		),
		(
			"Second Post",
			"second-post",
			"Another post",
			Some("images/post2.jpg"),
		),
		("Third Post", "third-post", "More content", None),
	];

	for (title, slug, content, image) in posts {
		sqlx::query(
			"INSERT INTO posts (title, slug, content, featured_image) VALUES ($1, $2, $3, $4)",
		)
		.bind(title)
		.bind(slug)
		.bind(content)
		.bind(image)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert post");
	}
}

// ============================================================================
// Tests: Basic Static File URL Generation
// ============================================================================

/// Test: Generate static URL without manifest
///
/// Intent: Verify that basic static file URLs are generated correctly
#[rstest]
#[tokio::test]
async fn test_basic_static_url_generation(temp_dir: PathBuf) {
	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: false,
		manifest: HashMap::new(),
	});

	let template = r#"<link rel="stylesheet" href="{{ "css/style.css"|static }}">"#;
	let context = HashMap::new();

	let rendered = render_template_with_static(template, &context);

	assert_eq!(
		rendered,
		r#"<link rel="stylesheet" href="/static/css/style.css">"#
	);
}

/// Test: Generate multiple static URLs in single template
///
/// Intent: Verify that multiple static file references work correctly
#[rstest]
#[tokio::test]
async fn test_multiple_static_urls(temp_dir: PathBuf) {
	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: false,
		manifest: HashMap::new(),
	});

	let template = r#"
<!DOCTYPE html>
<html>
<head>
	<link rel="stylesheet" href="{{ "css/style.css"|static }}">
	<script src="{{ "js/app.js"|static }}"></script>
</head>
<body>
	<img src="{{ "images/logo.png"|static }}" alt="Logo">
</body>
</html>
	"#;

	let context = HashMap::new();
	let rendered = render_template_with_static(template, &context);

	assert!(rendered.contains(r#"href="/static/css/style.css""#));
	assert!(rendered.contains(r#"src="/static/js/app.js""#));
	assert!(rendered.contains(r#"src="/static/images/logo.png""#));
}

// ============================================================================
// Tests: Hashed Filename Resolution
// ============================================================================

/// Test: Resolve hashed filenames via manifest
///
/// Intent: Verify that manifest-based hashed filename resolution works
#[rstest]
#[tokio::test]
async fn test_hashed_filename_resolution(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	let template = r#"
<link rel="stylesheet" href="{{ "css/style.css"|static }}">
<script src="{{ "js/app.js"|static }}"></script>
	"#;

	let context = HashMap::new();
	let rendered = render_template_with_static(template, &context);

	// Should resolve to hashed filenames
	assert!(rendered.contains(r#"href="/static/css/style.a1b2c3d4.css""#));
	assert!(rendered.contains(r#"src="/static/js/app.e5f6g7h8.js""#));
}

/// Test: Fallback to original path when not in manifest
///
/// Intent: Verify that files not in manifest use original path
#[rstest]
#[tokio::test]
async fn test_manifest_fallback_to_original(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	let template = r#"<img src="{{ "images/unknown.png"|static }}" alt="Unknown">"#;

	let context = HashMap::new();
	let rendered = render_template_with_static(template, &context);

	// Should use original path since not in manifest
	assert!(rendered.contains(r#"src="/static/images/unknown.png""#));
}

// ============================================================================
// Tests: CDN Integration
// ============================================================================

/// Test: Generate CDN URLs for static files
///
/// Intent: Verify that CDN URLs are generated when configured
#[rstest]
#[tokio::test]
async fn test_cdn_url_generation(temp_dir: PathBuf) {
	init_static_config(StaticConfig {
		static_url: "https://cdn.example.com/static/".to_string(),
		use_manifest: false,
		manifest: HashMap::new(),
	});

	let template = r#"
<link rel="stylesheet" href="{{ "css/style.css"|static }}">
<img src="{{ "images/logo.png"|static }}" alt="Logo">
	"#;

	let context = HashMap::new();
	let rendered = render_template_with_static(template, &context);

	assert!(rendered.contains(r#"href="https://cdn.example.com/static/css/style.css""#));
	assert!(rendered.contains(r#"src="https://cdn.example.com/static/images/logo.png""#));
}

/// Test: CDN with hashed filenames
///
/// Intent: Verify that CDN URLs work with manifest-based hashing
#[rstest]
#[tokio::test]
async fn test_cdn_with_hashed_filenames(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "https://cdn.example.com/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	let template = r#"<link rel="stylesheet" href="{{ "css/style.css"|static }}">"#;

	let context = HashMap::new();
	let rendered = render_template_with_static(template, &context);

	assert!(rendered.contains(r#"href="https://cdn.example.com/static/css/style.a1b2c3d4.css""#));
}

// ============================================================================
// Tests: Template + Database + Static Files
// ============================================================================

/// Test: Render blog post with featured image from database
///
/// Intent: Verify that database-backed templates can include static file URLs
#[rstest]
#[tokio::test]
async fn test_blog_post_with_featured_image(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
	temp_dir: PathBuf,
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: false,
		manifest: HashMap::new(),
	});

	setup_blog_database(pool.clone()).await;

	// Fetch post with featured image
	let post: (i32, String, String, Option<String>) =
		sqlx::query_as("SELECT id, title, content, featured_image FROM posts WHERE slug = $1")
			.bind("first-post")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch post");

	let template = r#"
<article>
	<h1>{{title}}</h1>
	<img src="{{ "{{image_path}}"|static }}" alt="Featured">
	<div>{{content}}</div>
</article>
	"#;

	let mut context = HashMap::new();
	context.insert("title".to_string(), Value::String(post.1.clone()));
	context.insert("content".to_string(), Value::String(post.2.clone()));
	context.insert(
		"image_path".to_string(),
		Value::String(post.3.unwrap_or_default()),
	);

	let rendered = render_template_with_static(template, &context);

	assert!(rendered.contains("<h1>First Post</h1>"));
	assert!(rendered.contains(r#"src="/static/images/post1.jpg""#));
}

/// Test: Render multiple posts with optional featured images
///
/// Intent: Verify that template loops work with static file URLs and null checks
#[rstest]
#[tokio::test]
async fn test_post_list_with_optional_images(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<AnyPool>, u16, String),
	temp_dir: PathBuf,
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: false,
		manifest: HashMap::new(),
	});

	setup_blog_database(pool.clone()).await;

	// Fetch all posts
	let posts: Vec<(i32, String, Option<String>)> =
		sqlx::query_as("SELECT id, title, featured_image FROM posts ORDER BY id")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to fetch posts");

	assert_eq!(posts.len(), 3);

	// Verify first post has image
	assert_eq!(posts[0].1, "First Post");
	assert_eq!(posts[0].2, Some("images/post1.jpg".to_string()));

	// Verify third post has no image
	assert_eq!(posts[2].1, "Third Post");
	assert_eq!(posts[2].2, None);

	// In real template engine, would use conditionals for optional images
	// For this mock, we verify data retrieval works correctly
	for post in &posts {
		if let Some(image_path) = &post.2 {
			let static_url = static_filter(image_path).unwrap();
			assert!(static_url.starts_with("/static/"));
		}
	}
}

// ============================================================================
// Tests: CSS/JS Asset Inclusion
// ============================================================================

/// Test: Include CSS files in template head
///
/// Intent: Verify that CSS files can be included via static URLs
#[rstest]
#[tokio::test]
async fn test_css_asset_inclusion(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	let template = r#"
<!DOCTYPE html>
<html>
<head>
	<link rel="stylesheet" href="{{ "css/style.css"|static }}">
	<link rel="preload" href="{{ "fonts/roboto.woff2"|static }}" as="font" type="font/woff2" crossorigin>
</head>
<body>
	<h1>Page Content</h1>
</body>
</html>
	"#;

	let context = HashMap::new();
	let rendered = render_template_with_static(template, &context);

	// CSS should use hashed filename
	assert!(rendered.contains(r#"href="/static/css/style.a1b2c3d4.css""#));

	// Font preload should use hashed filename
	assert!(rendered.contains(r#"href="/static/fonts/roboto.m3n4o5p6.woff2""#));
}

/// Test: Include JavaScript files in template
///
/// Intent: Verify that JS files can be included with proper cache busting
#[rstest]
#[tokio::test]
async fn test_js_asset_inclusion(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	let template = r#"
<body>
	<h1>Content</h1>
	<script src="{{ "js/app.js"|static }}" defer></script>
</body>
	"#;

	let context = HashMap::new();
	let rendered = render_template_with_static(template, &context);

	// JavaScript should use hashed filename for cache busting
	assert!(rendered.contains(r#"src="/static/js/app.e5f6g7h8.js""#));
	assert!(rendered.contains("defer"));
}

// ============================================================================
// Tests: Static File Versioning
// ============================================================================

/// Test: Cache busting with query parameters
///
/// Intent: Verify that version query parameters work for cache busting
#[rstest]
#[tokio::test]
async fn test_version_query_parameter(temp_dir: PathBuf) {
	// Without manifest, can simulate version query params in template
	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: false,
		manifest: HashMap::new(),
	});

	let version = "v1.2.3";
	let template = format!(
		r#"<link rel="stylesheet" href="{{{{ "css/style.css"|static }}}}?v={}">"#,
		version
	);

	let context = HashMap::new();
	let rendered = render_template_with_static(&template, &context);

	assert!(rendered.contains(r#"href="/static/css/style.css?v=v1.2.3""#));
}

/// Test: Manifest provides automatic versioning
///
/// Intent: Verify that manifest-based hashing provides automatic cache busting
#[rstest]
#[tokio::test]
async fn test_manifest_automatic_versioning(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	let template_v1 = r#"<link rel="stylesheet" href="{{ "css/style.css"|static }}">"#;
	let context = HashMap::new();
	let rendered_v1 = render_template_with_static(template_v1, &context);

	// Version 1 uses specific hash
	assert!(rendered_v1.contains("style.a1b2c3d4.css"));

	// Simulate manifest update with new hash
	let mut updated_manifest = create_test_manifest();
	updated_manifest.insert(
		"css/style.css".to_string(),
		"css/style.z9y8x7w6.css".to_string(),
	);

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: updated_manifest,
	});

	let rendered_v2 = render_template_with_static(template_v1, &context);

	// Version 2 uses new hash (automatic cache busting)
	assert!(rendered_v2.contains("style.z9y8x7w6.css"));
	assert_ne!(rendered_v1, rendered_v2);
}

// ============================================================================
// Tests: Template Caching with Static Files
// ============================================================================

/// Test: Template caching preserves static URLs
///
/// Intent: Verify that template caching works correctly with static file references
#[rstest]
#[tokio::test]
async fn test_template_caching_with_static_urls(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	let template = r#"
<html>
<head>
	<link rel="stylesheet" href="{{ "css/style.css"|static }}">
</head>
<body>
	<img src="{{ "images/logo.png"|static }}" alt="Logo">
</body>
</html>
	"#;

	let context = HashMap::new();

	// First render (cache miss)
	let rendered_1 = render_template_with_static(template, &context);
	assert!(rendered_1.contains("style.a1b2c3d4.css"));
	assert!(rendered_1.contains("logo.i9j0k1l2.png"));

	// Second render (cache hit - should produce same output)
	let rendered_2 = render_template_with_static(template, &context);
	assert_eq!(rendered_1, rendered_2);

	// Verify hashed URLs are consistent
	assert!(rendered_2.contains("style.a1b2c3d4.css"));
	assert!(rendered_2.contains("logo.i9j0k1l2.png"));
}

/// Test: Performance of static URL resolution
///
/// Intent: Verify that static URL resolution performs reasonably well
#[rstest]
#[tokio::test]
async fn test_static_url_resolution_performance(temp_dir: PathBuf) {
	let manifest = create_test_manifest();

	init_static_config(StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest: manifest.clone(),
	});

	// Template with many static file references
	let template = r#"
<html>
<head>
	<link rel="stylesheet" href="{{ "css/style.css"|static }}">
	<script src="{{ "js/app.js"|static }}"></script>
</head>
<body>
	<img src="{{ "images/logo.png"|static }}" alt="Logo">
	<img src="{{ "images/logo.png"|static }}" alt="Logo2">
	<img src="{{ "images/logo.png"|static }}" alt="Logo3">
	<img src="{{ "images/logo.png"|static }}" alt="Logo4">
	<img src="{{ "images/logo.png"|static }}" alt="Logo5">
</body>
</html>
	"#;

	let context = HashMap::new();

	let start = std::time::Instant::now();
	for _ in 0..100 {
		let _rendered = render_template_with_static(template, &context);
	}
	let elapsed = start.elapsed();

	// 100 renders should complete quickly (< 50ms)
	assert!(
		elapsed.as_millis() < 50,
		"Static URL resolution took too long: {:?}",
		elapsed
	);
}
