// Full ORM sitemap integration tests
// These tests use actual ORM integration functionality from reinhardt-sitemaps

use chrono::{DateTime, NaiveDate, Utc};
use reinhardt_sitemaps::orm_integration::*;
use reinhardt_sitemaps::*;
use serde::{Deserialize, Serialize};

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
