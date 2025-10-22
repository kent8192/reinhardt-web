//! Advanced sitemap tests with real implementations
//!
//! This file replaces sitemap_advanced_tests.rs with real HTTP integration

use bytes::Bytes;
use chrono::{DateTime, NaiveDate, Utc};
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_http::Request;
use reinhardt_sitemaps::*;

fn create_test_request(is_secure: bool, host: &str) -> Request {
    let mut headers = HeaderMap::new();
    headers.insert("host", host.parse().unwrap());

    let uri: Uri = "/".parse().unwrap();
    let mut request = Request::new(Method::GET, uri, Version::HTTP_11, headers, Bytes::new());
    request.is_secure = is_secure;
    request
}

// Provider that generates URLs from request context
#[derive(Debug)]
struct RequestAwareSitemapProvider {
    paths: Vec<String>,
}

impl RequestAwareSitemapProvider {
    fn new(paths: Vec<String>) -> Self {
        Self { paths }
    }

    fn get_sitemap_for_request(&self, request: &Request) -> SitemapResult<Sitemap> {
        let mut sitemap = Sitemap::new();
        for path in &self.paths {
            let item = create_sitemap_item_from_path(path, request);
            sitemap.add_item(item)?;
        }
        Ok(sitemap)
    }
}

impl SitemapProvider for RequestAwareSitemapProvider {
    fn get_sitemap(&self) -> SitemapResult<Sitemap> {
        // Default implementation for tests
        let mut sitemap = Sitemap::new();
        for path in &self.paths {
            let url = format!("https://example.com{}", path);
            sitemap.add_item(SitemapItem::new(url))?;
        }
        Ok(sitemap)
    }
}

#[test]
fn test_sitemap_with_https_request() {
    let request = create_test_request(true, "example.com");
    let provider =
        RequestAwareSitemapProvider::new(vec!["/page1".to_string(), "/page2".to_string()]);

    let sitemap = provider.get_sitemap_for_request(&request).unwrap();

    // Create a provider that returns the pre-built sitemap
    #[derive(Debug)]
    struct StaticProvider(Sitemap);
    impl SitemapProvider for StaticProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            Ok(self.0.clone())
        }
    }

    let view = SitemapView::new(Box::new(StaticProvider(sitemap)));
    let response = view.render().unwrap();

    // Verify all URLs use HTTPS
    assert!(response.content.contains("https://example.com/page1"));
    assert!(response.content.contains("https://example.com/page2"));
    assert!(!response.content.contains("http://example.com/page1"));
}

#[test]
fn test_sitemap_with_http_request() {
    let request = create_test_request(false, "example.com");
    let provider =
        RequestAwareSitemapProvider::new(vec!["/page1".to_string(), "/page2".to_string()]);

    let sitemap = provider.get_sitemap_for_request(&request).unwrap();

    #[derive(Debug)]
    struct StaticProvider(Sitemap);
    impl SitemapProvider for StaticProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            Ok(self.0.clone())
        }
    }

    let view = SitemapView::new(Box::new(StaticProvider(sitemap)));
    let response = view.render().unwrap();

    // Verify all URLs use HTTP
    assert!(response.content.contains("http://example.com/page1"));
    assert!(response.content.contains("http://example.com/page2"));
    assert!(!response.content.contains("https://"));
}

#[test]
fn test_sitemap_index_with_https_request() {
    let request = create_test_request(true, "example.com");

    let mut index = SitemapIndex::new();

    // Build URLs using the request context
    let base_url = get_base_url(&request);
    index
        .add_sitemap(SitemapReference::new(format!(
            "{}/sitemap-posts.xml",
            base_url
        )))
        .unwrap();
    index
        .add_sitemap(SitemapReference::new(format!(
            "{}/sitemap-pages.xml",
            base_url
        )))
        .unwrap();

    let view = SitemapIndexView::new(index);
    let response = view.render().unwrap();

    // Verify all sitemap URLs use HTTPS
    assert!(response
        .content
        .contains("https://example.com/sitemap-posts.xml"));
    assert!(response
        .content
        .contains("https://example.com/sitemap-pages.xml"));
}

#[test]
fn test_sitemap_protocol_adjustment() {
    // Create sitemap with HTTP URLs
    let mut sitemap = Sitemap::new();
    sitemap
        .add_item(SitemapItem::new("http://example.com/page1"))
        .unwrap();
    sitemap
        .add_item(SitemapItem::new("http://example.com/page2"))
        .unwrap();

    // Adjust to HTTPS based on request
    let https_request = create_test_request(true, "example.com");
    let adjusted = adjust_sitemap_protocol(sitemap, &https_request).unwrap();

    #[derive(Debug)]
    struct StaticProvider(Sitemap);
    impl SitemapProvider for StaticProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            Ok(self.0.clone())
        }
    }

    let view = SitemapView::new(Box::new(StaticProvider(adjusted)));
    let response = view.render().unwrap();

    assert!(response.content.contains("https://example.com/page1"));
    assert!(response.content.contains("https://example.com/page2"));
}

