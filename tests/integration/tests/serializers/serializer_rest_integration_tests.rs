// REST integration tests for Serializers
// Tests integration between reinhardt-serializers and reinhardt-rest

use reinhardt_orm::Model;
use reinhardt_pagination::{PaginatedResponse, PaginationMetadata};
use reinhardt_rest::{ApiResponse, DefaultRouter};
use reinhardt_serializers::{
    DefaultModelSerializer, Deserializer as ReinhardtDeserializer, JsonSerializer,
    ModelSerializer as ModelSerializerTrait, ModelSerializerBuilder, RelationshipStrategy,
    Serializer,
};
use serde::{Deserialize, Serialize};

// Test models

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Article {
    id: Option<i64>,
    title: String,
    content: String,
    author_id: i64,
    published: bool,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Author {
    id: Option<i64>,
    name: String,
    email: String,
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

// Test: API response with serializer
#[test]
fn test_api_response_with_serializer() {
    let article = Article {
        id: Some(1),
        title: "API Article".to_string(),
        content: "Content for API".to_string(),
        author_id: 1,
        published: true,
    };

    let response = ApiResponse::success(article.clone());
    assert_eq!(response.status, 200);
    assert!(response.data.is_some());
    assert_eq!(response.data.unwrap().title, "API Article");
}

// Test: Pagination with serializer
#[test]
fn test_serializer_rest_pagination() {
    let articles = vec![
        Article {
            id: Some(1),
            title: "Article 1".to_string(),
            content: "Content 1".to_string(),
            author_id: 1,
            published: true,
        },
        Article {
            id: Some(2),
            title: "Article 2".to_string(),
            content: "Content 2".to_string(),
            author_id: 1,
            published: true,
        },
    ];

    let metadata = PaginationMetadata {
        count: articles.len(),
        next: Some("http://example.com/api/articles/?page=2".to_string()),
        previous: None,
    };

    let paginated = PaginatedResponse::new(articles.clone(), metadata);

    assert_eq!(paginated.count, 2);
    assert!(paginated.next.is_some());
    assert!(paginated.previous.is_none());
    assert_eq!(paginated.results.len(), 2);
}

// Test: Filtering with serializer
#[test]
fn test_filtering_with_serializer() {
    let articles = vec![
        Article {
            id: Some(1),
            title: "Published Article".to_string(),
            content: "Content".to_string(),
            author_id: 1,
            published: true,
        },
        Article {
            id: Some(2),
            title: "Draft Article".to_string(),
            content: "Content".to_string(),
            author_id: 1,
            published: false,
        },
    ];

    let published: Vec<Article> = articles.into_iter().filter(|a| a.published).collect();
    assert_eq!(published.len(), 1);

    let serializer = JsonSerializer::<Vec<Article>>::new();
    let serialized = Serializer::serialize(&serializer, &published).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("Published Article"));
    assert!(!json_str.contains("Draft Article"));
}

// Test: List endpoint serialization
#[test]
fn test_list_endpoint_serialization() {
    let articles = vec![
        Article {
            id: Some(1),
            title: "First".to_string(),
            content: "Content 1".to_string(),
            author_id: 1,
            published: true,
        },
        Article {
            id: Some(2),
            title: "Second".to_string(),
            content: "Content 2".to_string(),
            author_id: 1,
            published: true,
        },
    ];

    let serializer = JsonSerializer::<Vec<Article>>::new();
    let serialized = Serializer::serialize(&serializer, &articles).unwrap();

    // Create API response
    let response = ApiResponse::success(articles.clone());
    assert_eq!(response.status, 200);

    // Verify can deserialize back
    let deserialized: Vec<Article> =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(articles.len(), deserialized.len());
}

// Test: Detail endpoint serialization
#[test]
fn test_detail_endpoint_serialization() {
    let article = Article {
        id: Some(1),
        title: "Detail Article".to_string(),
        content: "Detailed content".to_string(),
        author_id: 1,
        published: true,
    };

    let serializer = DefaultModelSerializer::<Article>::new();
    let serialized = Serializer::serialize(&serializer, &article).unwrap();

    let response = ApiResponse::success(article.clone());
    assert_eq!(response.status, 200);

    // Verify can deserialize back
    let deserialized: Article =
        ReinhardtDeserializer::deserialize(&serializer, &serialized).unwrap();
    assert_eq!(article, deserialized);
}

// Test: Create endpoint with serializer
#[test]
fn test_create_endpoint_with_serializer() {
    let new_article = Article {
        id: None,
        title: "New Article".to_string(),
        content: "New content".to_string(),
        author_id: 1,
        published: false,
    };

    let serializer = DefaultModelSerializer::<Article>::new();
    let created = ModelSerializerTrait::create(&serializer, new_article.clone()).unwrap();

    let response = ApiResponse::success(created.clone());
    assert_eq!(response.status, 200);
    assert_eq!(response.data.unwrap().title, "New Article");
}

// Test: Update endpoint with serializer
#[test]
fn test_update_endpoint_with_serializer() {
    let mut article = Article {
        id: Some(1),
        title: "Original Title".to_string(),
        content: "Original content".to_string(),
        author_id: 1,
        published: false,
    };

    let updated_data = Article {
        id: Some(1),
        title: "Updated Title".to_string(),
        content: "Updated content".to_string(),
        author_id: 1,
        published: true,
    };

    let serializer = DefaultModelSerializer::<Article>::new();
    ModelSerializerTrait::update(&serializer, &mut article, updated_data).unwrap();

    let response = ApiResponse::success(article.clone());
    assert_eq!(response.status, 200);
    assert_eq!(article.title, "Updated Title");
}

// Test: Error response serialization
#[test]
fn test_error_response_serialization() {
    let response: ApiResponse<Article> = ApiResponse::error("Article not found", 404);

    assert_eq!(response.status, 404);
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap(), "Article not found");
    assert!(response.data.is_none());
}

