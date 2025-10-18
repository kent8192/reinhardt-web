//! Sitemaps integration tests
//!
//! Based on Django's sitemaps tests from:
//! - django/tests/sitemaps_tests/test_generic.py

use chrono::Utc;
use reinhardt_contrib::{ChangeFrequency, Priority, Sitemap, SitemapIndex, SitemapItem};

#[test]
fn test_sitemap_item_creation() {
    let item = SitemapItem::new("https://example.com/page/");

    assert_eq!(item.loc, "https://example.com/page/");
    assert!(item.lastmod.is_none());
    assert!(item.changefreq.is_none());
    assert!(item.priority.is_none());
}

#[test]
fn test_sitemap_item_with_attributes() {
    let now = Utc::now();
    let priority = Priority::new(0.8).unwrap();

    let item = SitemapItem::new("https://example.com/")
        .with_lastmod(now)
        .with_changefreq(ChangeFrequency::Daily)
        .with_priority(priority);

    assert!(item.lastmod.is_some());
    assert_eq!(item.changefreq, Some(ChangeFrequency::Daily));
    assert_eq!(item.priority.unwrap().value(), 0.8);
}

#[test]
fn test_priority_validation() {
    assert!(Priority::new(0.0).is_ok());
    assert!(Priority::new(0.5).is_ok());
    assert!(Priority::new(1.0).is_ok());

    // Invalid priorities
    assert!(Priority::new(-0.1).is_err());
    assert!(Priority::new(1.1).is_err());
    assert!(Priority::new(2.0).is_err());
}

#[test]
fn test_sitemap_item_url_validation() {
    // Valid URLs
    let http_item = SitemapItem::new("http://example.com/");
    assert!(http_item.validate().is_ok());

    let https_item = SitemapItem::new("https://example.com/");
    assert!(https_item.validate().is_ok());

    // Invalid URLs (no protocol)
    let invalid_item = SitemapItem::new("example.com/page");
    assert!(invalid_item.validate().is_err());
}

#[test]
fn test_sitemap_item_url_length_validation() {
    let long_url = format!("https://example.com/{}", "a".repeat(2100));
    let item = SitemapItem::new(long_url);

    assert!(item.validate().is_err());
}

#[test]
fn test_sitemap_creation() {
    let mut sitemap = Sitemap::new();

    sitemap
        .add_item(SitemapItem::new("https://example.com/"))
        .ok();
    sitemap
        .add_item(SitemapItem::new("https://example.com/about/"))
        .ok();

    assert_eq!(sitemap.items.len(), 2);
}

#[test]
fn test_sitemap_xml_generation() {
    let mut sitemap = Sitemap::new();
    sitemap
        .add_item(SitemapItem::new("https://example.com/"))
        .ok();

    let xml = sitemap.to_xml().expect("Failed to generate XML");

    assert!(xml.contains("<?xml"));
    assert!(xml.contains("<urlset"));
    assert!(xml.contains("<loc>https://example.com/</loc>"));
}

#[test]
fn test_sitemap_with_lastmod() {
    let now = Utc::now();
    let item = SitemapItem::new("https://example.com/").with_lastmod(now);

    let xml = item.to_xml().expect("Failed to generate XML");
    assert!(xml.contains("<lastmod>"));
}

#[test]
fn test_sitemap_index_creation() {
    let index = SitemapIndex::new();
    assert_eq!(index.sitemaps.len(), 0);
}

#[test]
fn test_default_priority() {
    let priority = Priority::default();
    assert_eq!(priority.value(), 0.5);
}