// Test for empty page handling
#[test]
fn test_empty_page_handling() {
    #[derive(Debug)]
    struct EmptyPageProvider;

    impl SitemapProvider for EmptyPageProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            sitemap.add_item(SitemapItem::new("https://example.com/page1"))?;
            Ok(sitemap)
        }

        fn get_paginated_sitemap(&self, page: usize) -> SitemapResult<Option<Sitemap>> {
            if page == 1 {
                Ok(Some(self.get_sitemap()?))
            } else {
                Ok(None)
            }
        }

        fn get_page_count(&self) -> usize {
            1
        }
    }

    let provider = EmptyPageProvider;

    // Page 1 should work
    let view1 = SitemapView::new(Box::new(provider)).with_page(1);
    assert!(view1.render().is_ok());

    #[derive(Debug)]
    struct EmptyPageProvider2;

    impl SitemapProvider for EmptyPageProvider2 {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            Sitemap::new().with_item(SitemapItem::new("https://example.com/page1"))
        }

        fn get_paginated_sitemap(&self, page: usize) -> SitemapResult<Option<Sitemap>> {
            if page == 1 {
                Ok(Some(self.get_sitemap()?))
            } else {
                Ok(None)
            }
        }

        fn get_page_count(&self) -> usize {
            1
        }
    }

    // Page 2 should fail with appropriate error
    let view2 = SitemapView::new(Box::new(EmptyPageProvider2)).with_page(2);
    let result = view2.render();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Page 2 empty"));
}

#[test]
fn test_page_not_int_handling() {
    let result = parse_page_param(Some("abc"));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No page 'abc'"));

    let result = parse_page_param(Some("1.5"));
    assert!(result.is_err());

    let result = parse_page_param(Some(""));
    assert!(result.is_err());
}

#[test]
fn test_no_section_error() {
    let registry = SitemapRegistry::new();

    let result = registry.get_section("nonexistent");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No sitemap available for section: 'nonexistent'"));
}

#[test]
fn test_static_sitemap_configuration() {
    #[derive(Debug)]
    struct StaticSitemapProvider {
        sitemap: Sitemap,
    }

    impl StaticSitemapProvider {
        fn new(items: Vec<SitemapItem>) -> Self {
            let mut sitemap = Sitemap::new();
            for item in items {
                sitemap.add_item(item).unwrap();
            }
            Self { sitemap }
        }
    }

    impl SitemapProvider for StaticSitemapProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            Ok(self.sitemap.clone())
        }
    }

    let provider = StaticSitemapProvider::new(vec![
        SitemapItem::new("https://example.com/about"),
        SitemapItem::new("https://example.com/contact"),
        SitemapItem::new("https://example.com/privacy"),
    ]);

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response.content.contains("https://example.com/about"));
    assert!(response.content.contains("https://example.com/contact"));
    assert!(response.content.contains("https://example.com/privacy"));
}

#[test]
fn test_callable_sitemap_configuration() {
    #[derive(Debug)]
    struct CallableSitemapProvider {
        generator: fn() -> Vec<String>,
    }

    impl CallableSitemapProvider {
        fn new(generator: fn() -> Vec<String>) -> Self {
            Self { generator }
        }
    }

    impl SitemapProvider for CallableSitemapProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            let urls = (self.generator)();
            for url in urls {
                sitemap.add_item(SitemapItem::new(url))?;
            }
            Ok(sitemap)
        }
    }

    fn generate_urls() -> Vec<String> {
        vec![
            "https://example.com/page1".to_string(),
            "https://example.com/page2".to_string(),
            "https://example.com/page3".to_string(),
        ]
    }

    let provider = CallableSitemapProvider::new(generate_urls);
    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response.content.contains("https://example.com/page1"));
    assert!(response.content.contains("https://example.com/page2"));
    assert!(response.content.contains("https://example.com/page3"));
}

