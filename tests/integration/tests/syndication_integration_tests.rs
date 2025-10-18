//! Integration tests for reinhardt-syndication
//!
//! Tests the complete integration of feed views with HTTP serving

use bytes::Bytes;
use chrono::{DateTime, TimeZone, Utc};
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_http::{Request, Response};
use reinhardt_syndication::feed::{Feed, FeedItem};
use reinhardt_syndication::generator::{FeedGenerator, ModelFeed};
use reinhardt_syndication::view::{AtomFeedView, RssFeedView, StaticFeedView};
use reinhardt_views::View;

/// Mock model for testing feed generation
#[derive(Debug, Clone)]
struct TestArticle {
    id: i64,
    title: String,
    slug: String,
    content: String,
    author: String,
    published_at: DateTime<Utc>,
    category: String,
}

impl TestArticle {
    fn new(
        id: i64,
        title: &str,
        slug: &str,
        content: &str,
        author: &str,
        published_at: DateTime<Utc>,
        category: &str,
    ) -> Self {
        Self {
            id,
            title: title.to_string(),
            slug: slug.to_string(),
            content: content.to_string(),
            author: author.to_string(),
            published_at,
            category: category.to_string(),
        }
    }
}

/// Implementation of FeedGenerator for TestArticle
struct ArticleFeed {
    articles: Vec<TestArticle>,
}

impl FeedGenerator<TestArticle> for ArticleFeed {
    fn items(&self) -> Vec<TestArticle> {
        self.articles.clone()
    }

    fn title(&self) -> String {
        "My Blog".to_string()
    }

    fn link(&self) -> String {
        "http://example.com/".to_string()
    }

    fn description(&self) -> String {
        "Latest articles from my blog".to_string()
    }

    fn item_title(&self, item: &TestArticle) -> String {
        item.title.clone()
    }

    fn item_link(&self, item: &TestArticle) -> String {
        format!("http://example.com/articles/{}/", item.slug)
    }

    fn item_description(&self, item: &TestArticle) -> String {
        item.content.clone()
    }

    fn item_author(&self, item: &TestArticle) -> Option<String> {
        Some(item.author.clone())
    }

    fn item_pub_date(&self, item: &TestArticle) -> DateTime<Utc> {
        item.published_at
    }

    fn item_guid(&self, item: &TestArticle) -> Option<String> {
        Some(format!("article-{}", item.id))
    }

    fn item_categories(&self, item: &TestArticle) -> Vec<String> {
        vec![item.category.clone()]
    }
}

