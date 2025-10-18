//! Tag Tests
//!
//! Tests for custom tags and auto tag generation.

use reinhardt_openapi::{OpenApiSchema, Operation, PathItem};

#[test]
fn test_overridden_tags() {
    // Test custom tags
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    schema.add_tag("users".to_string(), Some("User operations".to_string()));
    schema.add_tag("posts".to_string(), Some("Post operations".to_string()));

    let tags = schema.tags.as_ref().unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].name, "users");
    assert_eq!(tags[0].description, Some("User operations".to_string()));
    assert_eq!(tags[1].name, "posts");
}

#[test]
fn test_auto_generated_apiview_tags() {
    // Test auto tag generation for API views
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Add paths that would auto-generate tags
    let mut path1 = PathItem::default();
    let mut op1 = Operation::new();
    op1.tags = Some(vec!["users".to_string()]);
    path1.get = Some(op1);
    schema.add_path("/users/".to_string(), path1);

    let mut path2 = PathItem::default();
    let mut op2 = Operation::new();
    op2.tags = Some(vec!["posts".to_string()]);
    path2.get = Some(op2);
    schema.add_path("/posts/".to_string(), path2);

    // Verify operations have tags
    let users_path = &schema.paths["/users/"];
    let users_op = users_path.get.as_ref().unwrap();
    assert!(users_op.tags.is_some());
    assert!(users_op
        .tags
        .as_ref()
        .unwrap()
        .contains(&"users".to_string()));

    let posts_path = &schema.paths["/posts/"];
    let posts_op = posts_path.get.as_ref().unwrap();
    assert!(posts_op.tags.is_some());
    assert!(posts_op
        .tags
        .as_ref()
        .unwrap()
        .contains(&"posts".to_string()));
}

#[test]
fn test_tag_without_description() {
    // Test tags without descriptions
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    schema.add_tag("misc".to_string(), None);

    let tags = schema.tags.as_ref().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "misc");
    assert_eq!(tags[0].description, None);
}

#[test]
fn test_multiple_tags_in_operation() {
    // Test operations with multiple tags
    let mut operation = Operation::new();
    operation.tags = Some(vec![
        "users".to_string(),
        "admin".to_string(),
        "deprecated".to_string(),
    ]);

    let tags = operation.tags.as_ref().unwrap();
    assert_eq!(tags.len(), 3);
    assert!(tags.contains(&"users".to_string()));
    assert!(tags.contains(&"admin".to_string()));
    assert!(tags.contains(&"deprecated".to_string()));
}

#[test]
fn test_tag_serialization() {
    // Test that tags serialize correctly
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    schema.add_tag("test".to_string(), Some("Test tag".to_string()));

    let json = schema.to_json().expect("Failed to serialize");

    assert!(json.contains("tags"));
    assert!(json.contains("test"));
    assert!(json.contains("Test tag"));
}

#[test]
fn test_duplicate_tags() {
    // Test adding duplicate tags (should append)
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    schema.add_tag("users".to_string(), Some("First description".to_string()));
    schema.add_tag("users".to_string(), Some("Second description".to_string()));

    let tags = schema.tags.as_ref().unwrap();
    // Both tags are added (duplicates are allowed)
    assert_eq!(tags.len(), 2);
}

#[test]
fn test_empty_tag_list() {
    // Test schema without tags
    let schema = OpenApiSchema::new("Test API", "1.0.0");

    assert!(schema.tags.is_none());
}

#[test]
fn test_operation_without_tags() {
    // Test operation without tags
    let operation = Operation::new();

    assert!(operation.tags.is_none());
}
