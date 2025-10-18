//! Path Construction Tests
//!
//! Tests for path construction, prefixes, and URL mounting.

use reinhardt_openapi::{OpenApiSchema, Operation, PathItem};

#[test]
fn test_paths_construction() {
    // Test basic path construction
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path_item = PathItem::default();
    path_item.get = Some(Operation::new());
    path_item.post = Some(Operation::new());

    schema.add_path("/example/".to_string(), path_item);

    assert!(schema.paths.contains_key("/example/"));
    let example_path = &schema.paths["/example/"];
    assert!(example_path.get.is_some());
    assert!(example_path.post.is_some());
}

#[test]
fn test_prefixed_paths_construction() {
    // Test path prefixes
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Add paths with common prefix
    let mut path1 = PathItem::default();
    path1.get = Some(Operation::new());
    schema.add_path("/v1/example/".to_string(), path1);

    let mut path2 = PathItem::default();
    path2.get = Some(Operation::new());
    schema.add_path("/v1/example/{id}/".to_string(), path2);

    assert!(schema.paths.contains_key("/v1/example/"));
    assert!(schema.paths.contains_key("/v1/example/{id}/"));
}

#[test]
fn test_mount_url_prefixed_to_paths() {
    // Test URL mounting with prefix
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Add paths that would be mounted at /api
    let mut path1 = PathItem::default();
    path1.get = Some(Operation::new());
    schema.add_path("/api/example/".to_string(), path1);

    let mut path2 = PathItem::default();
    path2.get = Some(Operation::new());
    schema.add_path("/api/example/{id}/".to_string(), path2);

    assert!(schema.paths.contains_key("/api/example/"));
    assert!(schema.paths.contains_key("/api/example/{id}/"));
}

#[test]
fn test_multiple_paths() {
    // Test multiple different paths
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    schema.add_path("/users/".to_string(), PathItem::default());
    schema.add_path("/posts/".to_string(), PathItem::default());
    schema.add_path("/comments/".to_string(), PathItem::default());

    assert_eq!(schema.paths.len(), 3);
    assert!(schema.paths.contains_key("/users/"));
    assert!(schema.paths.contains_key("/posts/"));
    assert!(schema.paths.contains_key("/comments/"));
}

#[test]
fn test_path_with_multiple_parameters() {
    // Test paths with multiple path parameters
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let path_item = PathItem::default();
    schema.add_path("/users/{user_id}/posts/{post_id}/".to_string(), path_item);

    assert!(schema
        .paths
        .contains_key("/users/{user_id}/posts/{post_id}/"));
}

#[test]
fn test_path_overwrite() {
    // Test that adding a path with the same key overwrites the previous one
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path1 = PathItem::default();
    path1.get = Some(Operation::new());
    schema.add_path("/example/".to_string(), path1);

    let mut path2 = PathItem::default();
    path2.post = Some(Operation::new());
    schema.add_path("/example/".to_string(), path2);

    // Should only have post operation now
    let path = &schema.paths["/example/"];
    assert!(path.get.is_none());
    assert!(path.post.is_some());
}

#[test]
fn test_openapi_path_construction_root() {
    // Test root path
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let path_item = PathItem::default();
    schema.add_path("/".to_string(), path_item);

    assert!(schema.paths.contains_key("/"));
}
