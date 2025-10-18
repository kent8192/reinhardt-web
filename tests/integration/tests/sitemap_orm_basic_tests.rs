//! Basic ORM integration tests for sitemaps
//!
//! These tests demonstrate sitemap generation with model-like structures
//! and basic ORM patterns using reinhardt-orm traits.

use chrono::{DateTime, NaiveDate, Utc};
use reinhardt_sitemaps::*;
use serde::{Deserialize, Serialize};

// Article model using composition pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
    id: Option<i64>,
    slug: String,
    title: String,
    published: bool,
    updated_at: DateTime<Utc>,
}

impl Article {
    fn new(slug: String, title: String, updated_at: DateTime<Utc>) -> Self {
        Self {
            id: None,
            slug,
            title,
            published: true,
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

// Simple queryset-like structure
#[derive(Debug, Clone)]
struct QuerySet<T> {
    items: Vec<T>,
}

impl<T: Clone> QuerySet<T> {
    fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    fn filter<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&T) -> bool,
    {
        self.items.retain(|item| predicate(item));
        self
    }

    fn all(&self) -> Vec<T> {
        self.items.clone()
    }

    fn count(&self) -> usize {
        self.items.len()
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

// Generic model sitemap provider (similar to Django's GenericSitemap)
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

    fn with_filter<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&Article) -> bool,
    {
        self.queryset = self.queryset.filter(predicate);
        self
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
            self.queryset.iter().map(lastmod_fn).max()
        } else {
            None
        }
    }
}

#[test]
fn test_basic_model_sitemap() {
    let articles = vec![
        Article::new(
            "first-article".to_string(),
            "First Article".to_string(),
            DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
        ),
        Article::new(
            "second-article".to_string(),
            "Second Article".to_string(),
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
    assert!(response.content.contains("<changefreq>weekly</changefreq>"));
    assert!(response.content.contains("<priority>0.7</priority>"));
}

#[test]
fn test_model_sitemap_with_filtering() {
    let articles = vec![
        Article::new(
            "published-article".to_string(),
            "Published".to_string(),
            Utc::now(),
        ),
        Article {
            published: false,
            ..Article::new("draft-article".to_string(), "Draft".to_string(), Utc::now())
        },
    ];

    let queryset = QuerySet::new(articles);
    let provider =
        ModelSitemapProvider::for_articles(queryset).with_filter(|article| article.published);

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Only published article should appear
    assert!(response
        .content
        .contains("https://example.com/articles/published-article"));
    assert!(!response
        .content
        .contains("https://example.com/articles/draft-article"));
}

#[test]
fn test_model_sitemap_empty_queryset() {
    let queryset = QuerySet::new(vec![]);
    let provider = ModelSitemapProvider::for_articles(queryset);

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should render empty sitemap
    assert!(response.content.contains("<urlset"));
    assert!(response.content.contains("</urlset>"));
}

#[test]
fn test_model_sitemap_single_item() {
    let articles = vec![Article::new(
        "only-article".to_string(),
        "Only Article".to_string(),
        Utc::now(),
    )];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::for_articles(queryset);

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response
        .content
        .contains("https://example.com/articles/only-article"));
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
        "recent-article".to_string(),
        "Recent".to_string(),
        updated_time,
    )];

    let queryset = QuerySet::new(articles);
    let provider = ModelSitemapProvider::for_articles(queryset);

    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    // Should have Last-Modified header
    assert!(response.has_header("Last-Modified"));
    assert!(response.content.contains("<lastmod>"));
}

#[test]
fn test_callable_location_and_lastmod() {
    // Custom model with different URL pattern
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Product {
        sku: String,
        updated_at: DateTime<Utc>,
    }

    impl Product {
        fn get_url(&self) -> String {
            format!("https://shop.example.com/products/{}", self.sku)
        }
    }

    let products = vec![
        Product {
            sku: "PROD-001".to_string(),
            updated_at: Utc::now(),
        },
        Product {
            sku: "PROD-002".to_string(),
            updated_at: Utc::now(),
        },
    ];

    #[derive(Debug)]
    struct ProductSitemapProvider {
        products: Vec<Product>,
    }

    impl SitemapProvider for ProductSitemapProvider {
        fn get_sitemap(&self) -> SitemapResult<Sitemap> {
            let mut sitemap = Sitemap::new();
            for product in &self.products {
                let item = SitemapItem::new(product.get_url())
                    .with_lastmod(product.updated_at)
                    .with_changefreq(ChangeFrequency::Daily)
                    .with_priority(Priority::new(0.8)?);
                sitemap.add_item(item)?;
            }
            Ok(sitemap)
        }
    }

    let provider = ProductSitemapProvider { products };
    let view = SitemapView::new(Box::new(provider));
    let response = view.render().unwrap();

    assert!(response
        .content
        .contains("https://shop.example.com/products/PROD-001"));
    assert!(response
        .content
        .contains("https://shop.example.com/products/PROD-002"));
}
