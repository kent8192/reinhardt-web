//! Request/Response Body Tests
//!
//! Tests for request bodies, response bodies, and content types.

use reinhardt_openapi::{MediaType, Operation, Response, Schema};
use std::collections::HashMap;

#[test]
fn test_request_body() {
    // Test request body schemas
    use reinhardt_openapi::RequestBody;

    let mut content = HashMap::new();
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(Schema::string()),
            example: None,
        },
    );

    let request_body = RequestBody {
        description: Some("Request body".to_string()),
        content,
        required: Some(true),
    };

    assert_eq!(request_body.description, Some("Request body".to_string()));
    assert_eq!(request_body.required, Some(true));
    assert!(request_body.content.contains_key("application/json"));
}

#[test]
fn test_response_body_generation() {
    // Test response body generation
    let mut content = HashMap::new();
    let mut schema_props = HashMap::new();
    schema_props.insert("id".to_string(), Schema::integer());
    schema_props.insert("name".to_string(), Schema::string());

    let schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(schema_props),
        required: Some(vec!["id".to_string(), "name".to_string()]),
        items: None,
        reference: None,
        description: None,
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };

    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(schema),
            example: None,
        },
    );

    let response = Response {
        description: "Success".to_string(),
        content: Some(content),
        headers: None,
    };

    assert_eq!(response.description, "Success");
    assert!(response.content.is_some());
    let content_map = response.content.as_ref().unwrap();
    assert!(content_map.contains_key("application/json"));
}

#[test]
fn test_list_response_body_generation() {
    // Test list responses
    let item_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: None,
        required: None,
        items: None,
        reference: Some("#/components/schemas/Item".to_string()),
        description: None,
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };

    let list_schema = Schema::array(item_schema);

    let mut content = HashMap::new();
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(list_schema),
            example: None,
        },
    );

    let response = Response {
        description: "List of items".to_string(),
        content: Some(content),
        headers: None,
    };

    let content_map = response.content.as_ref().unwrap();
    let media_type = &content_map["application/json"];
    let schema = media_type.schema.as_ref().unwrap();

    assert_eq!(schema.schema_type, Some("array".to_string()));
    assert!(schema.items.is_some());
}

#[test]
fn test_paginated_list_response_body_generation() {
    // Test paginated responses
    let mut schema_props = HashMap::new();
    schema_props.insert("count".to_string(), Schema::integer());
    schema_props.insert("next".to_string(), Schema::string());
    schema_props.insert("previous".to_string(), Schema::string());

    let item_schema = Schema::reference("#/components/schemas/Item");
    schema_props.insert("results".to_string(), Schema::array(item_schema));

    let paginated_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(schema_props),
        required: Some(vec!["count".to_string(), "results".to_string()]),
        items: None,
        reference: None,
        description: None,
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };

    let mut content = HashMap::new();
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(paginated_schema),
            example: None,
        },
    );

    let response = Response {
        description: "Paginated list".to_string(),
        content: Some(content),
        headers: None,
    };

    let content_map = response.content.as_ref().unwrap();
    let media_type = &content_map["application/json"];
    let schema = media_type.schema.as_ref().unwrap();
    let props = schema.properties.as_ref().unwrap();

    assert!(props.contains_key("count"));
    assert!(props.contains_key("next"));
    assert!(props.contains_key("previous"));
    assert!(props.contains_key("results"));
}

#[test]
fn test_multiple_content_types() {
    // Test multiple content types in response
    let mut content = HashMap::new();
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(Schema::string()),
            example: None,
        },
    );
    content.insert(
        "application/xml".to_string(),
        MediaType {
            schema: Some(Schema::string()),
            example: None,
        },
    );

    let response = Response {
        description: "Multi-format response".to_string(),
        content: Some(content),
        headers: None,
    };

    let content_map = response.content.as_ref().unwrap();
    assert_eq!(content_map.len(), 2);
    assert!(content_map.contains_key("application/json"));
    assert!(content_map.contains_key("application/xml"));
}

#[test]
fn test_response_with_example() {
    // Test response with example value
    use serde_json::json;

    let example = json!({
        "id": 1,
        "name": "Test Item"
    });

    let mut content = HashMap::new();
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(Schema::string()),
            example: Some(example.clone()),
        },
    );

    let response = Response {
        description: "Response with example".to_string(),
        content: Some(content),
        headers: None,
    };

    let content_map = response.content.as_ref().unwrap();
    let media_type = &content_map["application/json"];
    assert!(media_type.example.is_some());
    assert_eq!(media_type.example, Some(example));
}

#[test]
fn test_empty_response() {
    // Test response without content (e.g., 204 No Content)
    let response = Response {
        description: "No content".to_string(),
        content: None,
        headers: None,
    };

    assert_eq!(response.description, "No content");
    assert!(response.content.is_none());
}

#[test]
fn test_response_in_operation() {
    // Test adding responses to operations
    let mut operation = Operation::new();

    let success_response = Response {
        description: "Successful operation".to_string(),
        content: None,
        headers: None,
    };
    operation.add_response("200", success_response);

    let error_response = Response {
        description: "Not found".to_string(),
        content: None,
        headers: None,
    };
    operation.add_response("404", error_response);

    assert_eq!(operation.responses.len(), 2);
    assert!(operation.responses.contains_key("200"));
    assert!(operation.responses.contains_key("404"));
}