// Test: Validation error response
#[test]
fn test_serializer_rest_validation_error() {
    use std::collections::HashMap;

    let mut errors = HashMap::new();
    errors.insert(
        "title".to_string(),
        vec!["This field is required".to_string()],
    );
    errors.insert(
        "content".to_string(),
        vec!["Content must be at least 10 characters".to_string()],
    );

    let response: ApiResponse<Article> = ApiResponse::validation_error(errors.clone());

    assert_eq!(response.status, 400);
    assert!(response.errors.is_some());
    assert_eq!(response.errors.unwrap().len(), 2);
}

// Test: Nested resource serialization
#[test]
fn test_nested_resource_serialization() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct ArticleWithAuthor {
        id: Option<i64>,
        title: String,
        content: String,
        author: Author,
        published: bool,
    }

    impl Model for ArticleWithAuthor {
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

    let article = ArticleWithAuthor {
        id: Some(1),
        title: "Article with Author".to_string(),
        content: "Content".to_string(),
        author: Author {
            id: Some(1),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        },
        published: true,
    };

    let serializer = ModelSerializerBuilder::<ArticleWithAuthor>::new()
        .relationship_strategy(RelationshipStrategy::Nested)
        .depth(1)
        .build();

    let serialized = Serializer::serialize(&serializer, &article).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("John Doe"));
    assert!(json_str.contains("john@example.com"));

    let response = ApiResponse::success(article.clone());
    assert_eq!(response.status, 200);
}

// Test: Bulk operations endpoint
#[test]
fn test_bulk_operations_endpoint() {
    let articles = vec![
        Article {
            id: None,
            title: "Bulk 1".to_string(),
            content: "Content 1".to_string(),
            author_id: 1,
            published: false,
        },
        Article {
            id: None,
            title: "Bulk 2".to_string(),
            content: "Content 2".to_string(),
            author_id: 1,
            published: false,
        },
    ];

    let serializer = DefaultModelSerializer::<Article>::new();

    let mut created = Vec::new();
    for article in articles {
        created.push(ModelSerializerTrait::create(&serializer, article).unwrap());
    }

    let response = ApiResponse::success(created.clone());
    assert_eq!(response.status, 200);
    assert_eq!(response.data.unwrap().len(), 2);
}

// Test: Query parameter filtering
#[test]
fn test_query_parameter_filtering() {
    let articles = vec![
        Article {
            id: Some(1),
            title: "Tech Article".to_string(),
            content: "Tech content".to_string(),
            author_id: 1,
            published: true,
        },
        Article {
            id: Some(2),
            title: "Business Article".to_string(),
            content: "Business content".to_string(),
            author_id: 2,
            published: true,
        },
    ];

    let filtered: Vec<Article> = articles.into_iter().filter(|a| a.author_id == 1).collect();
    assert_eq!(filtered.len(), 1);

    let serializer = JsonSerializer::<Vec<Article>>::new();
    let serialized = Serializer::serialize(&serializer, &filtered).unwrap();
    let json_str = String::from_utf8(serialized).unwrap();

    assert!(json_str.contains("Tech Article"));
    assert!(!json_str.contains("Business Article"));
}

// Test: Router URL patterns
#[test]
fn test_router_url_patterns() {
    let _router = DefaultRouter::new();
    // Router is initialized correctly
    // Actual route registration would happen in ViewSet integration
}

// Test: HyperlinkedModelSerializer basic usage
#[test]
fn test_hyperlinked_model_serializer() {
    use reinhardt_serializers::{HyperlinkedModelSerializer, Serializer};

    let article = Article {
        id: Some(1),
        title: "Hyperlinked Article".to_string(),
        content: "Content with URL".to_string(),
        author_id: 1,
        published: true,
    };

    let serializer = HyperlinkedModelSerializer::<Article>::new("article-detail");
    let serialized = Serializer::serialize(&serializer, &article).unwrap();

    assert!(serialized.contains("\"url\""));
    assert!(serialized.contains("article-detail"));
    assert!(serialized.contains("Hyperlinked Article"));
}

// Test: HyperlinkedModelSerializer with custom URL field name
#[test]
fn test_hyperlinked_serializer_custom_url_field() {
    use reinhardt_serializers::{HyperlinkedModelSerializer, Serializer};

    let article = Article {
        id: Some(2),
        title: "Custom URL Field".to_string(),
        content: "Test content".to_string(),
        author_id: 1,
        published: true,
    };

    let serializer =
        HyperlinkedModelSerializer::<Article>::new("article-detail").url_field_name("self_link");
    let serialized = Serializer::serialize(&serializer, &article).unwrap();

    assert!(serialized.contains("\"self_link\""));
    assert!(!serialized.contains("\"url\""));
}

// Test: HyperlinkedModelSerializer deserialization
#[test]
fn test_hyperlinked_serializer_deserialization() {
    use reinhardt_serializers::{
        Deserializer as ReinhardtDeserializer, HyperlinkedModelSerializer,
    };

    let json = r#"{"id":3,"title":"Test","content":"Content","author_id":1,"published":true,"url":"/articles/article-detail/3"}"#;
    let serializer = HyperlinkedModelSerializer::<Article>::new("article-detail");

    let deserialized: Article =
        ReinhardtDeserializer::deserialize(&serializer, &json.to_string()).unwrap();
    assert_eq!(deserialized.id, Some(3));
    assert_eq!(deserialized.title, "Test");
}
