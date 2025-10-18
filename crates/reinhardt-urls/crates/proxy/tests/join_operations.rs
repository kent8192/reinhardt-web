//! Tests for JOIN operations and nested relationship traversal

use reinhardt_proxy::{
    FilterCondition, FilterOp, LoadingStrategy, NestedProxy, ProxyResult, Reflectable,
    RelationshipPath, ScalarValue,
};
use std::any::Any;

/// User model
#[derive(Debug, Clone)]
struct User {
    name: String,
    age: i64,
    posts: Vec<Post>,
}

/// Post model
#[derive(Debug, Clone)]
struct Post {
    title: String,
    content: String,
    comments: Vec<Comment>,
}

/// Comment model
#[derive(Debug, Clone)]
struct Comment {
    author: String,
    text: String,
}

impl Reflectable for User {
    fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
        match name {
            "posts" => {
                let posts: Vec<Box<dyn Reflectable>> = self
                    .posts
                    .iter()
                    .map(|p| Box::new(p.clone()) as Box<dyn Reflectable>)
                    .collect();
                Some(Box::new(posts))
            }
            _ => None,
        }
    }

    fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
        None
    }

    fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
        match name {
            "name" => Some(ScalarValue::String(self.name.clone())),
            "age" => Some(ScalarValue::Integer(self.age)),
            _ => None,
        }
    }

    fn set_attribute(&mut self, _name: &str, _value: ScalarValue) -> ProxyResult<()> {
        Ok(())
    }

    fn set_relationship_attribute(
        &mut self,
        relationship: &str,
        _attribute: &str,
        _value: ScalarValue,
    ) -> ProxyResult<()> {
        Err(reinhardt_proxy::ProxyError::RelationshipNotFound(
            relationship.to_string(),
        ))
    }
}

impl Reflectable for Post {
    fn get_relationship(&self, name: &str) -> Option<Box<dyn Any>> {
        match name {
            "comments" => {
                let comments: Vec<Box<dyn Reflectable>> = self
                    .comments
                    .iter()
                    .map(|c| Box::new(c.clone()) as Box<dyn Reflectable>)
                    .collect();
                Some(Box::new(comments))
            }
            _ => None,
        }
    }

    fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
        None
    }

    fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
        match name {
            "title" => Some(ScalarValue::String(self.title.clone())),
            "content" => Some(ScalarValue::String(self.content.clone())),
            _ => None,
        }
    }

    fn set_attribute(&mut self, _name: &str, _value: ScalarValue) -> ProxyResult<()> {
        Ok(())
    }

    fn set_relationship_attribute(
        &mut self,
        relationship: &str,
        _attribute: &str,
        _value: ScalarValue,
    ) -> ProxyResult<()> {
        Err(reinhardt_proxy::ProxyError::RelationshipNotFound(
            relationship.to_string(),
        ))
    }
}

impl Reflectable for Comment {
    fn get_relationship(&self, _name: &str) -> Option<Box<dyn Any>> {
        None
    }

    fn get_relationship_mut(&mut self, _name: &str) -> Option<&mut dyn Any> {
        None
    }

    fn get_attribute(&self, name: &str) -> Option<ScalarValue> {
        match name {
            "author" => Some(ScalarValue::String(self.author.clone())),
            "text" => Some(ScalarValue::String(self.text.clone())),
            _ => None,
        }
    }

    fn set_attribute(&mut self, _name: &str, _value: ScalarValue) -> ProxyResult<()> {
        Ok(())
    }

    fn set_relationship_attribute(
        &mut self,
        relationship: &str,
        _attribute: &str,
        _value: ScalarValue,
    ) -> ProxyResult<()> {
        Err(reinhardt_proxy::ProxyError::RelationshipNotFound(
            relationship.to_string(),
        ))
    }
}

fn create_test_user() -> User {
    User {
        name: "Alice".to_string(),
        age: 30,
        posts: vec![
            Post {
                title: "First Post".to_string(),
                content: "Hello World".to_string(),
                comments: vec![
                    Comment {
                        author: "Bob".to_string(),
                        text: "Great post!".to_string(),
                    },
                    Comment {
                        author: "Charlie".to_string(),
                        text: "Thanks for sharing".to_string(),
                    },
                ],
            },
            Post {
                title: "Second Post".to_string(),
                content: "More content".to_string(),
                comments: vec![
                    Comment {
                        author: "Bob".to_string(),
                        text: "Another comment".to_string(),
                    },
                    Comment {
                        author: "David".to_string(),
                        text: "Interesting".to_string(),
                    },
                    Comment {
                        author: "Bob".to_string(),
                        text: "One more".to_string(),
                    },
                ],
            },
        ],
    }
}

#[tokio::test]
async fn test_nested_proxy_single_level() {
    let user = create_test_user();

    // Access posts titles directly
    let proxy = NestedProxy::from_path("posts", "title");
    let titles = proxy.get_values(&user).await.unwrap();

    assert_eq!(titles.len(), 2);
    assert!(titles.contains(&ScalarValue::String("First Post".to_string())));
    assert!(titles.contains(&ScalarValue::String("Second Post".to_string())));
}

