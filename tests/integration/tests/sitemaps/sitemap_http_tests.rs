// HTTP integration tests for sitemaps
// These tests verify HTTP response generation without requiring a full server

use chrono::{DateTime, NaiveDate, Utc};
use reinhardt_sitemaps::*;

// Helper to create a test sitemap provider
#[derive(Debug)]
struct TestSitemapProvider {
    items: Vec<SitemapItem>,
    lastmod: Option<DateTime<Utc>>,
}

impl SitemapProvider for TestSitemapProvider {
    fn get_sitemap(&self) -> SitemapResult<Sitemap> {
        let mut sitemap = Sitemap::new();
        for item in &self.items {
            sitemap.add_item(item.clone())?;
        }
        Ok(sitemap)
    }

    fn get_latest_lastmod(&self) -> Option<DateTime<Utc>> {
        self.lastmod
    }
}

#[test]
fn test_sitemap_http_response() {
    // From test_http.py: test_simple_sitemap
    // Tests that sitemap can be served via HTTP with correct headers
    let lastmod = DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    );

    let item = SitemapItem::new("http://example.com/location/")
        .with_lastmod(lastmod)
        .with_changefreq(ChangeFrequency::Never)
        .with_priority(Priority::new(0.5).unwrap());

    let provider = TestSitemapProvider {
        items: vec![item],
        lastmod: Some(lastmod),
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Response status code 200
    assert_eq!(response.status_code, 200);

    // Content-Type: application/xml
    assert_eq!(response.content_type, "application/xml; charset=utf-8");

    // X-Robots-Tag header
    assert!(response.has_header("X-Robots-Tag"));
    assert_eq!(
        response.get_header("X-Robots-Tag").unwrap(),
        "noindex, noodp, noarchive"
    );

    // Proper XML encoding
    assert!(response
        .content
        .contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(response
        .content
        .contains("<loc>http://example.com/location/</loc>"));
    assert!(response.content.contains("<lastmod>2024-01-01</lastmod>"));
    assert!(response.content.contains("<changefreq>never</changefreq>"));
    assert!(response.content.contains("<priority>0.5</priority>"));
}

#[test]
fn test_sitemap_http_index_response() {
    // From test_http.py: test_simple_sitemap_index
    // Tests that sitemap index can be served via HTTP
    let lastmod = DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    );

    let mut index = SitemapIndex::new();
    index
        .add_sitemap(
            SitemapReference::new("http://example.com/simple/sitemap-simple.xml")
                .with_lastmod(lastmod),
        )
        .unwrap();

    let view = SitemapIndexView::new(index).with_latest_lastmod(lastmod);
    let response = view.render().unwrap();

    // Response status code 200
    assert_eq!(response.status_code, 200);

    // Content-Type: application/xml
    assert_eq!(response.content_type, "application/xml; charset=utf-8");

    // X-Robots-Tag header
    assert!(response.has_header("X-Robots-Tag"));

    // Proper XML encoding
    assert!(response
        .content
        .contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(response
        .content
        .contains("<loc>http://example.com/simple/sitemap-simple.xml</loc>"));
    assert!(response.content.contains("<lastmod>2024-01-01T00:00:00"));
}

#[test]
fn test_sitemap_http_no_section() {
    // From test_http.py: test_no_section
    // Tests that accessing non-existent sitemap section returns 404
    let registry = SitemapRegistry::new();

    let result = registry.get_section("simple2");
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("No sitemap available for section: 'simple2'"));
}

#[test]
fn test_sitemap_http_empty_page() {
    // From test_http.py: test_empty_page
    // Tests that requesting page 0 returns 404
    let result = parse_page_param(Some("0"));
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Page 0 empty"));
}

#[test]
fn test_sitemap_http_page_not_int() {
    // From test_http.py: test_page_not_int
    // Tests that invalid page parameter returns 404
    let result = parse_page_param(Some("test"));
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("No page 'test'"));
}

#[test]
fn test_sitemap_http_lastmod_header() {
    // From test_http.py: test_sitemap_last_modified
    // Tests Last-Modified HTTP header is set correctly
    let lastmod = DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2013, 3, 13)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap(),
        Utc,
    );

    let provider = TestSitemapProvider {
        items: vec![SitemapItem::new("http://example.com/location/").with_lastmod(lastmod)],
        lastmod: Some(lastmod),
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response.has_header("Last-Modified"));
    assert_eq!(
        response.get_header("Last-Modified").unwrap(),
        "Wed, 13 Mar 2013 10:00:00 GMT"
    );
}

