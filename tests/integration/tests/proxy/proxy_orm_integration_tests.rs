//! ORM integration tests for proxy functionality
//!
//! These tests verify proxy integration with ORM models using the OrmReflectable trait.

use reinhardt_proxy::{
    impl_orm_reflectable, CollectionProxy, OrmReflectable, ProxyError, ScalarProxy, ScalarValue,
};

#[derive(Clone, Debug)]
struct User {
    id: i64,
    name: String,
    email: String,
    posts: Vec<Post>,
}

#[derive(Clone, Debug)]
struct Post {
    id: i64,
    title: String,
    content: String,
    views: i64,
}

impl_orm_reflectable!(User {
    fields: {
        id => Integer,
        name => String,
        email => String,
    },
    relationships: {
        posts => Collection(Post),
    }
});

impl_orm_reflectable!(Post {
    fields: {
        id => Integer,
        title => String,
        content => String,
        views => Integer,
    },
    relationships: {}
});

#[tokio::test]
async fn test_scalar_proxy_with_orm_model() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        posts: vec![],
    };

    // Test getting attributes through Reflectable trait
    let name = user.get_attribute("name").unwrap();
    assert_eq!(name.as_string().unwrap(), "John Doe");

    let email = user.get_attribute("email").unwrap();
    assert_eq!(email.as_string().unwrap(), "john@example.com");

    let id = user.get_attribute("id").unwrap();
    assert_eq!(id.as_integer().unwrap(), 1);
}

#[tokio::test]
async fn test_collection_proxy_with_orm_models() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        posts: vec![
            Post {
                id: 1,
                title: "First Post".to_string(),
                content: "Content 1".to_string(),
                views: 100,
            },
            Post {
                id: 2,
                title: "Second Post".to_string(),
                content: "Content 2".to_string(),
                views: 200,
            },
        ],
    };

    // Create a collection proxy to access post titles
    let titles_proxy = CollectionProxy::new("posts", "title");

    // Get all post titles through the proxy
    let titles = titles_proxy.get_values(&user).await.unwrap();

    assert_eq!(titles.len(), 2);
    assert_eq!(titles[0].as_string().unwrap(), "First Post");
    assert_eq!(titles[1].as_string().unwrap(), "Second Post");
}

#[tokio::test]
async fn test_collection_proxy_count() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "Jane Doe".to_string(),
        email: "jane@example.com".to_string(),
        posts: vec![
            Post {
                id: 1,
                title: "Post 1".to_string(),
                content: "Content".to_string(),
                views: 50,
            },
            Post {
                id: 2,
                title: "Post 2".to_string(),
                content: "Content".to_string(),
                views: 75,
            },
            Post {
                id: 3,
                title: "Post 3".to_string(),
                content: "Content".to_string(),
                views: 100,
            },
        ],
    };

    let posts_proxy = CollectionProxy::new("posts", "id");
    let count = posts_proxy.count(&user).await.unwrap();

    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_collection_proxy_contains() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
        posts: vec![
            Post {
                id: 1,
                title: "Rust Tutorial".to_string(),
                content: "Learn Rust".to_string(),
                views: 500,
            },
            Post {
                id: 2,
                title: "Web Development".to_string(),
                content: "Build web apps".to_string(),
                views: 300,
            },
        ],
    };

    let titles_proxy = CollectionProxy::new("posts", "title");

    let contains_rust = titles_proxy
        .contains(&user, ScalarValue::String("Rust Tutorial".to_string()))
        .await
        .unwrap();
    assert!(contains_rust);

    let contains_python = titles_proxy
        .contains(&user, ScalarValue::String("Python Tutorial".to_string()))
        .await
        .unwrap();
    assert!(!contains_python);
}

