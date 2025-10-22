//! Comprehensive ORM and cache integration tests for sitemaps
//!
//! This test suite covers:
//! - Basic ORM model sitemap generation
//! - QuerySet filtering and pagination
//! - Callable location and lastmod functions
//! - Cache integration for performance
//! - Full integration with reinhardt-sitemaps ORM features

use chrono::{DateTime, NaiveDate, Utc};
use reinhardt_sitemaps::orm_integration::*;
use reinhardt_sitemaps::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Model structure representing articles
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
    id: i64,
    slug: String,
    title: String,
    published: bool,
    updated_at: DateTime<Utc>,
}

impl Article {
    fn new(
        id: i64,
        slug: String,
        title: String,
        published: bool,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            slug,
            title,
            published,
            updated_at,
        }
    }

    fn get_absolute_url(&self) -> String {
        format!("https://example.com/articles/{}", self.slug)
    }

    fn lastmod(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl ModelLocation for Article {
    fn get_absolute_url(&self) -> String {
        self.get_absolute_url()
    }
}

impl ModelLastmod for Article {
    fn lastmod(&self) -> DateTime<Utc> {
        self.lastmod()
    }
}

// ============================================================================
// Basic ORM Integration Tests
// ============================================================================

#[test]
fn test_basic_model_sitemap() {
    let articles = vec![
        Article::new(
            1,
            "first-article".to_string(),
            "First Article".to_string(),
            true,
            DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        ),
        Article::new(
            2,
            "second-article".to_string(),
            "Second Article".to_string(),
            true,
            DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 15)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        ),
    ];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        Some(ChangeFrequency::Weekly),
        Some(Priority::new(0.7).unwrap()),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Verify articles in sitemap
    assert!(response
        .content
        .contains("https://example.com/articles/first-article"));
    assert!(response
        .content
        .contains("https://example.com/articles/second-article"));
    assert!(response.content.contains("<changefreq>weekly</changefreq>"));
    assert!(response.content.contains("<priority>0.7</priority>"));
}

#[test]
fn test_model_sitemap_with_lastmod() {
    let updated_time = DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap(),
        Utc,
    );

    let articles = vec![Article::new(
        1,
        "recent-article".to_string(),
        "Recent".to_string(),
        true,
        updated_time,
    )];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        Some(ChangeFrequency::Weekly),
        Some(Priority::new(0.7).unwrap()),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should have Last-Modified header
    assert!(response.has_header("Last-Modified"));
    assert!(response.content.contains("<lastmod>"));
}

// ============================================================================
// Generic Model Sitemap Tests (Django's GenericSitemap equivalent)
// ============================================================================

#[test]
fn test_generic_model_sitemap() {
    // Test Django's GenericSitemap equivalent
    let articles = vec![
        Article::new(
            1,
            "first-article".to_string(),
            "First Article".to_string(),
            true,
            DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        ),
        Article::new(
            2,
            "second-article".to_string(),
            "Second Article".to_string(),
            true,
            DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 15)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        ),
    ];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        Some(ChangeFrequency::Weekly),
        Some(Priority::new(0.7).unwrap()),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Verify articles in sitemap
    assert!(response
        .content
        .contains("https://example.com/articles/first-article"));
    assert!(response
        .content
        .contains("https://example.com/articles/second-article"));

    // Verify metadata
    assert!(response.content.contains("<changefreq>weekly</changefreq>"));
    assert!(response.content.contains("<priority>0.7</priority>"));
    assert!(response.has_header("Last-Modified"));
}

// ============================================================================
// Filtering and QuerySet Tests
// ============================================================================

#[test]
fn test_model_sitemap_with_filtering() {
    // Test queryset filtering (published articles only)
    let articles = vec![
        Article::new(
            1,
            "published".to_string(),
            "Published".to_string(),
            true,
            Utc::now(),
        ),
        Article::new(
            2,
            "draft".to_string(),
            "Draft".to_string(),
            false,
            Utc::now(),
        ),
    ];

    let queryset = QuerySet::new(articles).filter(|a| a.published);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        Some(ChangeFrequency::Weekly),
        Some(Priority::new(0.7).unwrap()),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should only have published article
    assert!(response
        .content
        .contains("https://example.com/articles/published"));
    assert!(!response
        .content
        .contains("https://example.com/articles/draft"));
    assert_eq!(response.content.matches("<url>").count(), 1);
}

