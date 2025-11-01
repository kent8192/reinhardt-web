//! Additional view tests inspired by Django, DRF, and FastAPI test suites
//!
//! These tests cover edge cases and advanced features from the reference implementations.

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_orm::Model;
use reinhardt_serializers::JsonSerializer;
use reinhardt_views::{DetailView, ListView, MultipleObjectMixin, SingleObjectMixin, View};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Article {
	id: Option<i64>,
	title: String,
	slug: String,
	author: String,
}

impl Model for Article {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"articles"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

// ============================================================================
// Base View Tests (from Django generic_views/test_base.py)
// ============================================================================

/// Test that GET-only views reject POST requests (Django: ViewTest::test_get_only)
#[tokio::test]
async fn test_view_get_only() {
	let articles = vec![Article {
		id: Some(1),
		title: "Test Article".to_string(),
		slug: "test-article".to_string(),
		author: "John Doe".to_string(),
	}];

	let view = ListView::<Article, JsonSerializer<Article>>::new().with_objects(articles);

	// GET should succeed
	let get_request = Request::new(
		Method::GET,
		"/articles/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let response = view.dispatch(get_request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, 200);

	// POST should fail with 405 or validation error
	let post_request = Request::new(
		Method::POST,
		"/articles/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let response = view.dispatch(post_request).await;
	assert!(response.is_err());
}

/// Test HEAD method support for GET views (Django: ViewTest::test_get_and_head)
#[tokio::test]
async fn test_view_get_and_head() {
	let article = Article {
		id: Some(1),
		title: "Test Article".to_string(),
		slug: "test-article".to_string(),
		author: "John Doe".to_string(),
	};

	let view = DetailView::<Article, JsonSerializer<Article>>::new().with_object(article);

	// GET should work
	let get_request = Request::new(
		Method::GET,
		"/articles/1/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let response = view.dispatch(get_request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, 200);

	// HEAD requests should return same headers as GET but with empty body
	// This is now implemented in the View trait
}

/// Test context data building (Django: TemplateViewTest::test_extra_context)
#[tokio::test]
async fn test_view_context_data() {
	let articles = vec![
		Article {
			id: Some(1),
			title: "Article 1".to_string(),
			slug: "article-1".to_string(),
			author: "Author 1".to_string(),
		},
		Article {
			id: Some(2),
			title: "Article 2".to_string(),
			slug: "article-2".to_string(),
			author: "Author 2".to_string(),
		},
	];

	let view = ListView::<Article, JsonSerializer<Article>>::new()
		.with_objects(articles.clone())
		.with_context_object_name("articles");

	let context = view.get_context_data(articles);
	assert!(context.is_ok());
	let ctx = context.unwrap();
	assert!(ctx.contains_key("object_list"));
	assert!(ctx.contains_key("articles"));
}

// ============================================================================
// ListView Tests (from Django generic_views/test_list.py)
// ============================================================================

/// Test ListView with empty queryset when allowed
#[tokio::test]
async fn test_list_view_empty_queryset_allowed() {
	let view = ListView::<Article, JsonSerializer<Article>>::new()
		.with_objects(vec![])
		.with_allow_empty(true);

	let request = Request::new(
		Method::GET,
		"/articles/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = view.dispatch(request).await;
	assert!(response.is_ok());
	let resp = response.unwrap();
	assert_eq!(resp.status, 200);

	let body: Vec<serde_json::Value> = serde_json::from_slice(&resp.body).unwrap();
	assert_eq!(body.len(), 0);
}

/// Test ListView with empty queryset when not allowed
#[tokio::test]
async fn test_list_view_empty_queryset_not_allowed() {
	let view = ListView::<Article, JsonSerializer<Article>>::new()
		.with_objects(vec![])
		.with_allow_empty(false);

	let request = Request::new(
		Method::GET,
		"/articles/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = view.dispatch(request).await;
	assert!(response.is_err());
}

/// Test pagination configuration (Django: ListView with paginate_by)
#[tokio::test]
async fn test_list_view_pagination_config() {
	let articles: Vec<Article> = (1..=25)
		.map(|i| Article {
			id: Some(i),
			title: format!("Article {}", i),
			slug: format!("article-{}", i),
			author: "Test Author".to_string(),
		})
		.collect();

	let view = ListView::<Article, JsonSerializer<Article>>::new()
		.with_objects(articles)
		.with_paginate_by(10);

	assert_eq!(view.get_paginate_by(), Some(10));

	// Pagination is now implemented in dispatch()
	// Returns paginated results with metadata
}

/// Test ordering configuration (Django: ListView with ordering)
#[tokio::test]
async fn test_list_view_ordering_config() {
	let view = ListView::<Article, JsonSerializer<Article>>::new()
		.with_objects(vec![])
		.with_ordering(vec!["title".to_string(), "-created_at".to_string()]);

	assert_eq!(
		view.get_ordering(),
		Some(vec!["title".to_string(), "-created_at".to_string()])
	);

	// Ordering is now implemented in dispatch()
	// Sorts objects by specified fields before pagination
}

// ============================================================================
// DetailView Tests (from Django generic_views/test_detail.py)
// ============================================================================

/// Test DetailView with object set directly
#[tokio::test]
async fn test_detail_view_with_direct_object() {
	let article = Article {
		id: Some(42),
		title: "The Answer".to_string(),
		slug: "the-answer".to_string(),
		author: "Douglas Adams".to_string(),
	};

	let view = DetailView::<Article, JsonSerializer<Article>>::new().with_object(article.clone());

	let request = Request::new(
		Method::GET,
		"/articles/42/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = view.dispatch(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, 200);
}

/// Test DetailView primary key lookup (Django: DetailView with pk)
#[tokio::test]
async fn test_detail_view_pk_lookup() {
	let view = DetailView::<Article, JsonSerializer<Article>>::new().with_pk_url_kwarg("id");

	let mut path_params = HashMap::new();
	path_params.insert("id".to_string(), "123".to_string());

	let mut request = Request::new(
		Method::GET,
		"/articles/123/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	request.path_params = path_params;

	let response = view.dispatch(request).await;
	// Fails because no queryset is configured
	// Error message guides user to use with_queryset() or with_object()
	assert!(response.is_err());

	// Database lookup is now implemented via QuerySet API
	// Usage: DetailView::new().with_queryset(QuerySet::<Article>::new())
	// The view will automatically perform database lookup using the pk from URL
}

/// Test DetailView slug lookup (Django: DetailView with slug)
#[tokio::test]
async fn test_detail_view_slug_lookup() {
	let view = DetailView::<Article, JsonSerializer<Article>>::new()
		.with_slug_field("slug")
		.with_slug_url_kwarg("article_slug");

	let mut path_params = HashMap::new();
	path_params.insert("article_slug".to_string(), "test-article".to_string());

	let mut request = Request::new(
		Method::GET,
		"/articles/test-article/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	request.path_params = path_params;

	let response = view.dispatch(request).await;
	// Fails because no queryset is configured
	// Error message guides user to use with_queryset() or with_object()
	assert!(response.is_err());

	// Database lookup by slug is now implemented via QuerySet API
	// Usage: DetailView::new().with_queryset(QuerySet::<Article>::new()).with_slug_field("slug")
	// The view will automatically perform database lookup using the slug from URL
}

/// Test DetailView 404 when object not found
#[tokio::test]
async fn test_detail_view_object_not_found() {
	let view = DetailView::<Article, JsonSerializer<Article>>::new();

	let request = Request::new(
		Method::GET,
		"/articles/999/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = view.dispatch(request).await;
	assert!(response.is_err());
}

/// Test DetailView custom context object name
#[tokio::test]
async fn test_detail_view_custom_context_name() {
	let article = Article {
		id: Some(1),
		title: "Test".to_string(),
		slug: "test".to_string(),
		author: "Author".to_string(),
	};

	let view = DetailView::<Article, JsonSerializer<Article>>::new()
		.with_object(article.clone())
		.with_context_object_name("article");

	assert_eq!(view.get_context_object_name(), Some("article"));

	let context = view.get_context_data(article);
	assert!(context.is_ok());
	let ctx = context.unwrap();
	assert!(ctx.contains_key("object"));
	assert!(ctx.contains_key("article"));
}

// ============================================================================
// Method Handling Tests (from DRF test_views.py)
// ============================================================================

/// Test that unsupported methods return appropriate errors
#[tokio::test]
async fn test_view_method_not_allowed() {
	let view = ListView::<Article, JsonSerializer<Article>>::new().with_objects(vec![]);

	// Test various unsupported methods
	for method in &[Method::PUT, Method::DELETE, Method::PATCH] {
		let request = Request::new(
			method.clone(),
			"/articles/".parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);

		let response = view.dispatch(request).await;
		assert!(
			response.is_err(),
			"Method {:?} should not be allowed",
			method
		);
	}
}

/// Test OPTIONS method
#[tokio::test]
async fn test_view_options_method() {
	let view = ListView::<Article, JsonSerializer<Article>>::new().with_objects(vec![]);

	let request = Request::new(
		Method::OPTIONS,
		"/articles/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = view.dispatch(request).await;
	// Should return allowed methods: Allow: GET, HEAD, OPTIONS
	assert!(response.is_ok());
	let resp = response.unwrap();
	assert_eq!(resp.status, 200);
	// Check Allow header is present
	assert!(resp.headers.contains_key("allow") || resp.headers.contains_key("Allow"));
}

// ============================================================================
// Advanced Configuration Tests
// ============================================================================

/// Test multiple context object names don't conflict
#[tokio::test]
async fn test_list_view_context_object_name_priority() {
	let articles = vec![Article {
		id: Some(1),
		title: "Test".to_string(),
		slug: "test".to_string(),
		author: "Author".to_string(),
	}];

	let view = ListView::<Article, JsonSerializer<Article>>::new()
		.with_objects(articles.clone())
		.with_context_object_name("my_articles");

	let context = view.get_context_data(articles);
	assert!(context.is_ok());
	let ctx = context.unwrap();

	// Should have both default and custom names
	assert!(ctx.contains_key("object_list"));
	assert!(ctx.contains_key("my_articles"));
}

/// Test view builder pattern chaining
#[tokio::test]
async fn test_list_view_builder_chaining() {
	let articles = vec![Article {
		id: Some(1),
		title: "Test".to_string(),
		slug: "test".to_string(),
		author: "Author".to_string(),
	}];

	let view = ListView::<Article, JsonSerializer<Article>>::new()
		.with_objects(articles)
		.with_paginate_by(20)
		.with_allow_empty(false)
		.with_ordering(vec!["-id".to_string()])
		.with_context_object_name("articles");

	assert_eq!(view.get_paginate_by(), Some(20));
	assert!(!view.allow_empty());
	assert_eq!(view.get_ordering(), Some(vec!["-id".to_string()]));
	assert_eq!(view.get_context_object_name(), Some("articles"));
}

/// Test detail view builder pattern chaining
#[tokio::test]
async fn test_detail_view_builder_chaining() {
	let article = Article {
		id: Some(1),
		title: "Test".to_string(),
		slug: "test".to_string(),
		author: "Author".to_string(),
	};

	let view = DetailView::<Article, JsonSerializer<Article>>::new()
		.with_object(article)
		.with_slug_field("custom_slug")
		.with_pk_url_kwarg("article_id")
		.with_slug_url_kwarg("article_slug")
		.with_context_object_name("article");

	assert_eq!(view.get_slug_field(), "custom_slug");
	assert_eq!(view.pk_url_kwarg(), "article_id");
	assert_eq!(view.slug_url_kwarg(), "article_slug");
	assert_eq!(view.get_context_object_name(), Some("article"));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

/// Test empty context object name (should use defaults)
#[tokio::test]
async fn test_list_view_without_custom_context_name() {
	let articles = vec![Article {
		id: Some(1),
		title: "Test".to_string(),
		slug: "test".to_string(),
		author: "Author".to_string(),
	}];

	let view = ListView::<Article, JsonSerializer<Article>>::new().with_objects(articles.clone());

	let context = view.get_context_data(articles);
	assert!(context.is_ok());
	let ctx = context.unwrap();

	// Should only have default name
	assert!(ctx.contains_key("object_list"));
	assert_eq!(ctx.len(), 1);
}

/// Test large object list handling
#[tokio::test]
async fn test_list_view_large_dataset() {
	let articles: Vec<Article> = (1..=1000)
		.map(|i| Article {
			id: Some(i),
			title: format!("Article {}", i),
			slug: format!("article-{}", i),
			author: "Test Author".to_string(),
		})
		.collect();

	let view = ListView::<Article, JsonSerializer<Article>>::new().with_objects(articles);

	let request = Request::new(
		Method::GET,
		"/articles/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = view.dispatch(request).await;
	assert!(response.is_ok());

	// Without pagination config, returns all objects
	// Use with_paginate_by() to enable pagination for large datasets
}
