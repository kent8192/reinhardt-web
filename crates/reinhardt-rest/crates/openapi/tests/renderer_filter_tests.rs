//! Renderer and Filter Tests
//!
//! Tests for response renderer mapping and query parameter filter schemas.

use reinhardt_openapi::{MediaType, Operation, Parameter, ParameterLocation, Response, Schema};
use std::collections::HashMap;

#[test]
fn test_renderer_mapping() {
    // Test response with different Content-Type renderers
    let mut operation = Operation::new();

    // Create response with multiple content types
    let mut content = HashMap::new();

    // JSON renderer
    content.insert(
        "application/json".to_string(),
        MediaType {
            schema: Some(Schema::object()),
            example: Some(serde_json::json!({"status": "success"})),
        },
    );

    // XML renderer
    content.insert(
        "application/xml".to_string(),
        MediaType {
            schema: Some(Schema::object()),
            example: None,
        },
    );

    // Plain text renderer
    content.insert(
        "text/plain".to_string(),
        MediaType {
            schema: Some(Schema::string()),
            example: None,
        },
    );

    operation.add_response(
        "200",
        Response {
            description: "Response with multiple renderers".to_string(),
            content: Some(content),
            headers: None,
        },
    );

    let response = &operation.responses["200"];
    assert_eq!(response.description, "Response with multiple renderers");

    let content_map = response.content.as_ref().unwrap();
    assert_eq!(content_map.len(), 3);
    assert!(content_map.contains_key("application/json"));
    assert!(content_map.contains_key("application/xml"));
    assert!(content_map.contains_key("text/plain"));

    // Verify JSON renderer has example
    let json_media = &content_map["application/json"];
    assert!(json_media.example.is_some());

    // Verify text/plain uses string schema
    let text_media = &content_map["text/plain"];
    let text_schema = text_media.schema.as_ref().unwrap();
    assert_eq!(text_schema.schema_type, Some("string".to_string()));
}

#[test]
fn test_filters() {
    // Test query parameter filter schemas
    let mut operation = Operation::new();

    // Create parameters
    let search_param = Parameter {
        name: "search".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: None,
        schema: Some(Schema::string()),
    };

    let mut ordering_schema = Schema::string();
    ordering_schema.enum_values = Some(vec![
        serde_json::json!("name"),
        serde_json::json!("-name"),
        serde_json::json!("created_at"),
        serde_json::json!("-created_at"),
    ]);

    let ordering_param = Parameter {
        name: "ordering".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: None,
        schema: Some(ordering_schema),
    };

    let mut page_size_schema = Schema::integer();
    page_size_schema.minimum = Some(1.0);
    page_size_schema.maximum = Some(100.0);

    let page_size_param = Parameter {
        name: "page_size".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: None,
        schema: Some(page_size_schema),
    };

    let mut status_schema = Schema::string();
    status_schema.enum_values = Some(vec![
        serde_json::json!("active"),
        serde_json::json!("inactive"),
        serde_json::json!("pending"),
    ]);

    let status_param = Parameter {
        name: "status".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: None,
        schema: Some(status_schema),
    };

    operation.parameters = Some(vec![
        search_param,
        ordering_param,
        page_size_param,
        status_param,
    ]);

    // Verify all parameters
    let params = operation.parameters.as_ref().unwrap();
    assert_eq!(params.len(), 4);

    // Verify search filter
    let search = &params[0];
    assert_eq!(search.name, "search");
    assert_eq!(search.location, ParameterLocation::Query);

    // Verify ordering filter has enum
    let ordering = &params[1];
    assert_eq!(ordering.name, "ordering");
    assert!(ordering.schema.as_ref().unwrap().enum_values.is_some());
    assert_eq!(
        ordering
            .schema
            .as_ref()
            .unwrap()
            .enum_values
            .as_ref()
            .unwrap()
            .len(),
        4
    );

    // Verify page_size has constraints
    let page_size = &params[2];
    assert_eq!(page_size.name, "page_size");
    assert_eq!(page_size.schema.as_ref().unwrap().minimum, Some(1.0));
    assert_eq!(page_size.schema.as_ref().unwrap().maximum, Some(100.0));

    // Verify status filter has enum
    let status = &params[3];
    assert_eq!(status.name, "status");
    assert!(status.schema.as_ref().unwrap().enum_values.is_some());
    assert_eq!(
        status
            .schema
            .as_ref()
            .unwrap()
            .enum_values
            .as_ref()
            .unwrap()
            .len(),
        3
    );
}