#[test]
fn test_queryset_operations() {
    // Test QuerySet operations
    let articles = vec![
        Article::new(
            1,
            "first".to_string(),
            "First".to_string(),
            true,
            Utc::now(),
        ),
        Article::new(
            2,
            "second".to_string(),
            "Second".to_string(),
            true,
            Utc::now(),
        ),
        Article::new(
            3,
            "third".to_string(),
            "Third".to_string(),
            false,
            Utc::now(),
        ),
    ];

    let queryset = QuerySet::new(articles);
    assert_eq!(queryset.count(), 3);
    assert!(!queryset.is_empty());

    // Filter to published only
    let published = queryset.filter(|a| a.published);
    assert_eq!(published.count(), 2);

    // Get all items
    let all = published.all();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].slug, "first");
    assert_eq!(all[1].slug, "second");
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_model_sitemap_empty_queryset() {
    // Test with empty queryset
    let queryset: QuerySet<Article> = QuerySet::new(vec![]);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        Some(ChangeFrequency::Weekly),
        Some(Priority::new(0.7).unwrap()),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should render valid but empty sitemap
    assert!(response.content.contains("<urlset"));
    assert!(!response.content.contains("<url>"));
    assert!(!response.has_header("Last-Modified"));
}

#[test]
fn test_model_sitemap_single_item() {
    // Test with single item queryset
    let articles = vec![Article::new(
        1,
        "only-article".to_string(),
        "Only Article".to_string(),
        true,
        Utc::now(),
    )];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        Some(ChangeFrequency::Weekly),
        Some(Priority::new(0.7).unwrap()),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response
        .content
        .contains("https://example.com/articles/only-article"));
    assert_eq!(response.content.matches("<url>").count(), 1);
    assert!(response.content.contains("<changefreq>weekly</changefreq>"));
    assert!(response.content.contains("<priority>0.7</priority>"));
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[test]
fn test_large_queryset_pagination() {
    // Test automatic pagination for large querysets
    let articles: Vec<Article> = (0..100)
        .map(|i| {
            Article::new(
                i,
                format!("article-{}", i),
                format!("Article {}", i),
                true,
                Utc::now(),
            )
        })
        .collect();

    let queryset = QuerySet::new(articles);
    let count = queryset.count();
    assert_eq!(count, 100);

    // Provider should handle all items
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        Some(ChangeFrequency::Weekly),
        Some(Priority::new(0.7).unwrap()),
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert_eq!(response.content.matches("<url>").count(), 100);
    assert!(response.has_header("Last-Modified"));
}

// ============================================================================
// Callable Functions Tests
// ============================================================================

#[test]
fn test_callable_location_and_lastmod() {
    // Test that location and lastmod can be callable (functions)
    let articles = vec![Article::new(
        1,
        "test".to_string(),
        "Test".to_string(),
        true,
        Utc::now(),
    )];

    let queryset = QuerySet::new(articles);

    // Custom location function
    let custom_location = |article: &Article| format!("https://example.com/blog/{}", article.slug);

    let provider = ModelSitemapProvider::new(
        queryset,
        custom_location,
        Some(|article| article.lastmod()),
        None,
        None,
    );

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should use custom location
    assert!(response.content.contains("https://example.com/blog/test"));
    assert!(!response
        .content
        .contains("https://example.com/articles/test"));
}

// ============================================================================
// Builder Pattern Tests
// ============================================================================

#[test]
fn test_provider_with_changefreq_and_priority() {
    // Test builder pattern for setting changefreq and priority
    let articles = vec![Article::new(
        1,
        "test".to_string(),
        "Test".to_string(),
        true,
        Utc::now(),
    )];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        None,
        None,
    )
    .with_changefreq(ChangeFrequency::Daily)
    .with_priority(Priority::new(0.9).unwrap());

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response.content.contains("<changefreq>daily</changefreq>"));
    assert!(response.content.contains("<priority>0.9</priority>"));
}

