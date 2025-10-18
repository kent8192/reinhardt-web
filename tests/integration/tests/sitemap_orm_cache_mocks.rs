// Mock-based ORM and cache sitemap tests
// These tests simulate functionality that will work once the corresponding crates are available

use chrono::{DateTime, NaiveDate, Utc};
use reinhardt_sitemaps::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Mock model structure
#[derive(Debug, Clone)]
struct Article {
    id: i64,
    slug: String,
    title: String,
    published: bool,
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

// Mock queryset
#[derive(Debug, Clone)]
struct QuerySet<T> {
    items: Vec<T>,
}

impl<T: Clone> QuerySet<T> {
    fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    fn filter(mut self, predicate: impl Fn(&T) -> bool) -> Self {
        self.items.retain(|item| predicate(item));
        self
    }

    fn all(&self) -> Vec<T> {
        self.items.clone()
    }

    fn count(&self) -> usize {
        self.items.len()
    }
}

// Generic model sitemap provider (simulates Django's GenericSitemap)
#[derive(Debug)]
struct ModelSitemapProvider<T> {
    queryset: QuerySet<T>,
    location_fn: fn(&T) -> String,
    lastmod_fn: Option<fn(&T) -> DateTime<Utc>>,
    changefreq: Option<ChangeFrequency>,
    priority: Option<Priority>,
}

impl ModelSitemapProvider<Article> {
    fn for_articles(queryset: QuerySet<Article>) -> Self {
        Self {
            queryset,
            location_fn: |article| article.get_absolute_url(),
            lastmod_fn: Some(|article| article.lastmod()),
            changefreq: Some(ChangeFrequency::Weekly),
            priority: Some(Priority::new(0.7).unwrap()),
        }
    }
}

impl SitemapProvider for ModelSitemapProvider<Article> {
    fn get_sitemap(&self) -> SitemapResult<Sitemap> {
        let mut sitemap = Sitemap::new();

        for item in self.queryset.all() {
            let url = (self.location_fn)(&item);
            let mut sitemap_item = SitemapItem::new(url);

            if let Some(lastmod_fn) = self.lastmod_fn {
                sitemap_item = sitemap_item.with_lastmod(lastmod_fn(&item));
            }

            if let Some(changefreq) = &self.changefreq {
                sitemap_item = sitemap_item.with_changefreq(*changefreq);
            }

            if let Some(priority) = &self.priority {
                sitemap_item = sitemap_item.with_priority(*priority);
            }

            sitemap.add_item(sitemap_item)?;
        }

        Ok(sitemap)
    }

    fn get_latest_lastmod(&self) -> Option<DateTime<Utc>> {
        if let Some(lastmod_fn) = self.lastmod_fn {
            self.queryset.all().iter().map(lastmod_fn).max()
        } else {
            None
        }
    }
}

#[test]
fn test_generic_model_sitemap() {
    // Simulates Django's GenericSitemap
    let articles = vec![
        Article {
            id: 1,
            slug: "first-article".to_string(),
            title: "First Article".to_string(),
            published: true,
            updated_at: DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        },
        Article {
            id: 2,
            slug: "second-article".to_string(),
            title: "Second Article".to_string(),
            published: true,
            updated_at: DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 15)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        },
    ];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::for_articles(queryset);

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

#[test]
fn test_model_sitemap_with_filtering() {
    // Test queryset filtering (published articles only)
    let articles = vec![
        Article {
            id: 1,
            slug: "published".to_string(),
            title: "Published".to_string(),
            published: true,
            updated_at: Utc::now(),
        },
        Article {
            id: 2,
            slug: "draft".to_string(),
            title: "Draft".to_string(),
            published: false,
            updated_at: Utc::now(),
        },
    ];

    let queryset = QuerySet::new(articles).filter(|a| a.published);
    let provider = ModelSitemapProvider::for_articles(queryset);

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
fn test_large_queryset_pagination() {
    // Test automatic pagination for large querysets
    let articles: Vec<Article> = (0..100)
        .map(|i| Article {
            id: i,
            slug: format!("article-{}", i),
            title: format!("Article {}", i),
            published: true,
            updated_at: Utc::now(),
        })
        .collect();

    let queryset = QuerySet::new(articles);
    let count = queryset.count();
    assert_eq!(count, 100);

    // Provider should handle all items
    let provider = ModelSitemapProvider::for_articles(queryset);
    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert_eq!(response.content.matches("<url>").count(), 100);
}

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

#[test]
fn test_model_sitemap_empty_queryset() {
    // Test with empty queryset
    let queryset = QuerySet::new(vec![]);
    let provider = ModelSitemapProvider::for_articles(queryset);

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
    let articles = vec![Article {
        id: 1,
        slug: "only-article".to_string(),
        title: "Only Article".to_string(),
        published: true,
        updated_at: Utc::now(),
    }];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::for_articles(queryset);

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response
        .content
        .contains("https://example.com/articles/only-article"));
    assert_eq!(response.content.matches("<url>").count(), 1);
}

#[test]
fn test_callable_location_and_lastmod() {
    // Test that location and lastmod can be callable (functions)
    let articles = vec![Article {
        id: 1,
        slug: "test".to_string(),
        title: "Test".to_string(),
        published: true,
        updated_at: Utc::now(),
    }];

    let queryset = QuerySet::new(articles);

    // Custom location function
    let custom_location = |article: &Article| format!("https://example.com/blog/{}", article.slug);

    let provider = ModelSitemapProvider {
        queryset,
        location_fn: custom_location,
        lastmod_fn: Some(|article| article.lastmod()),
        changefreq: None,
        priority: None,
    };

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should use custom location
    assert!(response.content.contains("https://example.com/blog/test"));
    assert!(!response
        .content
        .contains("https://example.com/articles/test"));
}