#[test]
fn test_sitemap_http_lastmod_date() {
    // From test_http.py: test_sitemap_last_modified_date
    // Tests Last-Modified header with date (without time)
    let lastmod = DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2013, 3, 13)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    );

    let provider = TestSitemapProvider {
        items: vec![SitemapItem::new("http://example.com/location/")],
        lastmod: Some(lastmod),
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response.has_header("Last-Modified"));
    assert_eq!(
        response.get_header("Last-Modified").unwrap(),
        "Wed, 13 Mar 2013 00:00:00 GMT"
    );
}

#[test]
fn test_sitemap_http_lastmod_tz() {
    // From test_http.py: test_sitemap_last_modified_tz
    // Tests Last-Modified header converted from timezone aware dates to GMT
    // Note: In Rust, DateTime<Utc> is already in GMT/UTC
    let lastmod = DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2013, 3, 13)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap(),
        Utc,
    );

    let provider = TestSitemapProvider {
        items: vec![SitemapItem::new("http://example.com/location/")],
        lastmod: Some(lastmod),
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response.has_header("Last-Modified"));
    assert_eq!(
        response.get_header("Last-Modified").unwrap(),
        "Wed, 13 Mar 2013 15:00:00 GMT"
    );
}

#[test]
fn test_sitemap_http_lastmod_missing() {
    // From test_http.py: test_sitemap_last_modified_missing
    // Tests Last-Modified header is missing when sitemap has no lastmod
    let provider = TestSitemapProvider {
        items: vec![SitemapItem::new("http://example.com/location/")],
        lastmod: None,
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(!response.has_header("Last-Modified"));
}

#[test]
fn test_sitemap_http_x_robots() {
    // From test_http.py: test_x_robots_sitemap
    // Tests X-Robots-Tag header is set
    let provider = TestSitemapProvider {
        items: vec![SitemapItem::new("http://example.com/location/")],
        lastmod: None,
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response.has_header("X-Robots-Tag"));
    assert_eq!(
        response.get_header("X-Robots-Tag").unwrap(),
        "noindex, noodp, noarchive"
    );

    // Also test for sitemap index
    let index = SitemapIndex::new();
    let index_view = SitemapIndexView::new(index);
    let index_response = index_view.render().unwrap();

    assert!(index_response.has_header("X-Robots-Tag"));
    assert_eq!(
        index_response.get_header("X-Robots-Tag").unwrap(),
        "noindex, noodp, noarchive"
    );
}

#[test]
fn test_sitemap_http_empty_status() {
    // From test_http.py: test_empty_sitemap
    // Tests empty sitemap returns 200
    let provider = TestSitemapProvider {
        items: vec![],
        lastmod: None,
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert_eq!(response.status_code, 200);
    assert!(response.content.contains("<urlset"));
    assert!(response.content.contains("</urlset>"));
}

#[test]
fn test_sitemap_http_format_date() {
    // Tests HTTP date formatting (RFC 7231)
    let dt = DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2013, 3, 13)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap(),
        Utc,
    );

    let formatted = format_http_date(&dt);
    assert_eq!(formatted, "Wed, 13 Mar 2013 10:00:00 GMT");
}

#[test]
fn test_sitemap_http_registry_sections() {
    // Tests sitemap registry for managing multiple sections
    let mut registry = SitemapRegistry::new();

    let provider1 = TestSitemapProvider {
        items: vec![SitemapItem::new("http://example.com/section1/")],
        lastmod: None,
    };

    let provider2 = TestSitemapProvider {
        items: vec![SitemapItem::new("http://example.com/section2/")],
        lastmod: None,
    };

    registry.register("section1".to_string(), Box::new(provider1));
    registry.register("section2".to_string(), Box::new(provider2));

    // Test getting sections
    assert!(registry.get_section("section1").is_ok());
    assert!(registry.get_section("section2").is_ok());
    assert!(registry.get_section("nonexistent").is_err());

    // Test listing sections
    let sections = registry.sections();
    assert_eq!(sections.len(), 2);
    assert!(sections.contains(&"section1".to_string()));
    assert!(sections.contains(&"section2".to_string()));
}
