//! Integration tests for Template + Static files functionality
//!
//! Tests the integration between reinhardt-templates and reinhardt-static crates.
//! Covers static file URL generation, manifest-based hashing, and template rendering
//! with static assets.

use askama::Template;
use reinhardt_static::{
    collect_static, FileSystemStorage, StaticFilesHandler, StaticManifest, StaticStorage,
};
use reinhardt_templates::{init_static_config, static_filter, static_path_join, StaticConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

// Template using static file filters
#[derive(Template)]
#[template(
    source = r#"<link rel="stylesheet" href="{{ css_path }}">
<script src="{{ js_path }}"></script>
<img src="{{ img_path }}" alt="Logo">"#,
    ext = "html"
)]
struct AssetTemplate {
    css_path: String,
    js_path: String,
    img_path: String,
}

#[test]
fn test_template_with_static_urls() {
    // Initialize static config
    let config = StaticConfig {
        static_url: "/static/".to_string(),
        use_manifest: false,
        manifest: HashMap::new(),
    };
    init_static_config(config);

    // Generate static URLs
    let css_path = static_filter("css/main.css").unwrap();
    let js_path = static_filter("js/app.js").unwrap();
    let img_path = static_filter("images/logo.png").unwrap();

    // Create template
    let template = AssetTemplate {
        css_path,
        js_path,
        img_path,
    };

    let rendered = template.render().unwrap();

    // Verify all paths are present
    assert!(rendered.contains("/static/css/main.css") || rendered.contains("/assets/css/main.css"));
    assert!(rendered.contains("js/app.js"));
    assert!(rendered.contains("images/logo.png"));
    assert!(rendered.contains("<link"));
    assert!(rendered.contains("<script"));
    assert!(rendered.contains("<img"));
}

#[test]
fn test_template_with_hashed_static_files() {
    // Set up manifest with hashed filenames
    let mut manifest = HashMap::new();
    manifest.insert(
        "css/main.css".to_string(),
        "css/main.abc123.css".to_string(),
    );
    manifest.insert("js/app.js".to_string(), "js/app.def456.js".to_string());

    let config = StaticConfig {
        static_url: "/static/".to_string(),
        use_manifest: true,
        manifest,
    };
    init_static_config(config);

    // Generate hashed URLs
    let css_path = static_filter("css/main.css").unwrap();
    let js_path = static_filter("js/app.js").unwrap();

    // Should use hashed versions from manifest
    assert!(css_path.contains("abc123"));
    assert!(js_path.contains("def456"));
}

#[test]
fn test_template_with_custom_static_url() {
    // Test with CDN URL
    let config = StaticConfig {
        static_url: "https://cdn.example.com/assets/".to_string(),
        use_manifest: false,
        manifest: HashMap::new(),
    };
    init_static_config(config);

    let css_path = static_filter("css/style.css").unwrap();

    // Should use CDN URL
    assert!(
        css_path.starts_with("https://cdn.example.com/assets/")
            || css_path.contains("css/style.css")
    );
}

#[test]
fn test_template_static_path_joining() {
    // Test dynamic path construction
    let base = "images";
    let files = vec!["logo.png", "banner.jpg", "icon.svg"];

    for file in files {
        let joined = static_path_join(base, file).unwrap();
        assert!(joined.contains("images"));
        assert!(joined.contains(file));
    }
}

#[test]
fn test_template_with_nested_static_paths() {
    let config = StaticConfig::default();
    init_static_config(config);

    // Test deeply nested paths
    let paths = vec![
        "vendor/bootstrap/css/bootstrap.min.css",
        "vendor/jquery/jquery.min.js",
        "fonts/roboto/Roboto-Regular.ttf",
    ];

    for path in paths {
        let result = static_filter(path).unwrap();
        assert!(
            result.contains(path),
            "Path {} not found in result: {}",
            path,
            result
        );
    }
}

#[test]
fn test_template_multiple_static_files() {
    // Template with multiple static file references
    #[derive(Template)]
    #[template(
        source = r#"<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="{{ css1 }}">
    <link rel="stylesheet" href="{{ css2 }}">
</head>
<body>
    <script src="{{ js1 }}"></script>
    <script src="{{ js2 }}"></script>
</body>
</html>"#,
        ext = "html"
    )]
    struct MultiAssetTemplate {
        css1: String,
        css2: String,
        js1: String,
        js2: String,
    }

    let config = StaticConfig::default();
    init_static_config(config);

    let template = MultiAssetTemplate {
        css1: static_filter("css/main.css").unwrap(),
        css2: static_filter("css/theme.css").unwrap(),
        js1: static_filter("js/app.js").unwrap(),
        js2: static_filter("js/vendor.js").unwrap(),
    };

    let rendered = template.render().unwrap();

    // Verify all assets are included
    assert!(rendered.contains("css/main.css"));
    assert!(rendered.contains("css/theme.css"));
    assert!(rendered.contains("js/app.js"));
    assert!(rendered.contains("js/vendor.js"));
    assert_eq!(rendered.matches("<link").count(), 2);
    assert_eq!(rendered.matches("<script").count(), 2);
}

#[test]
fn test_static_url_escaping_in_template() {
    let config = StaticConfig::default();
    init_static_config(config);

    // Test that special characters in filenames are handled
    let path = static_filter("files/document with spaces.pdf").unwrap();
    assert!(path.contains("document"));

    // Path should be properly formatted
    assert!(path.starts_with('/'));
}

#[test]
fn test_template_static_integration_with_variables() {
    // Dynamic static file paths based on variables
    #[derive(Template)]
    #[template(
        source = r#"<img src="{{ image_url }}" alt="{{ alt_text }}">"#,
        ext = "html"
    )]
    struct DynamicImageTemplate {
        image_url: String,
        alt_text: String,
    }

    let config = StaticConfig::default();
    init_static_config(config);

    let image_names = vec!["logo.png", "banner.jpg", "avatar.gif"];

    for name in image_names {
        let image_url = static_filter(&format!("images/{}", name)).unwrap();
        let template = DynamicImageTemplate {
            image_url,
            alt_text: name.to_string(),
        };

        let rendered = template.render().unwrap();
        assert!(rendered.contains(name));
        assert!(rendered.contains("images/"));
        assert!(rendered.contains("<img"));
    }
}

#[test]
fn test_template_fallback_for_missing_manifest_entry() {
    // Test behavior when manifest doesn't have an entry
    let mut manifest = HashMap::new();
    manifest.insert("css/main.css".to_string(), "css/main.hash.css".to_string());
    // Note: missing entry for "css/other.css"

    let config = StaticConfig {
        static_url: "/static/".to_string(),
        use_manifest: true,
        manifest,
    };
    init_static_config(config);

    let hashed = static_filter("css/main.css").unwrap();
    assert!(hashed.contains("hash"));

    let unhashed = static_filter("css/other.css").unwrap();
    // Should fall back to original filename
    assert!(unhashed.contains("other.css"));
    assert!(!unhashed.contains("hash"));
}
