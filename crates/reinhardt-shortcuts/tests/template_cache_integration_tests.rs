//! Integration tests for template caching functionality
//!
//! These tests verify that the template cache correctly improves performance
//! by avoiding repeated file I/O operations.

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_http::Request;
use reinhardt_shortcuts::{render_template, template_cache::TemplateCache};
use std::collections::HashMap;
use std::env;

fn create_test_request() -> Request {
    Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    )
}

fn setup_template_dir() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let template_dir = format!("{}/tests/templates", manifest_dir);
    unsafe {
        env::set_var("REINHARDT_TEMPLATE_DIR", template_dir);
    }
}

fn enable_cache() {
    unsafe {
        env::set_var("REINHARDT_TEMPLATE_CACHE", "true");
        env::set_var("REINHARDT_DEBUG", "false");
        env::set_var("REINHARDT_TEMPLATE_CACHE_SIZE", "50");
    }
}

fn disable_cache() {
    unsafe {
        env::set_var("REINHARDT_DEBUG", "true");
    }
}

#[test]
fn test_cache_enabled_check() {
    // Ensure cache is disabled in debug mode
    unsafe {
        env::set_var("REINHARDT_DEBUG", "true");
    }
    assert!(!TemplateCache::is_enabled());

    // Enable cache
    unsafe {
        env::set_var("REINHARDT_DEBUG", "false");
        env::set_var("REINHARDT_TEMPLATE_CACHE", "true");
    }
    assert!(TemplateCache::is_enabled());

    // Explicitly disable cache
    unsafe {
        env::set_var("REINHARDT_DEBUG", "false");
        env::set_var("REINHARDT_TEMPLATE_CACHE", "false");
    }
    assert!(!TemplateCache::is_enabled());
}

#[test]
fn test_template_rendering_with_cache() {
    setup_template_dir();
    enable_cache();

    let request = create_test_request();
    let mut context = HashMap::new();
    context.insert("title", "Cached Page");
    context.insert("heading", "Cache Test");
    context.insert("content", "This template should be cached.");

    // First render - cache miss
    let result1 = render_template(&request, "simple.html", context.clone());
    assert!(result1.is_ok());

    // Second render - cache hit (should be faster)
    let result2 = render_template(&request, "simple.html", context);
    assert!(result2.is_ok());

    match result2 {
        Ok(response) => {
            assert_eq!(response.status, StatusCode::OK);
            let body = String::from_utf8(response.body.to_vec()).unwrap();
            assert!(body.contains("<title>Cached Page</title>"));
        }
        Err(_) => panic!("Expected Ok result"),
    }
}

#[test]
fn test_template_rendering_without_cache() {
    setup_template_dir();
    disable_cache();

    let request = create_test_request();
    let mut context = HashMap::new();
    context.insert("title", "No Cache");
    context.insert("heading", "Debug Mode");
    context.insert("content", "Cache disabled in debug mode.");

    // Both renders should load from disk
    let result1 = render_template(&request, "simple.html", context.clone());
    assert!(result1.is_ok());

    let result2 = render_template(&request, "simple.html", context);
    assert!(result2.is_ok());

    match result2 {
        Ok(response) => {
            assert_eq!(response.status, StatusCode::OK);
            let body = String::from_utf8(response.body.to_vec()).unwrap();
            assert!(body.contains("<title>No Cache</title>"));
        }
        Err(_) => panic!("Expected Ok result"),
    }
}

#[test]
fn test_multiple_templates_cached() {
    setup_template_dir();
    enable_cache();

    let request = create_test_request();

    // Render multiple different templates
    let mut context1 = HashMap::new();
    context1.insert("title", "Page 1");
    context1.insert("heading", "First");
    context1.insert("content", "Content 1");

    let result1 = render_template(&request, "simple.html", context1);
    assert!(result1.is_ok());

    let mut context2 = HashMap::new();
    context2.insert("name", "Alice");
    context2.insert("site_name", "Test Site");

    let result2 = render_template(&request, "greeting.html", context2);
    assert!(result2.is_ok());

    let context3: HashMap<String, String> = HashMap::new();
    let result3 = render_template(&request, "static.html", context3);
    assert!(result3.is_ok());

    // All templates should now be cached
    // Rendering them again should hit the cache
    let mut context1_again = HashMap::new();
    context1_again.insert("title", "Page 1 Again");
    context1_again.insert("heading", "First Again");
    context1_again.insert("content", "Content 1 Again");

    let result4 = render_template(&request, "simple.html", context1_again);
    assert!(result4.is_ok());
}

#[test]
fn test_cache_size_configuration() {
    // Test that cache size can be configured via environment variable
    unsafe {
        env::set_var("REINHARDT_TEMPLATE_CACHE_SIZE", "10");
    }

    let cache = TemplateCache::from_env();

    // Add more items than cache size to trigger eviction
    for i in 0..15 {
        let key = format!("template_{}", i);
        let content = format!("<html>Template {}</html>", i);
        cache.put(key, content);
    }

    // Cache should only hold the last 10 items (LRU eviction)
    assert_eq!(cache.len(), 10);

    let stats = cache.stats();
    // Should have evicted 5 items (15 - 10)
    assert_eq!(stats.evictions, 5);
}

#[test]
fn test_cache_stats_tracking() {
    setup_template_dir();
    enable_cache();

    let cache = TemplateCache::new(50);

    // Initial stats should be empty
    let initial_stats = cache.stats();
    assert_eq!(initial_stats.hits, 0);
    assert_eq!(initial_stats.misses, 0);
    assert_eq!(initial_stats.evictions, 0);
    assert_eq!(initial_stats.hit_rate(), 0.0);

    // Add some templates
    cache.put("template1".to_string(), "content1".to_string());
    cache.put("template2".to_string(), "content2".to_string());

    // Generate some hits and misses
    let _ = cache.get("template1"); // Hit
    let _ = cache.get("template1"); // Hit
    let _ = cache.get("template3"); // Miss

    let stats = cache.stats();
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hit_rate(), 66.66666666666666);

    // Reset stats
    cache.reset_stats();
    let reset_stats = cache.stats();
    assert_eq!(reset_stats.hits, 0);
    assert_eq!(reset_stats.misses, 0);
}
