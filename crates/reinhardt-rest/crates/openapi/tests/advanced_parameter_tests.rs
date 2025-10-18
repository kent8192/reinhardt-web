//! Advanced Parameter Tests
//!
//! Tests for primary key related fields and parser mapping.

use reinhardt_openapi::{MediaType, Operation, Parameter, ParameterLocation, RequestBody, Schema};
use std::collections::HashMap;

#[test]
fn test_primary_key_related_field() {
    // Test primary key field as path parameter
    let mut operation = Operation::new();

    let id_param = Parameter {
        name: "id".to_string(),
        location: ParameterLocation::Path,
        description: Some("Primary key identifier".to_string()),
        required: Some(true),
        schema: Some(Schema::integer()),
    };

    operation.parameters = Some(vec![id_param]);

    let params = operation.parameters.as_ref().unwrap();
    assert_eq!(params.len(), 1);

    let param = &params[0];
    assert_eq!(param.name, "id");
    assert_eq!(param.location, ParameterLocation::Path);
    assert_eq!(param.required, Some(true));
    assert_eq!(
        param.description,
        Some("Primary key identifier".to_string())
    );

    // Verify schema is integer type
    assert_eq!(
        param.schema.as_ref().unwrap().schema_type,
        Some("integer".to_string())
    );
}

#[test]
fn test_parser_mapping() {
    // Test request body with different Content-Type parsers
    let mut operation = Operation::new();

    // Create request body with multiple content types
    let mut content = HashMap::new();

    // JSON parser
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(Schema::object()),
            example: Some(serde_json::json!({"key": "value"})),
        },
    );

    // Form data parser
    content.insert(
        "application/x-www-form-urlencoded".to_string(),
        MediaType {
            schema: Some(Schema::object()),
            example: None,
        },
    );

    // Multipart form parser
    content.insert(
        "multipart/form-data".to_string(),
        MediaType {
            schema: Some(Schema::object()),
            example: None,
        },
    );

    operation.request_body = Some(RequestBody {
        description: Some("Request with multiple parsers".to_string()),
        content,
        required: Some(true),
    });

    let request_body = operation.request_body.as_ref().unwrap();
    assert_eq!(request_body.required, Some(true));

    let content_map = &request_body.content;
    assert_eq!(content_map.len(), 3);
    assert!(content_map.contains_key("application/json"));
    assert!(content_map.contains_key("application/x-www-form-urlencoded"));
    assert!(content_map.contains_key("multipart/form-data"));

    // Verify JSON parser has example
    let json_media = &content_map["application/json"];
    assert!(json_media.example.is_some());

    // Verify all parsers have object schema
    for media_type in content_map.values() {
        let schema = media_type.schema.as_ref().unwrap();
        assert_eq!(schema.schema_type, Some("object".to_string()));
    }
}