#[tokio::test]
async fn test_collection_proxy_filter_by() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        posts: vec![
            Post {
                id: 1,
                title: "Popular Post".to_string(),
                content: "Content".to_string(),
                views: 1500,
            },
            Post {
                id: 2,
                title: "Normal Post".to_string(),
                content: "Content".to_string(),
                views: 500,
            },
            Post {
                id: 3,
                title: "Viral Post".to_string(),
                content: "Content".to_string(),
                views: 5000,
            },
        ],
    };

    let views_proxy = CollectionProxy::new("posts", "views");

    // Filter posts with more than 1000 views
    let popular_views = views_proxy
        .filter_by(&user, |v| matches!(v, ScalarValue::Integer(n) if *n > 1000))
        .await
        .unwrap();

    assert_eq!(popular_views.len(), 2);
    assert_eq!(popular_views[0].as_integer().unwrap(), 1500);
    assert_eq!(popular_views[1].as_integer().unwrap(), 5000);
}

#[tokio::test]
async fn test_mutable_operations_on_proxy() {
    use reinhardt_proxy::Reflectable;

    let mut user = User {
        id: 1,
        name: "Charlie".to_string(),
        email: "charlie@example.com".to_string(),
        posts: vec![Post {
            id: 1,
            title: "Original Title".to_string(),
            content: "Content".to_string(),
            views: 100,
        }],
    };

    // Update user's name through set_attribute
    user.set_attribute("name", ScalarValue::String("Charles".to_string()))
        .unwrap();

    assert_eq!(user.name, "Charles");

    // Update email
    user.set_attribute(
        "email",
        ScalarValue::String("charles@example.com".to_string()),
    )
    .unwrap();

    assert_eq!(user.email, "charles@example.com");
}

#[tokio::test]
async fn test_collection_proxy_remove() {
    use reinhardt_proxy::Reflectable;

    let mut user = User {
        id: 1,
        name: "Dave".to_string(),
        email: "dave@example.com".to_string(),
        posts: vec![
            Post {
                id: 1,
                title: "Keep This".to_string(),
                content: "Content".to_string(),
                views: 100,
            },
            Post {
                id: 2,
                title: "Remove This".to_string(),
                content: "Content".to_string(),
                views: 200,
            },
        ],
    };

    let titles_proxy = CollectionProxy::new("posts", "title");

    // Remove post with title "Remove This"
    titles_proxy
        .remove(&mut user, ScalarValue::String("Remove This".to_string()))
        .await
        .unwrap();

    // Verify only one post remains
    assert_eq!(user.posts.len(), 1);
    assert_eq!(user.posts[0].title, "Keep This");
}

#[tokio::test]
async fn test_collection_proxy_unique() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "Eve".to_string(),
        email: "eve@example.com".to_string(),
        posts: vec![
            Post {
                id: 1,
                title: "Duplicate".to_string(),
                content: "Content 1".to_string(),
                views: 100,
            },
            Post {
                id: 2,
                title: "Unique".to_string(),
                content: "Content 2".to_string(),
                views: 200,
            },
            Post {
                id: 3,
                title: "Duplicate".to_string(),
                content: "Content 3".to_string(),
                views: 300,
            },
        ],
    };

    // Create a unique collection proxy
    let titles_proxy = CollectionProxy::unique("posts", "title");

    let titles = titles_proxy.get_values(&user).await.unwrap();

    // Should only have 2 unique titles
    assert_eq!(titles.len(), 2);
}

#[tokio::test]
async fn test_attribute_not_found_error() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "Frank".to_string(),
        email: "frank@example.com".to_string(),
        posts: vec![],
    };

    // Try to get non-existent attribute
    let result = user.get_attribute("nonexistent");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_relationship_not_found_error() {
    use reinhardt_proxy::Reflectable;

    let user = User {
        id: 1,
        name: "Grace".to_string(),
        email: "grace@example.com".to_string(),
        posts: vec![],
    };

    let proxy = CollectionProxy::new("comments", "text");

    // Try to access non-existent relationship
    let result = proxy.get_values(&user).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ProxyError::RelationshipNotFound(_)
    ));
}