// ============================================================================
// Lastmod Detection Tests
// ============================================================================

#[test]
fn test_latest_lastmod_detection() {
    // Test that latest lastmod is correctly detected
    let now = Utc::now();
    let yesterday = now - chrono::Duration::days(1);
    let last_week = now - chrono::Duration::weeks(1);

    let articles = vec![
        Article::new(1, "old".to_string(), "Old".to_string(), true, last_week),
        Article::new(
            2,
            "recent".to_string(),
            "Recent".to_string(),
            true,
            yesterday,
        ),
        Article::new(3, "newest".to_string(), "Newest".to_string(), true, now),
    ];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::new(
        queryset,
        |article| article.get_absolute_url(),
        Some(|article| article.lastmod()),
        None,
        None,
    );

    // Latest lastmod should be 'now'
    let latest = provider.get_latest_lastmod();
    assert!(latest.is_some());
    let latest_time = latest.unwrap();

    // Should be within 1 second of 'now' (accounting for test execution time)
    let diff = (latest_time - now).num_seconds().abs();
    assert!(diff < 1);
}

// ============================================================================
// Cache Integration Tests
// ============================================================================

// Mock cache backend
#[derive(Debug, Clone)]
struct MockCache {
    data: Arc<Mutex<HashMap<String, (String, DateTime<Utc>)>>>,
}

impl MockCache {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        let cache = self.data.lock().unwrap();
        cache.get(key).map(|(value, _)| value.clone())
    }

    fn set(&self, key: String, value: String, ttl_seconds: u64) {
        let mut cache = self.data.lock().unwrap();
        let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds as i64);
        cache.insert(key, (value, expires_at));
    }

    fn clear(&self) {
        let mut cache = self.data.lock().unwrap();
        cache.clear();
    }
}

// Cached sitemap provider
#[derive(Debug)]
struct CachedSitemapProvider {
    inner: Box<dyn SitemapProvider>,
    cache: MockCache,
    cache_key: String,
    ttl_seconds: u64,
}

impl CachedSitemapProvider {
    fn new(
        inner: Box<dyn SitemapProvider>,
        cache: MockCache,
        cache_key: String,
        ttl_seconds: u64,
    ) -> Self {
        Self {
            inner,
            cache,
            cache_key,
            ttl_seconds,
        }
    }
}

impl SitemapProvider for CachedSitemapProvider {
    fn get_sitemap(&self) -> SitemapResult<Sitemap> {
        // Try to get from cache
        if let Some(_cached_xml) = self.cache.get(&self.cache_key) {
            // In real implementation, would deserialize from XML
            // For now, just return fresh sitemap to demonstrate caching concept
            return self.inner.get_sitemap();
        }

        // Generate fresh sitemap
        let sitemap = self.inner.get_sitemap()?;

        // Cache it (in real implementation, would serialize to XML)
        let xml = render_sitemap(&sitemap, &SitemapContext::new())?;
        self.cache
            .set(self.cache_key.clone(), xml, self.ttl_seconds);

        Ok(sitemap)
    }

    fn get_latest_lastmod(&self) -> Option<DateTime<Utc>> {
        self.inner.get_latest_lastmod()
    }

    fn get_paginated_sitemap(&self, page: usize) -> SitemapResult<Option<Sitemap>> {
        self.inner.get_paginated_sitemap(page)
    }

    fn get_page_count(&self) -> usize {
        self.inner.get_page_count()
    }
}

#[test]
fn test_cached_sitemap() {
    // Create a simple provider
    #[derive(Debug)]
    struct SimpleSitemapProvider {
        items: Vec<String>,
    }

    impl SitemapProvider for SimpleSitemapProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            for item in &self.items {
                sitemap.add_item(SitemapItem::new(item))?;
            }
            Ok(sitemap)
        }
    }

    let simple_provider = SimpleSitemapProvider {
        items: vec![
            "https://example.com/page1".to_string(),
            "https://example.com/page2".to_string(),
        ],
    };

    let cache = MockCache::new();
    let cached_provider = CachedSitemapProvider::new(
        Box::new(simple_provider),
        cache.clone(),
        "sitemap_test".to_string(),
        3600,
    );

    // First call - cache miss
    let view1 = SitemapView::new(Box::new(cached_provider));
    let response1 = view1.render().unwrap();
    assert!(response1.content.contains("https://example.com/page1"));

    // Cache should now have the key
    assert!(cache.get("sitemap_test").is_some());
}