#[tokio::test]
async fn test_nested_proxy_two_levels() {
    let user = create_test_user();

    // Access comment authors through posts
    let proxy = NestedProxy::from_path("posts.comments", "author");
    let authors = proxy.get_values(&user).await.unwrap();

    assert_eq!(authors.len(), 5); // 2 + 3 comments
    assert!(authors.contains(&ScalarValue::String("Bob".to_string())));
    assert!(authors.contains(&ScalarValue::String("Charlie".to_string())));
    assert!(authors.contains(&ScalarValue::String("David".to_string())));
}

#[tokio::test]
async fn test_nested_proxy_with_unique() {
    let user = create_test_user();

    // Get unique comment authors
    let proxy = NestedProxy::from_path("posts.comments", "author").unique();
    let authors = proxy.get_values(&user).await.unwrap();

    // Bob appears 3 times but should only be counted once
    assert_eq!(authors.len(), 3); // Bob, Charlie, David
    assert!(authors.contains(&ScalarValue::String("Bob".to_string())));
    assert!(authors.contains(&ScalarValue::String("Charlie".to_string())));
    assert!(authors.contains(&ScalarValue::String("David".to_string())));
}

#[tokio::test]
async fn test_nested_proxy_with_filter() {
    let user = create_test_user();

    // Get comment authors where author == "Bob"
    let filter = FilterCondition::new("author", FilterOp::eq("Bob"));
    let proxy = NestedProxy::from_path("posts.comments", "author").with_filter(filter);
    let authors = proxy.get_values(&user).await.unwrap();

    assert_eq!(authors.len(), 3); // Bob appears 3 times
    assert!(authors
        .iter()
        .all(|a| a == &ScalarValue::String("Bob".to_string())));
}

#[tokio::test]
async fn test_nested_proxy_with_filter_and_unique() {
    let user = create_test_user();

    // Get unique comment authors where author starts with "B"
    let filter = FilterCondition::new("author", FilterOp::starts_with("B"));
    let proxy = NestedProxy::from_path("posts.comments", "author")
        .with_filter(filter)
        .unique();
    let authors = proxy.get_values(&user).await.unwrap();

    assert_eq!(authors.len(), 1); // Only Bob
    assert_eq!(authors[0], ScalarValue::String("Bob".to_string()));
}

#[tokio::test]
async fn test_nested_proxy_count() {
    let user = create_test_user();

    // Count total comments
    let proxy = NestedProxy::from_path("posts.comments", "author");
    let count = proxy.count(&user).await.unwrap();

    assert_eq!(count, 5); // 2 + 3 comments
}

#[tokio::test]
async fn test_nested_proxy_count_with_unique() {
    let user = create_test_user();

    // Count unique authors
    let proxy = NestedProxy::from_path("posts.comments", "author").unique();
    let count = proxy.count(&user).await.unwrap();

    assert_eq!(count, 3); // Bob, Charlie, David
}

#[tokio::test]
async fn test_nested_proxy_contains() {
    let user = create_test_user();

    let proxy = NestedProxy::from_path("posts.comments", "author");

    // Check if Bob is an author
    let has_bob = proxy
        .contains(&user, ScalarValue::String("Bob".to_string()))
        .await
        .unwrap();
    assert!(has_bob);

    // Check if Eve is an author
    let has_eve = proxy
        .contains(&user, ScalarValue::String("Eve".to_string()))
        .await
        .unwrap();
    assert!(!has_eve);
}

#[tokio::test]
async fn test_nested_proxy_with_loading_strategy() {
    let user = create_test_user();

    // Test different loading strategies
    let lazy_proxy = NestedProxy::from_path("posts", "title").with_strategy(LoadingStrategy::Lazy);
    let joined_proxy =
        NestedProxy::from_path("posts", "title").with_strategy(LoadingStrategy::Joined);
    let subquery_proxy =
        NestedProxy::from_path("posts", "title").with_strategy(LoadingStrategy::Subquery);

    // All should return the same results (strategy affects SQL generation, not in-memory ops)
    let lazy_titles = lazy_proxy.get_values(&user).await.unwrap();
    let joined_titles = joined_proxy.get_values(&user).await.unwrap();
    let subquery_titles = subquery_proxy.get_values(&user).await.unwrap();

    assert_eq!(lazy_titles.len(), 2);
    assert_eq!(joined_titles.len(), 2);
    assert_eq!(subquery_titles.len(), 2);
}

#[tokio::test]
async fn test_relationship_path_parsing() {
    // Test dot-separated path parsing
    let path = RelationshipPath::from_string("user.posts.comments");
    assert_eq!(path.depth(), 3);
    assert_eq!(path.segments[0], "user");
    assert_eq!(path.segments[1], "posts");
    assert_eq!(path.segments[2], "comments");
}

#[tokio::test]
async fn test_nested_proxy_comment_texts() {
    let user = create_test_user();

    // Get all comment texts
    let proxy = NestedProxy::from_path("posts.comments", "text");
    let texts = proxy.get_values(&user).await.unwrap();

    assert_eq!(texts.len(), 5);
    assert!(texts.contains(&ScalarValue::String("Great post!".to_string())));
    assert!(texts.contains(&ScalarValue::String("Thanks for sharing".to_string())));
    assert!(texts.contains(&ScalarValue::String("Another comment".to_string())));
}

#[tokio::test]
async fn test_nested_proxy_empty_relationship() {
    let user = User {
        name: "Bob".to_string(),
        age: 25,
        posts: vec![], // No posts
    };

    let proxy = NestedProxy::from_path("posts", "title");
    let titles = proxy.get_values(&user).await.unwrap();

    assert_eq!(titles.len(), 0);
}
