//! Integration tests for WritableNestedSerializer with ORM operations
//!
//! These tests demonstrate the complete workflow of using WritableNestedSerializer
//! with Transaction and ORM operations.

use reinhardt_orm::Model;
use reinhardt_serializers::{Serializer, WritableNestedSerializer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Post {
    pub id: Option<i64>,
    pub title: String,
    pub author_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Author {
    pub id: Option<i64>,
    pub name: String,
    pub email: String,
}

impl Model for Post {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "posts"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
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

#[test]
fn test_writable_nested_serializer_validates_create_permission() {
    // Serializer without create permission
    let serializer = WritableNestedSerializer::<Post, Author>::new("author");

    let json = r#"{
        "title": "My First Post",
        "author": {
            "id": null,
            "name": "Alice",
            "email": "alice@example.com"
        }
    }"#;

    // Should fail - create not allowed
    let result = serializer.deserialize(&json.to_string());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .message()
        .contains("Creating nested instances is not allowed"));
}

#[test]
fn test_writable_nested_serializer_validates_update_permission() {
    // Serializer without update permission
    let serializer = WritableNestedSerializer::<Post, Author>::new("author");

    let json = r#"{
        "title": "My First Post",
        "author": {
            "id": 42,
            "name": "Alice Updated",
            "email": "alice@example.com"
        }
    }"#;

    // Should fail - update not allowed
    let result = serializer.deserialize(&json.to_string());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .message()
        .contains("Updating nested instances is not allowed"));
}

#[test]
fn test_writable_nested_serializer_allows_create_with_permission() {
    // Serializer with create permission
    let serializer = WritableNestedSerializer::<Post, Author>::new("author").allow_create(true);

    let json = r#"{
        "title": "My First Post",
        "author": {
            "id": null,
            "name": "Alice",
            "email": "alice@example.com"
        }
    }"#;

    // Should succeed - create allowed
    let result = serializer.deserialize(&json.to_string());
    assert!(result.is_ok());

    let post = result.unwrap();
    assert_eq!(post.title, "My First Post");
}

#[test]
fn test_writable_nested_serializer_extract_nested_data() {
    let serializer = WritableNestedSerializer::<Post, Author>::new("author").allow_create(true);

    let json = r#"{
        "title": "My First Post",
        "author": {
            "id": null,
            "name": "Alice",
            "email": "alice@example.com"
        }
    }"#;

    // Extract nested author data
    let nested_data = serializer.extract_nested_data(json).unwrap();
    assert!(nested_data.is_some());

    let author_value = nested_data.unwrap();
    assert!(author_value.is_object());
    assert_eq!(author_value.get("name").unwrap().as_str().unwrap(), "Alice");
    assert_eq!(
        author_value.get("email").unwrap().as_str().unwrap(),
        "alice@example.com"
    );

    // Deserialize as Author
    let author: Author = serde_json::from_value(author_value).unwrap();
    assert_eq!(author.name, "Alice");
    assert_eq!(author.email, "alice@example.com");
    assert_eq!(author.id, None); // Create operation
}

#[test]
fn test_writable_nested_serializer_is_create_operation() {
    let create_data = serde_json::json!({
        "id": null,
        "name": "Alice",
        "email": "alice@example.com"
    });

    assert!(WritableNestedSerializer::<Post, Author>::is_create_operation(&create_data));

    let update_data = serde_json::json!({
        "id": 42,
        "name": "Alice Updated",
        "email": "alice@example.com"
    });

    assert!(!WritableNestedSerializer::<Post, Author>::is_create_operation(&update_data));
}

#[test]
fn test_writable_nested_serializer_workflow_simulation() {
    // This test simulates the complete workflow that would be used
    // with actual database operations
    let serializer = WritableNestedSerializer::<Post, Author>::new("author").allow_create(true);

    let json = r#"{
        "title": "My First Post",
        "author": {
            "id": null,
            "name": "Alice",
            "email": "alice@example.com"
        }
    }"#;

    // Step 1: Validate and deserialize
    let post = serializer.deserialize(&json.to_string()).unwrap();
    assert_eq!(post.title, "My First Post");

    // Step 2: Extract nested data
    let author_data = serializer.extract_nested_data(json).unwrap().unwrap();

    // Step 3: Check if it's a create or update operation
    assert!(WritableNestedSerializer::<Post, Author>::is_create_operation(&author_data));

    // Step 4: Deserialize nested object
    let author: Author = serde_json::from_value(author_data).unwrap();
    assert_eq!(author.name, "Alice");
    assert_eq!(author.email, "alice@example.com");

    // In real usage with database:
    // let mut tx = Transaction::new();
    // tx.begin()?;
    // let saved_author = Author::objects().create(&author).await?;
    // post.author_id = saved_author.id;
    // let saved_post = Post::objects().create(&post).await?;
    // tx.commit()?;
}

#[test]
fn test_writable_nested_serializer_with_array_of_nested_objects() {
    let serializer = WritableNestedSerializer::<Author, Post>::new("posts").allow_create(true);

    let json = r#"{
        "name": "Alice",
        "email": "alice@example.com",
        "posts": [
            {
                "id": null,
                "title": "First Post",
                "author_id": null
            },
            {
                "id": null,
                "title": "Second Post",
                "author_id": null
            }
        ]
    }"#;

    // Should succeed - create allowed
    let result = serializer.deserialize(&json.to_string());
    assert!(result.is_ok());

    let author = result.unwrap();
    assert_eq!(author.name, "Alice");

    // Extract nested posts
    let posts_data = serializer.extract_nested_data(json).unwrap();
    assert!(posts_data.is_some());

    let posts_array = posts_data.unwrap();
    assert!(posts_array.is_array());
    assert_eq!(posts_array.as_array().unwrap().len(), 2);

    // Each post can be processed individually
    for post_value in posts_array.as_array().unwrap() {
        let post: Post = serde_json::from_value(post_value.clone()).unwrap();
        assert!(post.id.is_none()); // Create operation
        assert!(post.title.starts_with("First") || post.title.starts_with("Second"));
    }
}

#[test]
fn test_writable_nested_serializer_mixed_create_and_update() {
    let serializer = WritableNestedSerializer::<Author, Post>::new("posts")
        .allow_create(true)
        .allow_update(true);

    let json = r#"{
        "name": "Alice",
        "email": "alice@example.com",
        "posts": [
            {
                "id": null,
                "title": "New Post",
                "author_id": null
            },
            {
                "id": 42,
                "title": "Existing Post Updated",
                "author_id": 1
            }
        ]
    }"#;

    // Should succeed - both create and update allowed
    let result = serializer.deserialize(&json.to_string());
    assert!(result.is_ok());

    // Extract and process posts
    let posts_data = serializer.extract_nested_data(json).unwrap().unwrap();
    let posts_array = posts_data.as_array().unwrap();

    // First post is create operation
    assert!(WritableNestedSerializer::<Author, Post>::is_create_operation(&posts_array[0]));

    // Second post is update operation
    assert!(!WritableNestedSerializer::<Author, Post>::is_create_operation(&posts_array[1]));
}
