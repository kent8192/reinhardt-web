//! Real HTTP integration tests for sitemaps
//!
//! These tests verify HTTP-aware sitemap generation including protocol
//! detection and URL scheme adjustment.

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_http::Request;
use reinhardt_sitemaps::{
    adjust_sitemap_protocol, build_absolute_url, create_sitemap_item_from_path, get_base_url,
    Sitemap, SitemapItem,
};

fn create_test_request(is_secure: bool, host: &str) -> Request {
    let mut headers = HeaderMap::new();
    headers.insert("host", host.parse().unwrap());

    let uri: Uri = "/".parse().unwrap();
    let mut request = Request::new(Method::GET, uri, Version::HTTP_11, headers, Bytes::new());
    request.is_secure = is_secure;
    request
}

#[test]
fn test_sitemap_http_protocol_detection() {
    // Test HTTPS request
    let https_request = create_test_request(true, "example.com");
    assert!(https_request.is_secure);

    // Test HTTP request
    let http_request = create_test_request(false, "example.com");
    assert!(!http_request.is_secure);
}

#[test]
fn test_sitemap_protocol_adjustment_https() {
    let mut sitemap = Sitemap::new();
    sitemap
        .add_item(SitemapItem::new("http://example.com/page1"))
        .unwrap();
    sitemap
        .add_item(SitemapItem::new("http://example.com/page2"))
        .unwrap();

    let request = create_test_request(true, "example.com");
    let adjusted = adjust_sitemap_protocol(sitemap, &request).unwrap();

    // All URLs should be converted to HTTPS
    assert_eq!(adjusted.items[0].loc, "https://example.com/page1");
    assert_eq!(adjusted.items[1].loc, "https://example.com/page2");
}

#[test]
fn test_sitemap_protocol_adjustment_http() {
    let mut sitemap = Sitemap::new();
    sitemap
        .add_item(SitemapItem::new("https://example.com/page1"))
        .unwrap();
    sitemap
        .add_item(SitemapItem::new("https://example.com/page2"))
        .unwrap();

    let request = create_test_request(false, "example.com");
    let adjusted = adjust_sitemap_protocol(sitemap, &request).unwrap();

    // All URLs should be converted to HTTP
    assert_eq!(adjusted.items[0].loc, "http://example.com/page1");
    assert_eq!(adjusted.items[1].loc, "http://example.com/page2");
}

#[test]
fn test_get_base_url_from_request() {
    // HTTPS request
    let https_request = create_test_request(true, "example.com");
    assert_eq!(get_base_url(&https_request), "https://example.com");

    // HTTP request
    let http_request = create_test_request(false, "example.com");
    assert_eq!(get_base_url(&http_request), "http://example.com");

    // Different host
    let subdomain_request = create_test_request(true, "www.example.com");
    assert_eq!(get_base_url(&subdomain_request), "https://www.example.com");
}

#[test]
fn test_build_absolute_url_from_path() {
    let request = create_test_request(true, "example.com");

    // Path with leading slash
    let url1 = build_absolute_url("/products/item1", &request);
    assert_eq!(url1, "https://example.com/products/item1");

    // Path without leading slash
    let url2 = build_absolute_url("products/item1", &request);
    assert_eq!(url2, "https://example.com/products/item1");

    // Root path
    let url3 = build_absolute_url("/", &request);
    assert_eq!(url3, "https://example.com/");
}

#[test]
fn test_create_sitemap_item_from_relative_path() {
    let request = create_test_request(true, "example.com");

    let item = create_sitemap_item_from_path("/about", &request);
    assert_eq!(item.loc, "https://example.com/about");

    let item2 = create_sitemap_item_from_path("/contact", &request);
    assert_eq!(item2.loc, "https://example.com/contact");
}

#[test]
fn test_sitemap_with_mixed_protocols() {
    let mut sitemap = Sitemap::new();
    sitemap
        .add_item(SitemapItem::new("http://example.com/page1"))
        .unwrap();
    sitemap
        .add_item(SitemapItem::new("https://example.com/page2"))
        .unwrap();
    sitemap
        .add_item(SitemapItem::new("http://example.com/page3"))
        .unwrap();

    let request = create_test_request(true, "example.com");
    let adjusted = adjust_sitemap_protocol(sitemap, &request).unwrap();

    // All should be HTTPS
    assert_eq!(adjusted.items[0].loc, "https://example.com/page1");
    assert_eq!(adjusted.items[1].loc, "https://example.com/page2");
    assert_eq!(adjusted.items[2].loc, "https://example.com/page3");
}

#[test]
fn test_sitemap_protocol_preservation() {
    let mut sitemap = Sitemap::new();
    sitemap
        .add_item(SitemapItem::new("https://example.com/page1"))
        .unwrap();

    // HTTPS request with HTTPS URLs - should be unchanged
    let request = create_test_request(true, "example.com");
    let adjusted = adjust_sitemap_protocol(sitemap, &request).unwrap();
    assert_eq!(adjusted.items[0].loc, "https://example.com/page1");
}
