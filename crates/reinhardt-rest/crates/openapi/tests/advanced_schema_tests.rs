//! Advanced Schema Tests
//!
//! Tests for request/response schema separation, custom schemas, and component deduplication.

use reinhardt_openapi::openapi::Components;
use reinhardt_openapi::{MediaType, OpenApiSchema, Operation, PathItem, Response, Schema};
use std::collections::HashMap;

#[test]
fn test_different_request_response_objects() {
    // Test request and response with different schemas
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Define Request schema
    let mut request_props = HashMap::new();
    request_props.insert("text".to_string(), Schema::string());

    let request_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(request_props),
        required: Some(vec!["text".to_string()]),
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

    // Define Response schema
    let mut response_props = HashMap::new();
    response_props.insert("text".to_string(), Schema::boolean());

    let response_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(response_props),
        required: Some(vec!["text".to_string()]),
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

    // Add schemas to components
    let mut components = Components::default();
    components.add_schema("Request".to_string(), request_schema);
    components.add_schema("Response".to_string(), response_schema);

    schema.components = Some(components);

    // Verify components
    let comps = schema.components.as_ref().unwrap();
    let schemas_map = comps.schemas.as_ref().unwrap();

    assert!(schemas_map.contains_key("Request"));
    assert!(schemas_map.contains_key("Response"));

    // Verify Request schema
    let req_schema = &schemas_map["Request"];
    let req_props = req_schema.properties.as_ref().unwrap();
    assert_eq!(req_props["text"].schema_type, Some("string".to_string()));

    // Verify Response schema
    let res_schema = &schemas_map["Response"];
    let res_props = res_schema.properties.as_ref().unwrap();
    assert_eq!(res_props["text"].schema_type, Some("boolean".to_string()));
}

#[test]
fn test_custom_response_schema() {
    // Test custom response schema override
    let mut operation = Operation::new();

    // Create a custom response with specific schema
    let mut custom_schema_props = HashMap::new();
    custom_schema_props.insert("id".to_string(), Schema::integer());
    custom_schema_props.insert("message".to_string(), Schema::string());

    let custom_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(custom_schema_props),
        required: Some(vec!["id".to_string()]),
        items: None,
        reference: None,
        description: Some("Custom response".to_string()),
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
            schema: Some(custom_schema),
            example: None,
        },
    );

    operation.add_response(
        "200",
        Response {
            description: "Custom response".to_string(),
            content: Some(content),
            headers: None,
        },
    );

    let response = &operation.responses["200"];
    assert_eq!(response.description, "Custom response");

    let content_map = response.content.as_ref().unwrap();
    let media_type = &content_map["application/json"];
    let schema = media_type.schema.as_ref().unwrap();

    assert_eq!(schema.description, Some("Custom response".to_string()));
    assert!(schema.properties.is_some());
}

#[test]
fn test_component_name_deduplication() {
    // Test automatic renaming of duplicate component names
    let mut components = Components::default();

    let schema1 = Schema::string();
    let schema2 = Schema::integer();
    let schema3 = Schema::boolean();

    // Add first schema with name "Item"
    let name1 = components.add_schema_with_dedup("Item".to_string(), schema1);
    assert_eq!(name1, "Item");

    // Add second schema with same name "Item" - should be renamed to "Item2"
    let name2 = components.add_schema_with_dedup("Item".to_string(), schema2);
    assert_eq!(name2, "Item2");

    // Add third schema with same name "Item" - should be renamed to "Item3"
    let name3 = components.add_schema_with_dedup("Item".to_string(), schema3);
    assert_eq!(name3, "Item3");

    // Verify all schemas exist
    let schemas_map = components.schemas.as_ref().unwrap();
    assert!(schemas_map.contains_key("Item"));
    assert!(schemas_map.contains_key("Item2"));
    assert!(schemas_map.contains_key("Item3"));
    assert_eq!(schemas_map.len(), 3);
}

#[test]
fn test_serializer_model_components() {
    // Test model serializer schema in components
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Create a User model schema
    let mut user_props = HashMap::new();
    user_props.insert("id".to_string(), Schema::integer());
    user_props.insert("username".to_string(), Schema::string());
    user_props.insert("email".to_string(), Schema::string());
    user_props.insert("created_at".to_string(), Schema::datetime());

    let user_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(user_props),
        required: Some(vec![
            "id".to_string(),
            "username".to_string(),
            "email".to_string(),
        ]),
        items: None,
        reference: None,
        description: Some("User model".to_string()),
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };

    let mut components = Components::default();
    components.add_schema("User".to_string(), user_schema);

    schema.components = Some(components);

    // Verify the schema
    let comps = schema.components.as_ref().unwrap();
    let schemas_map = comps.schemas.as_ref().unwrap();

    assert!(schemas_map.contains_key("User"));

    let user = &schemas_map["User"];
    assert_eq!(user.schema_type, Some("object".to_string()));
    assert_eq!(user.description, Some("User model".to_string()));

    let props = user.properties.as_ref().unwrap();
    assert_eq!(props.len(), 4);
    assert!(props.contains_key("id"));
    assert!(props.contains_key("username"));
    assert!(props.contains_key("email"));
    assert!(props.contains_key("created_at"));

    // Verify datetime format
    assert_eq!(props["created_at"].format, Some("date-time".to_string()));
}
