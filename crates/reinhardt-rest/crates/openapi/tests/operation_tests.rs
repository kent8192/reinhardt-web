//! Operation Tests
//!
//! Tests for operation IDs, summaries, and descriptions.

use reinhardt_openapi::{OpenApiSchema, Operation, PathItem, Response};

#[test]
fn test_operation_id_generation() {
    // Test operation ID generation
    let mut operation = Operation::new();
    operation.operation_id = Some("listItems".to_string());

    assert_eq!(operation.operation_id, Some("listItems".to_string()));
}

#[test]
fn test_operation_id_custom_name() {
    // Test custom operation names
    let mut operation = Operation::new();
    operation.operation_id = Some("customOperationName".to_string());
    operation.summary = Some("Custom operation".to_string());

    assert_eq!(
        operation.operation_id,
        Some("customOperationName".to_string())
    );
    assert_eq!(operation.summary, Some("Custom operation".to_string()));
}

#[test]
fn test_operation_id_override_get() {
    // Test override operation IDs for GET method
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path_item = PathItem::default();
    let mut get_op = Operation::new();
    get_op.operation_id = Some("customGetOperation".to_string());
    path_item.get = Some(get_op);

    schema.add_path("/items/".to_string(), path_item);

    let path = &schema.paths["/items/"];
    let op = path.get.as_ref().unwrap();
    assert_eq!(op.operation_id, Some("customGetOperation".to_string()));
}

#[test]
fn test_operation_summary_and_description() {
    // Test operation summary and description
    let mut operation = Operation::new();
    operation.summary = Some("List all items".to_string());
    operation.description = Some("Returns a list of all items in the system".to_string());

    assert_eq!(operation.summary, Some("List all items".to_string()));
    assert_eq!(
        operation.description,
        Some("Returns a list of all items in the system".to_string())
    );
}

#[test]
fn test_operation_tags() {
    // Test operation tags
    let mut operation = Operation::new();
    operation.tags = Some(vec!["items".to_string(), "admin".to_string()]);

    let tags = operation.tags.as_ref().unwrap();
    assert_eq!(tags.len(), 2);
    assert!(tags.contains(&"items".to_string()));
    assert!(tags.contains(&"admin".to_string()));
}

#[test]
fn test_operation_add_response() {
    // Test adding responses to operations
    let mut operation = Operation::new();

    let response_200 = Response {
        description: "Successful response".to_string(),
        content: None,
        headers: None,
    };
    operation.add_response("200", response_200);

    let response_404 = Response {
        description: "Not found".to_string(),
        content: None,
        headers: None,
    };
    operation.add_response("404", response_404);

    assert_eq!(operation.responses.len(), 2);
    assert!(operation.responses.contains_key("200"));
    assert!(operation.responses.contains_key("404"));
}

#[test]
fn test_operation_all_http_methods() {
    // Test all HTTP methods in a path item
    let mut path_item = PathItem::default();

    let mut get_op = Operation::new();
    get_op.operation_id = Some("getItem".to_string());
    path_item.get = Some(get_op);

    let mut post_op = Operation::new();
    post_op.operation_id = Some("createItem".to_string());
    path_item.post = Some(post_op);

    let mut put_op = Operation::new();
    put_op.operation_id = Some("updateItem".to_string());
    path_item.put = Some(put_op);

    let mut patch_op = Operation::new();
    patch_op.operation_id = Some("patchItem".to_string());
    path_item.patch = Some(patch_op);

    let mut delete_op = Operation::new();
    delete_op.operation_id = Some("deleteItem".to_string());
    path_item.delete = Some(delete_op);

    assert!(path_item.get.is_some());
    assert!(path_item.post.is_some());
    assert!(path_item.put.is_some());
    assert!(path_item.patch.is_some());
    assert!(path_item.delete.is_some());
}

#[test]
fn test_operation_default() {
    // Test default operation creation
    let operation = Operation::default();

    assert!(operation.tags.is_none());
    assert!(operation.summary.is_none());
    assert!(operation.description.is_none());
    assert!(operation.operation_id.is_none());
    assert!(operation.responses.is_empty());
}

#[test]
fn test_operation_security() {
    // Test operation security requirements
    use std::collections::HashMap;

    let mut operation = Operation::new();
    let mut security_req = HashMap::new();
    security_req.insert("api_key".to_string(), vec![]);
    operation.security = Some(vec![security_req]);

    assert!(operation.security.is_some());
    let security = operation.security.as_ref().unwrap();
    assert_eq!(security.len(), 1);
}