#[test]
fn test_callable_lastmod_simulation() {
    #[derive(Debug, Clone)]
    struct Article {
        slug: String,
        updated_at: DateTime<Utc>,
    }

    impl Article {
        fn get_absolute_url(&self) -> String {
            format!("https://example.com/articles/{}", self.slug)
        }

        fn lastmod(&self) -> DateTime<Utc> {
            self.updated_at
        }
    }

    #[derive(Debug)]
    struct ArticleSitemapProvider {
        articles: Vec<Article>,
    }

    impl SitemapProvider for ArticleSitemapProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            for article in &self.articles {
                let item = SitemapItem::new(article.get_absolute_url())
                    .with_lastmod(article.lastmod())
                    .with_changefreq(ChangeFrequency::Weekly)
                    .with_priority(Priority::new(0.8)?);
                sitemap.add_item(item)?;
            }
            Ok(sitemap)
        }

        fn get_latest_lastmod(&self) -> Option<DateTime<Utc>> {
            self.articles.iter().map(|a| a.lastmod()).max()
        }
    }

    let articles = vec![
        Article {
            slug: "first-post".to_string(),
            updated_at: DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        },
        Article {
            slug: "second-post".to_string(),
            updated_at: DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 15)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        },
    ];

    let provider = ArticleSitemapProvider { articles };
    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response
        .content
        .contains("https://example.com/articles/first-post"));
    assert!(response
        .content
        .contains("https://example.com/articles/second-post"));
    assert!(response.has_header("Last-Modified"));
}

#[test]
fn test_callable_lastmod_partial() {
    #[derive(Debug, Clone)]
    struct Page {
        url: String,
        updated_at: Option<DateTime<Utc>>,
    }

    #[derive(Debug)]
    struct PageSitemapProvider {
        pages: Vec<Page>,
    }

    impl SitemapProvider for PageSitemapProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            for page in &self.pages {
                let mut item = SitemapItem::new(&page.url);
                if let Some(updated) = page.updated_at {
                    item = item.with_lastmod(updated);
                }
                sitemap.add_item(item)?;
            }
            Ok(sitemap)
        }
    }

    let pages = vec![
        Page {
            url: "https://example.com/page1".to_string(),
            updated_at: Some(Utc::now()),
        },
        Page {
            url: "https://example.com/page2".to_string(),
            updated_at: None,
        },
    ];

    let provider = PageSitemapProvider { pages };
    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(!response.has_header("Last-Modified"));
}

#[test]
fn test_callable_lastmod_no_items() {
    #[derive(Debug)]
    struct EmptyProvider;

    impl SitemapProvider for EmptyProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            Ok(Sitemap::new())
        }

        fn get_latest_lastmod(&self) -> Option<DateTime<Utc>> {
            None
        }
    }

    let provider = EmptyProvider;
    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(!response.has_header("Last-Modified"));
    assert!(response.content.contains("<urlset"));
}

#[test]
fn test_custom_template_simulation() {
    let mut sitemap = Sitemap::new();
    sitemap
        .add_item(SitemapItem::new("https://example.com/page1"))
        .unwrap();

    let context = SitemapContext::new().with_custom_content(
        "<!-- Generated by Custom Sitemap System -->\n<!-- Version 1.0 -->\n".to_string(),
    );

    let xml = render_sitemap(&sitemap, &context).unwrap();

    assert!(xml.contains("<!-- Generated by Custom Sitemap System -->"));
    assert!(xml.contains("<!-- Version 1.0 -->"));
    assert!(xml.contains("https://example.com/page1"));
}

#[test]
fn test_cache_ready_sitemap() {
    #[derive(Debug)]
    struct CacheReadyProvider {
        items: Vec<SitemapItem>,
        cache_key: String,
    }

    impl CacheReadyProvider {
        fn new(items: Vec<SitemapItem>) -> Self {
            Self {
                cache_key: format!("sitemap_v1_{}", items.len()),
                items,
            }
        }

        fn get_cache_key(&self) -> &str {
            &self.cache_key
        }
    }

    impl SitemapProvider for CacheReadyProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            for item in &self.items {
                sitemap.add_item(item.clone())?;
            }
            Ok(sitemap)
        }
    }

    let items = vec![
        SitemapItem::new("https://example.com/page1"),
        SitemapItem::new("https://example.com/page2"),
    ];

    let provider = CacheReadyProvider::new(items);
    assert_eq!(provider.get_cache_key(), "sitemap_v1_2");

    let view1 = SitemapView::new(Box::new(provider));
    let response1 = view1.render().unwrap();

    let items2 = vec![
        SitemapItem::new("https://example.com/page1"),
        SitemapItem::new("https://example.com/page2"),
    ];
    let provider2 = CacheReadyProvider::new(items2);
    let view2 = SitemapView::new(Box::new(provider2));
    let response2 = view2.render().unwrap();

    assert_eq!(response1.content, response2.content);
}
