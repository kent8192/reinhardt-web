// Template integration tests for sitemaps

use chrono::{DateTime, NaiveDate, Utc};
use reinhardt_sitemaps::*;

#[derive(Debug)]
struct TestSitemapProvider {
    items: Vec<SitemapItem>,
}

impl SitemapProvider for TestSitemapProvider {
    fn get_sitemap(&self) -> SitemapResult<Sitemap> {
        let mut sitemap = Sitemap::new();
        for item in &self.items {
            sitemap.add_item(item.clone())?;
        }
        Ok(sitemap)
    }
}

#[test]
fn test_sitemap_custom_template() {
    // From test_http.py: test_simple_custom_sitemap
    // Tests sitemap with custom template (with custom comment)
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

    let provider = TestSitemapProvider { items: vec![item] };

    let context = SitemapContext::new()
        .with_custom_content("<!-- This is a customized template -->".to_string());

    let view = SitemapView::new(Box::new(provider)).with_context(context);
    let response = view.render().unwrap();

    // Should have custom comment
    assert!(response
        .content
        .contains("<!-- This is a customized template -->"));

    // Should have standard sitemap content
    assert!(response
        .content
        .contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(response
        .content
        .contains(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9""#));
    assert!(response
        .content
        .contains("<loc>http://example.com/location/</loc>"));
    assert!(response.content.contains("<lastmod>2024-01-01</lastmod>"));
    assert!(response.content.contains("<changefreq>never</changefreq>"));
    assert!(response.content.contains("<priority>0.5</priority>"));

    // Should have proper headers
    assert_eq!(response.status_code, 200);
    assert!(response.has_header("X-Robots-Tag"));
}

#[test]
fn test_sitemap_custom_lastmod_index() {
    // From test_http.py: test_simple_sitemap_custom_lastmod_index
    // Tests sitemap index with custom template
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

    let context = SitemapContext::new()
        .with_custom_content("<!-- This is a customized template -->".to_string());

    let view = SitemapIndexView::new(index)
        .with_latest_lastmod(lastmod)
        .with_context(context);

    let response = view.render().unwrap();

    // Should have custom comment
    assert!(response
        .content
        .contains("<!-- This is a customized template -->"));

    // Should have standard sitemap index content
    assert!(response
        .content
        .contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(response
        .content
        .contains(r#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#));
    assert!(response
        .content
        .contains("<loc>http://example.com/simple/sitemap-simple.xml</loc>"));
    assert!(response.content.contains("<lastmod>2024-01-01T00:00:00"));

    // Should have proper headers
    assert_eq!(response.status_code, 200);
    assert!(response.has_header("X-Robots-Tag"));
    assert!(response.has_header("Last-Modified"));
}

#[test]
fn test_sitemap_default_template() {
    // Tests that default template rendering works without custom context
    let item = SitemapItem::new("https://example.com/page");

    let provider = TestSitemapProvider { items: vec![item] };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should use default rendering
    assert!(response
        .content
        .contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(response
        .content
        .contains("<loc>https://example.com/page</loc>"));
    assert_eq!(response.status_code, 200);
}

#[test]
fn test_sitemap_index_default_template() {
    // Tests that default template rendering works for index
    let mut index = SitemapIndex::new();
    index
        .add_sitemap(SitemapReference::new("https://example.com/sitemap.xml"))
        .unwrap();

    let view = SitemapIndexView::new(index);
    let response = view.render().unwrap();

    // Should use default rendering
    assert!(response
        .content
        .contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(response
        .content
        .contains("<loc>https://example.com/sitemap.xml</loc>"));
    assert_eq!(response.status_code, 200);
}

#[test]
fn test_custom_template_with_multiple_items() {
    // Tests custom template with multiple sitemap items
    let items = vec![
        SitemapItem::new("https://example.com/page1"),
        SitemapItem::new("https://example.com/page2"),
        SitemapItem::new("https://example.com/page3"),
    ];

    let provider = TestSitemapProvider { items };

    let context =
        SitemapContext::new().with_custom_content("<!-- Multi-item sitemap -->".to_string());

    let view = SitemapView::new(Box::new(provider)).with_context(context);
    let response = view.render().unwrap();

    assert!(response.content.contains("<!-- Multi-item sitemap -->"));
    assert!(response
        .content
        .contains("<loc>https://example.com/page1</loc>"));
    assert!(response
        .content
        .contains("<loc>https://example.com/page2</loc>"));
    assert!(response
        .content
        .contains("<loc>https://example.com/page3</loc>"));
}
