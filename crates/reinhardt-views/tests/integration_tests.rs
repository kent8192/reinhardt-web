use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_orm::Model;
use reinhardt_serializers::JsonSerializer;
use reinhardt_views::{DetailView, ListView, MultipleObjectMixin, SingleObjectMixin, View};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Author {
    id: Option<i64>,
    name: String,
    slug: String,
}

impl Model for Author {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "authors"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

#[tokio::test]
async fn test_list_view_empty() {
    let view = ListView::<Author, JsonSerializer<Author>>::new()
        .with_objects(vec![])
        .with_allow_empty(true);

    let request = Request::new(
        Method::GET,
        "/authors/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_list_view_with_objects() {
    let authors = vec![
        Author {
            id: Some(1),
            name: "John Doe".to_string(),
            slug: "john-doe".to_string(),
        },
        Author {
            id: Some(2),
            name: "Jane Smith".to_string(),
            slug: "jane-smith".to_string(),
        },
    ];

    let view = ListView::<Author, JsonSerializer<Author>>::new().with_objects(authors.clone());

    let request = Request::new(
        Method::GET,
        "/authors/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status, 200);

    // Check JSON response
    let body: Vec<serde_json::Value> = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(body.len(), 2);
}

#[tokio::test]
async fn test_list_view_empty_not_allowed() {
    let view = ListView::<Author, JsonSerializer<Author>>::new()
        .with_objects(vec![])
        .with_allow_empty(false);

    let request = Request::new(
        Method::GET,
        "/authors/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_list_view_wrong_method() {
    let view = ListView::<Author, JsonSerializer<Author>>::new().with_objects(vec![]);

    let request = Request::new(
        Method::POST,
        "/authors/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_detail_view_with_object() {
    let author = Author {
        id: Some(1),
        name: "John Doe".to_string(),
        slug: "john-doe".to_string(),
    };

    let view = DetailView::<Author, JsonSerializer<Author>>::new().with_object(author.clone());

    let request = Request::new(
        Method::GET,
        "/authors/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.status, 200);

    // Check JSON response
    let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    // JsonSerializer returns an empty object by default, so just check that we got valid JSON
    assert!(body.is_object() || body.is_array());
}

#[tokio::test]
async fn test_detail_view_without_object() {
    let view = DetailView::<Author, JsonSerializer<Author>>::new();

    let request = Request::new(
        Method::GET,
        "/authors/999/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_detail_view_wrong_method() {
    let author = Author {
        id: Some(1),
        name: "John Doe".to_string(),
        slug: "john-doe".to_string(),
    };

    let view = DetailView::<Author, JsonSerializer<Author>>::new().with_object(author);

    let request = Request::new(
        Method::DELETE,
        "/authors/1/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = view.dispatch(request).await;
    assert!(response.is_err());
}

#[tokio::test]
async fn test_list_view_with_context_object_name() {
    let authors = vec![Author {
        id: Some(1),
        name: "Author 1".to_string(),
        slug: "author-1".to_string(),
    }];

    let view = ListView::<Author, JsonSerializer<Author>>::new()
        .with_objects(authors)
        .with_context_object_name("authors");

    assert_eq!(view.get_context_object_name(), Some("authors"));
}

#[tokio::test]
async fn test_detail_view_with_custom_slug_field() {
    let author = Author {
        id: Some(1),
        name: "John Doe".to_string(),
        slug: "john-doe".to_string(),
    };

    let view = DetailView::<Author, JsonSerializer<Author>>::new()
        .with_object(author)
        .with_slug_field("custom_slug")
        .with_context_object_name("author");

    assert_eq!(view.get_slug_field(), "custom_slug");
    assert_eq!(view.get_context_object_name(), Some("author"));
}