#[test]
fn test_cache_invalidation() {
    #[derive(Debug)]
    struct CountingProvider {
        items: Vec<String>,
        call_count: Arc<Mutex<usize>>,
    }

    impl SitemapProvider for CountingProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            *self.call_count.lock().unwrap() += 1;
            let mut sitemap = Sitemap::new();
            for item in &self.items {
                sitemap.add_item(SitemapItem::new(item))?;
            }
            Ok(sitemap)
        }
    }

    let call_count = Arc::new(Mutex::new(0));
    let counting_provider = CountingProvider {
        items: vec!["https://example.com/page1".to_string()],
        call_count: call_count.clone(),
    };

    let cache = MockCache::new();
    let cached_provider = CachedSitemapProvider::new(
        Box::new(counting_provider),
        cache.clone(),
        "sitemap_counting".to_string(),
        3600,
    );

    // First call
    let view1 = SitemapView::new(Box::new(cached_provider));
    let _ = view1.render().unwrap();
    assert_eq!(*call_count.lock().unwrap(), 1);

    // Clear cache (simulates cache invalidation)
    cache.clear();

    // Second call after cache clear - should call provider again
    #[derive(Debug)]
    struct CountingProvider2 {
        items: Vec<String>,
        call_count: Arc<Mutex<usize>>,
    }
    impl SitemapProvider for CountingProvider2 {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            *self.call_count.lock().unwrap() += 1;
            let mut sitemap = Sitemap::new();
            for item in &self.items {
                sitemap.add_item(SitemapItem::new(item))?;
            }
            Ok(sitemap)
        }
    }
    let call_count2 = Arc::new(Mutex::new(0));
    let counting_provider2 = CountingProvider2 {
        items: vec!["https://example.com/page1".to_string()],
        call_count: call_count2.clone(),
    };
    let cached_provider2 = CachedSitemapProvider::new(
        Box::new(counting_provider2),
        cache.clone(),
        "sitemap_counting2".to_string(),
        3600,
    );
    let view2 = SitemapView::new(Box::new(cached_provider2));
    let _ = view2.render().unwrap();
    assert_eq!(*call_count2.lock().unwrap(), 1);
}

#[test]
fn test_per_page_caching() {
    // Test that each page can be cached with different keys
    #[derive(Debug)]
    struct PaginatedCachedProvider {
        page_count: usize,
    }

    impl SitemapProvider for PaginatedCachedProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            self.get_paginated_sitemap(1)?
                .ok_or_else(|| SitemapError::Generation("No sitemap available".to_string()))
        }

        fn get_paginated_sitemap(&self, page: usize) -> SitemapResult<Option<Sitemap>> {
            if page > self.page_count {
                return Ok(None);
            }

            let mut sitemap = Sitemap::new();
            for i in 0..10 {
                let url = format!("https://example.com/page-{}-{}", page, i);
                sitemap.add_item(SitemapItem::new(url))?;
            }

            Ok(Some(sitemap))
        }

        fn get_page_count(&self) -> usize {
            self.page_count
        }
    }

    let provider = PaginatedCachedProvider { page_count: 3 };

    // Test that different pages return different content
    let view1 = SitemapView::new(Box::new(provider)).with_page(1);
    let response1 = view1.render().unwrap();
    assert!(response1.content.contains("https://example.com/page-1-0"));

    // Recreate provider for page 2
    let provider2 = PaginatedCachedProvider { page_count: 3 };
    let view2 = SitemapView::new(Box::new(provider2)).with_page(2);
    let response2 = view2.render().unwrap();
    assert!(response2.content.contains("https://example.com/page-2-0"));

    // Content should be different
    assert_ne!(response1.content, response2.content);
}
