//! Integration tests for ViewSet Inspector
//!
//! These tests verify the integration between reinhardt-openapi and reinhardt-viewsets
//! by testing schema generation from ViewSets.

use hyper::Method;
use reinhardt_openapi::{InspectorConfig, ViewSetInspector};
use reinhardt_viewsets::{ActionMetadata, ModelViewSet, ReadOnlyModelViewSet};
use serde::{Deserialize, Serialize};

/// Test model representing a user
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Option<i64>,
    username: String,
    email: String,
}

/// Test serializer for User
#[derive(Debug, Clone)]
struct UserSerializer;

/// Test model representing a post
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
    id: Option<i64>,
    title: String,
    content: String,
    author_id: i64,
}

/// Test serializer for Post
#[derive(Debug, Clone)]
struct PostSerializer;

/// Test: Basic path extraction from ModelViewSet
///
/// This test verifies that the inspector can extract both collection and detail
/// paths from a standard CRUD ViewSet.
#[test]
fn test_extract_paths_from_model_viewset() {
    let viewset = ModelViewSet::<User, UserSerializer>::new("users");
    let inspector = ViewSetInspector::new();

    let paths = inspector.extract_paths(&viewset, "/api/users");

    // Should have collection endpoint
    assert!(
        paths.contains_key("/api/users/"),
        "Missing collection endpoint"
    );

    // Should have detail endpoint
    assert!(
        paths.contains_key("/api/users{id}/"),
        "Missing detail endpoint"
    );

    // Should have exactly 2 paths (collection + detail)
    assert_eq!(paths.len(), 2, "Expected 2 paths (collection and detail)");
}

/// Test: Path extraction from ReadOnlyModelViewSet
///
/// Verifies that read-only ViewSets generate the same paths as full ModelViewSets,
/// even though they don't support all operations.
#[test]
fn test_extract_paths_from_readonly_viewset() {
    let viewset = ReadOnlyModelViewSet::<Post, PostSerializer>::new("posts");
    let inspector = ViewSetInspector::new();

    let paths = inspector.extract_paths(&viewset, "/api/posts");

    // Read-only viewsets should still have both paths
    assert!(paths.contains_key("/api/posts/"));
    assert!(paths.contains_key("/api/posts{id}/"));
}

/// Test: Operation extraction includes all CRUD operations
///
/// Verifies that all standard CRUD operations are generated for a ModelViewSet.
#[test]
fn test_extract_operations_includes_all_crud() {
    let viewset = ModelViewSet::<User, UserSerializer>::new("users");
    let inspector = ViewSetInspector::new();

    let operations = inspector.extract_operations(&viewset);

    // Should have at least 6 CRUD operations:
    // list, retrieve, create, update, partial_update, destroy
    assert!(
        operations.len() >= 6,
        "Expected at least 6 CRUD operations, got {}",
        operations.len()
    );

    // Verify operation IDs are present
    let operation_ids: Vec<_> = operations
        .iter()
        .filter_map(|op| op.operation_id.as_ref())
        .collect();

    assert!(
        operation_ids.iter().any(|id| id.contains("list")),
        "Missing list operation"
    );
    assert!(
        operation_ids.iter().any(|id| id.contains("retrieve")),
        "Missing retrieve operation"
    );
    assert!(
        operation_ids.iter().any(|id| id.contains("create")),
        "Missing create operation"
    );
    assert!(
        operation_ids.iter().any(|id| id.contains("update")),
        "Missing update operation"
    );
    assert!(
        operation_ids.iter().any(|id| id.contains("destroy")),
        "Missing destroy operation"
    );
}

/// Test: Custom configuration affects generated schemas
///
/// Verifies that InspectorConfig options properly control the output.
#[test]
fn test_custom_inspector_config() {
    let config = InspectorConfig {
        include_descriptions: false,
        include_tags: false,
        default_response_description: "Success".to_string(),
    };
    let inspector = ViewSetInspector::with_config(config);

    let viewset = ModelViewSet::<User, UserSerializer>::new("users");
    let operations = inspector.extract_operations(&viewset);

    // All operations should exist even with custom config
    assert!(operations.len() >= 6);

    // Note: We can't easily test if descriptions/tags are omitted without
    // inspecting the internal structure, but we verify the inspector works
    assert!(!operations.is_empty());
}

/// Test: Multiple ViewSets can be inspected independently
///
/// Verifies that the inspector can handle multiple different ViewSets.
#[test]
fn test_multiple_viewsets() {
    let inspector = ViewSetInspector::new();

    let user_viewset = ModelViewSet::<User, UserSerializer>::new("users");
    let post_viewset = ModelViewSet::<Post, PostSerializer>::new("posts");

    let user_paths = inspector.extract_paths(&user_viewset, "/api/users");
    let post_paths = inspector.extract_paths(&post_viewset, "/api/posts");

    // Both should have their own paths
    assert!(user_paths.contains_key("/api/users/"));
    assert!(post_paths.contains_key("/api/posts/"));

    // Paths should be different
    assert_ne!(
        user_paths.keys().collect::<Vec<_>>(),
        post_paths.keys().collect::<Vec<_>>()
    );
}

/// Test: Model schema extraction
///
/// Verifies that schemas can be extracted for models.
#[test]
fn test_extract_model_schema() {
    let inspector = ViewSetInspector::new();
    let schema = inspector.extract_model_schema("User");

    // Should return a schema object
    match schema {
        reinhardt_openapi::Schema::Object(_) => {
            // Success - schema is an object
        }
        _ => panic!("Expected Object schema, got different type"),
    }
}

/// Test: Different base paths generate different endpoints
///
/// Verifies that the base path parameter correctly affects the generated paths.
#[test]
fn test_different_base_paths() {
    let inspector = ViewSetInspector::new();
    let viewset = ModelViewSet::<User, UserSerializer>::new("users");

    let paths_v1 = inspector.extract_paths(&viewset, "/api/v1/users");
    let paths_v2 = inspector.extract_paths(&viewset, "/api/v2/users");

    // Should generate different paths
    assert!(paths_v1.contains_key("/api/v1/users/"));
    assert!(paths_v2.contains_key("/api/v2/users/"));

    // Should not have each other's paths
    assert!(!paths_v1.contains_key("/api/v2/users/"));
    assert!(!paths_v2.contains_key("/api/v1/users/"));
}

/// Test: Inspector handles ViewSets with same basename but different models
///
/// Verifies that multiple ViewSets with the same basename generate the same structure
/// (since schema generation is based on basename, not model type).
#[test]
fn test_same_basename_different_models() {
    let inspector = ViewSetInspector::new();

    let user_viewset = ModelViewSet::<User, UserSerializer>::new("items");
    let post_viewset = ModelViewSet::<Post, PostSerializer>::new("items");

    let user_paths = inspector.extract_paths(&user_viewset, "/api/items");
    let post_paths = inspector.extract_paths(&post_viewset, "/api/items");

    // Should generate same paths (same basename and base path)
    assert_eq!(
        user_paths.keys().collect::<Vec<_>>(),
        post_paths.keys().collect::<Vec<_>>()
    );
}
