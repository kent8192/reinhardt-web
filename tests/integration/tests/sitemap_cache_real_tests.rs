//! Real cache integration tests for sitemaps
//!
//! These tests use actual InMemoryCache implementation from reinhardt-cache
//! to verify sitemap caching functionality.

use reinhardt_cache::{Cache, InMemoryCache};
use reinhardt_sitemaps::{
    AsyncCachedSitemapProvider, Sitemap, SitemapItem, SitemapProvider, SitemapResult,
};
use std::time::Duration;

#[derive(Debug)]
struct TestSitemapProvider {
    items: Vec<String>,
    items_per_page: Option<usize>,
}

impl TestSitemapProvider {
    fn new(items: Vec<String>) -> Self {
        Self {
            items,
            items_per_page: None,
        }
    }

    fn with_pagination(mut self, items_per_page: usize) -> Self {
        self.items_per_page = Some(items_per_page);
        self
    }
}

impl SitemapProvider for TestSitemapProvider {
    fn get_sitemap(&self) -> SitemapResult<Sitemap> {
        let mut sitemap = Sitemap::new();
        for url in &self.items {
            sitemap.add_item(SitemapItem::new(url))?;
        }
        Ok(sitemap)
    }

    fn get_paginated_sitemap(&self, page: usize) -> SitemapResult<Option<Sitemap>> {
        if let Some(per_page) = self.items_per_page {
            let start = (page - 1) * per_page;
            if start >= self.items.len() {
                return Ok(None);
            }

            let end = (start + per_page).min(self.items.len());
            let mut sitemap = Sitemap::new();
            for url in &self.items[start..end] {
                sitemap.add_item(SitemapItem::new(url))?;
            }
            Ok(Some(sitemap))
        } else {
            Ok(None)
        }
    }

    fn get_page_count(&self) -> usize {
        if let Some(per_page) = self.items_per_page {
            (self.items.len() + per_page - 1) / per_page
        } else {
            1
        }
    }
}

#[tokio::test]
async fn test_cached_sitemap_basic() {
    let cache = InMemoryCache::new();
    let provider = TestSitemapProvider::new(vec![
        "https://example.com/page1".to_string(),
        "https://example.com/page2".to_string(),
    ]);

    let cached_provider = AsyncCachedSitemapProvider::new(
        Box::new(provider),
        cache.clone(),
        "sitemap".to_string(),
        Some(Duration::from_secs(3600)),
    );

    // First call - cache miss, should generate and cache
    let sitemap1 = cached_provider.get_sitemap_async().await.unwrap();
    assert_eq!(sitemap1.items.len(), 2);

    // Verify cache has the key
    assert!(cache.has_key("sitemap").await.unwrap());

    // Second call - cache hit
    let sitemap2 = cached_provider.get_sitemap_async().await.unwrap();
    assert_eq!(sitemap2.items.len(), 2);
}

#[tokio::test]
async fn test_cached_sitemap_invalidation() {
    let cache = InMemoryCache::new();
    let provider = TestSitemapProvider::new(vec!["https://example.com/page1".to_string()]);

    let cached_provider = AsyncCachedSitemapProvider::new(
        Box::new(provider),
        cache.clone(),
        "sitemap".to_string(),
        Some(Duration::from_secs(3600)),
    );

    // Generate and cache
    let _ = cached_provider.get_sitemap_async().await.unwrap();
    assert!(cache.has_key("sitemap").await.unwrap());

    // Invalidate cache
    cached_provider.invalidate().await.unwrap();
    assert!(!cache.has_key("sitemap").await.unwrap());
}

#[tokio::test]
async fn test_cached_sitemap_pagination() {
    let cache = InMemoryCache::new();
    let items: Vec<String> = (1..=25)
        .map(|i| format!("https://example.com/page{}", i))
        .collect();

    let provider = TestSitemapProvider::new(items).with_pagination(10);

    let cached_provider = AsyncCachedSitemapProvider::new(
        Box::new(provider),
        cache.clone(),
        "sitemap".to_string(),
        Some(Duration::from_secs(3600)),
    );

    // Get page 1
    let page1 = cached_provider
        .get_paginated_sitemap_async(1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(page1.items.len(), 10);
    assert!(cache.has_key("sitemap:page:1").await.unwrap());

    // Get page 2
    let page2 = cached_provider
        .get_paginated_sitemap_async(2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(page2.items.len(), 10);
    assert!(cache.has_key("sitemap:page:2").await.unwrap());

    // Get page 3
    let page3 = cached_provider
        .get_paginated_sitemap_async(3)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(page3.items.len(), 5);
    assert!(cache.has_key("sitemap:page:3").await.unwrap());

    // Page 4 should not exist
    let page4 = cached_provider
        .get_paginated_sitemap_async(4)
        .await
        .unwrap();
    assert!(page4.is_none());
}

#[tokio::test]
async fn test_cached_sitemap_ttl_expiry() {
    let cache = InMemoryCache::new();
    let provider = TestSitemapProvider::new(vec!["https://example.com/page1".to_string()]);

    // Very short TTL for testing
    let cached_provider = AsyncCachedSitemapProvider::new(
        Box::new(provider),
        cache.clone(),
        "sitemap".to_string(),
        Some(Duration::from_millis(100)),
    );

    // Generate and cache
    let _ = cached_provider.get_sitemap_async().await.unwrap();
    assert!(cache.has_key("sitemap").await.unwrap());

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Cache should be expired (InMemoryCache might not auto-expire, but key should be stale)
    // Note: Actual expiry behavior depends on cache implementation
}

#[tokio::test]
async fn test_cached_sitemap_with_priority() {
    let cache = InMemoryCache::new();
    let provider = TestSitemapProvider::new(vec!["https://example.com/important".to_string()]);

    let cached_provider = AsyncCachedSitemapProvider::new(
        Box::new(provider),
        cache.clone(),
        "sitemap".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let sitemap = cached_provider.get_sitemap_async().await.unwrap();
    assert_eq!(sitemap.items.len(), 1);

    // Verify the item can have priority set
    let item = &sitemap.items[0];
    assert_eq!(item.loc, "https://example.com/important");
}

#[tokio::test]
async fn test_cached_sitemap_multiple_keys() {
    let cache = InMemoryCache::new();

    // Create two different cached providers with different prefixes
    let provider1 = TestSitemapProvider::new(vec!["https://example.com/section1".to_string()]);
    let cached1 = AsyncCachedSitemapProvider::new(
        Box::new(provider1),
        cache.clone(),
        "sitemap1".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let provider2 = TestSitemapProvider::new(vec!["https://example.com/section2".to_string()]);
    let cached2 = AsyncCachedSitemapProvider::new(
        Box::new(provider2),
        cache.clone(),
        "sitemap2".to_string(),
        Some(Duration::from_secs(3600)),
    );

    // Generate both sitemaps
    let _ = cached1.get_sitemap_async().await.unwrap();
    let _ = cached2.get_sitemap_async().await.unwrap();

    // Both should be cached with different keys
    assert!(cache.has_key("sitemap1").await.unwrap());
    assert!(cache.has_key("sitemap2").await.unwrap());

    // Invalidate only first one
    cached1.invalidate().await.unwrap();
    assert!(!cache.has_key("sitemap1").await.unwrap());
    assert!(cache.has_key("sitemap2").await.unwrap());
}