#[tokio::test]
async fn test_feed_view_integration() {
    // Test RSS feed view as HTTP view
    let pub_date1 = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
    let pub_date2 = Utc.with_ymd_and_hms(2024, 1, 2, 15, 30, 0).unwrap();

    let articles = vec![
        TestArticle::new(
            1,
            "First Article",
            "first-article",
            "This is the first article",
            "John Doe",
            pub_date1,
            "Technology",
        ),
        TestArticle::new(
            2,
            "Second Article",
            "second-article",
            "This is the second article",
            "Jane Smith",
            pub_date2,
            "Science",
        ),
    ];

    let feed_gen = ArticleFeed { articles };
    let rss_view = RssFeedView::new(feed_gen);

    // Create a mock GET request
    let request = Request::new(
        Method::GET,
        "/feed/rss/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Dispatch the request
    let response = rss_view
        .dispatch(request)
        .await
        .expect("Failed to dispatch request");

    // Verify response
    assert_eq!(response.status, hyper::StatusCode::OK);
    assert_eq!(
        response
            .headers
            .get("content-type")
            .and_then(|h| h.to_str().ok()),
        Some("application/rss+xml; charset=utf-8")
    );
    assert!(response.headers.contains_key("cache-control"));
    assert!(response.headers.contains_key("etag"));

    // Verify RSS content
    let body = String::from_utf8(response.body.to_vec()).expect("Invalid UTF-8 in response body");
    assert!(body.contains("<rss version=\"2.0\">"));
    assert!(body.contains("<title>My Blog</title>"));
    assert!(body.contains("<title>First Article</title>"));
    assert!(body.contains("<title>Second Article</title>"));
    assert!(body.contains("<guid>article-1</guid>"));
    assert!(body.contains("<guid>article-2</guid>"));
}

#[tokio::test]
async fn test_atom_feed_view_integration() {
    // Test Atom feed view as HTTP view
    let pub_date = Utc.with_ymd_and_hms(2024, 3, 15, 12, 0, 0).unwrap();

    let articles = vec![TestArticle::new(
        10,
        "Atom Article",
        "atom-article",
        "This is an Atom article",
        "Atom Author",
        pub_date,
        "Atom",
    )];

    let feed_gen = ArticleFeed { articles };
    let atom_view = AtomFeedView::new(feed_gen);

    // Create a mock GET request
    let request = Request::new(
        Method::GET,
        "/feed/atom/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Dispatch the request
    let response = atom_view
        .dispatch(request)
        .await
        .expect("Failed to dispatch request");

    // Verify response
    assert_eq!(response.status, hyper::StatusCode::OK);
    assert_eq!(
        response
            .headers
            .get("content-type")
            .and_then(|h| h.to_str().ok()),
        Some("application/atom+xml; charset=utf-8")
    );
    assert!(response.headers.contains_key("cache-control"));
    assert!(response.headers.contains_key("etag"));

    // Verify Atom content
    let body = String::from_utf8(response.body.to_vec()).expect("Invalid UTF-8 in response body");
    assert!(body.contains("<feed xmlns=\"http://www.w3.org/2005/Atom\">"));
    assert!(body.contains("<title>My Blog</title>"));
    assert!(body.contains("<title>Atom Article</title>"));
    assert!(body.contains("<name>Atom Author</name>"));
}

#[tokio::test]
async fn test_static_feed_view_integration() {
    // Test static feed view
    let pub_date = Utc.with_ymd_and_hms(2024, 6, 1, 9, 0, 0).unwrap();

    let mut feed = Feed::new("Static Blog", "http://example.com/", "A static blog feed");
    feed.add_item(
        FeedItem::new(
            "Static Article",
            "http://example.com/static-article/",
            "This is a static article",
            pub_date,
        )
        .with_author("Static Author")
        .with_guid("static-123"),
    );

    let static_view = StaticFeedView::new_rss(feed);

    // Create a mock GET request
    let request = Request::new(
        Method::GET,
        "/static-feed/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Dispatch the request
    let response = static_view
        .dispatch(request)
        .await
        .expect("Failed to dispatch request");

    // Verify response
    assert_eq!(response.status, hyper::StatusCode::OK);
    assert_eq!(
        response
            .headers
            .get("content-type")
            .and_then(|h| h.to_str().ok()),
        Some("application/rss+xml; charset=utf-8")
    );

    // Verify RSS content
    let body = String::from_utf8(response.body.to_vec()).expect("Invalid UTF-8 in response body");
    assert!(body.contains("<rss version=\"2.0\">"));
    assert!(body.contains("<title>Static Blog</title>"));
    assert!(body.contains("<title>Static Article</title>"));
    assert!(body.contains("<guid>static-123</guid>"));
}

#[tokio::test]
async fn test_feed_view_with_orm() {
    // Test feed view with ORM integration using ModelFeed
    let pub_date1 = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
    let pub_date2 = Utc.with_ymd_and_hms(2024, 1, 2, 15, 30, 0).unwrap();

    let articles = vec![
        TestArticle::new(
            1,
            "ORM Article 1",
            "orm-article-1",
            "First ORM article",
            "ORM Author 1",
            pub_date1,
            "ORM",
        ),
        TestArticle::new(
            2,
            "ORM Article 2",
            "orm-article-2",
            "Second ORM article",
            "ORM Author 2",
            pub_date2,
            "ORM",
        ),
    ];

    // Create ModelFeed with custom mapper
    let model_feed = ModelFeed::new(
        "ORM Blog",
        "http://example.com/orm/",
        "ORM-powered blog feed",
        articles,
        |article: &TestArticle| {
            FeedItem::new(
                &article.title,
                format!("http://example.com/orm/{}/", article.slug),
                &article.content,
                article.published_at,
            )
            .with_author(&article.author)
            .with_guid(format!("orm-article-{}", article.id))
            .with_categories(vec![article.category.clone()])
        },
    );

    let feed = model_feed.generate();
    let static_view = StaticFeedView::new_rss(feed);

    // Create a mock GET request
    let request = Request::new(
        Method::GET,
        "/orm-feed/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Dispatch the request
    let response = static_view
        .dispatch(request)
        .await
        .expect("Failed to dispatch request");

    // Verify response
    assert_eq!(response.status, hyper::StatusCode::OK);

    // Verify RSS content
    let body = String::from_utf8(response.body.to_vec()).expect("Invalid UTF-8 in response body");
    assert!(body.contains("<rss version=\"2.0\">"));
    assert!(body.contains("<title>ORM Blog</title>"));
    assert!(body.contains("<title>ORM Article 1</title>"));
    assert!(body.contains("<title>ORM Article 2</title>"));
    assert!(body.contains("<guid>orm-article-1</guid>"));
    assert!(body.contains("<guid>orm-article-2</guid>"));
}

#[tokio::test]
async fn test_feed_view_caching() {
    // Test that feed views include proper caching headers
    let pub_date = Utc.with_ymd_and_hms(2024, 9, 1, 12, 0, 0).unwrap();

    let articles = vec![TestArticle::new(
        100,
        "Cached Article",
        "cached-article",
        "This article should be cached",
        "Cache Author",
        pub_date,
        "Caching",
    )];

    let feed_gen = ArticleFeed { articles };
    let rss_view = RssFeedView::new(feed_gen);

    // Create a mock GET request
    let request = Request::new(
        Method::GET,
        "/cached-feed/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Dispatch the request
    let response = rss_view
        .dispatch(request)
        .await
        .expect("Failed to dispatch request");

    // Verify caching headers
    assert_eq!(response.status, hyper::StatusCode::OK);
    assert!(response.headers.contains_key("cache-control"));
    assert!(response.headers.contains_key("etag"));

    let cache_control = response
        .headers
        .get("cache-control")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cache_control.contains("public"));
    assert!(cache_control.contains("max-age=3600"));

    // Verify ETag format
    let etag = response.headers.get("etag").unwrap().to_str().unwrap();
    assert!(etag.starts_with('"'));
    assert!(etag.ends_with('"'));
    assert!(etag.len() > 2); // Should contain actual hash
}

#[tokio::test]
async fn test_feed_view_method_not_allowed() {
    // Test that non-GET methods are rejected
    let articles = vec![];
    let feed_gen = ArticleFeed { articles };
    let rss_view = RssFeedView::new(feed_gen);

    // Create a mock POST request
    let request = Request::new(
        Method::POST,
        "/feed/rss/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Dispatch the request - should fail
    let result = rss_view.dispatch(request).await;
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(error.to_string().contains("Method POST not allowed"));
    }
}

#[tokio::test]
async fn test_feed_view_empty_feed() {
    // Test feed view with empty feed
    let articles = vec![];
    let feed_gen = ArticleFeed { articles };
    let rss_view = RssFeedView::new(feed_gen);

    // Create a mock GET request
    let request = Request::new(
        Method::GET,
        "/empty-feed/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Dispatch the request
    let response = rss_view
        .dispatch(request)
        .await
        .expect("Failed to dispatch request");

    // Verify response
    assert_eq!(response.status, hyper::StatusCode::OK);

    // Verify RSS content (should be valid even with no items)
    let body = String::from_utf8(response.body.to_vec()).expect("Invalid UTF-8 in response body");
    assert!(body.contains("<rss version=\"2.0\">"));
    assert!(body.contains("<title>My Blog</title>"));
    assert!(!body.contains("<item>")); // No items
}
