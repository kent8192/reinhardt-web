//! Components/Schemas Tests
//!
//! Tests for component schemas, naming, and duplicate handling.

use reinhardt_openapi::openapi::Components;
use reinhardt_openapi::{OpenApiSchema, Schema};
use std::collections::HashMap;

#[test]
fn test_serializer_model() {
    // Test model serializer schemas in components
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut item_props = HashMap::new();
    item_props.insert("id".to_string(), Schema::integer());
    item_props.insert("name".to_string(), Schema::string());

    let item_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(item_props),
        required: Some(vec!["id".to_string()]),
        items: None,
        reference: None,
        description: Some("Item model".to_string()),
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };

    let mut schemas = HashMap::new();
    schemas.insert("Item".to_string(), item_schema);

    schema.components = Some(Components {
        schemas: Some(schemas),
        security_schemes: None,
    });

    assert!(schema.components.is_some());
    let components = schema.components.as_ref().unwrap();
    assert!(components.schemas.is_some());
    let schemas_map = components.schemas.as_ref().unwrap();
    assert!(schemas_map.contains_key("Item"));
}

#[test]
fn test_component_name() {
    // Test component naming
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let user_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: None,
        required: None,
        items: None,
        reference: None,
        description: Some("User component".to_string()),
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };

    let mut schemas = HashMap::new();
    schemas.insert("User".to_string(), user_schema);

    schema.components = Some(Components {
        schemas: Some(schemas),
        security_schemes: None,
    });

    let components = schema.components.as_ref().unwrap();
    let schemas_map = components.schemas.as_ref().unwrap();

    // Component name should be "User"
    assert!(schemas_map.contains_key("User"));
    assert!(!schemas_map.contains_key("user"));
}

#[test]
fn test_duplicate_component_name() {
    // Test duplicate handling - later one overwrites
    let mut schemas = HashMap::new();

    let schema1 = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: None,
        required: None,
        items: None,
        reference: None,
        description: Some("First version".to_string()),
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };
    schemas.insert("Item".to_string(), schema1);

    let schema2 = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: None,
        required: None,
        items: None,
        reference: None,
        description: Some("Second version".to_string()),
        minimum: None,
        maximum: None,
        pattern: None,
        enum_values: None,
        min_length: None,
        max_length: None,
        default: None,
    };
    schemas.insert("Item".to_string(), schema2);

    // Should only have one entry with the second description
    assert_eq!(schemas.len(), 1);
    assert_eq!(
        schemas["Item"].description,
        Some("Second version".to_string())
    );
}

#[test]
fn test_multiple_components() {
    // Test multiple component schemas
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut schemas = HashMap::new();
    schemas.insert("User".to_string(), Schema::string());
    schemas.insert("Post".to_string(), Schema::string());
    schemas.insert("Comment".to_string(), Schema::string());

    schema.components = Some(Components {
        schemas: Some(schemas),
        security_schemes: None,
    });

    let components = schema.components.as_ref().unwrap();
    let schemas_map = components.schemas.as_ref().unwrap();

    assert_eq!(schemas_map.len(), 3);
    assert!(schemas_map.contains_key("User"));
    assert!(schemas_map.contains_key("Post"));
    assert!(schemas_map.contains_key("Comment"));
}

#[test]
fn test_component_reference() {
    // Test referencing components
    let ref_schema = Schema::reference("#/components/schemas/User");

    assert!(ref_schema.reference.is_some());
    assert_eq!(
        ref_schema.reference,
        Some("#/components/schemas/User".to_string())
    );
    assert!(ref_schema.schema_type.is_none());
}

#[test]
fn test_nested_component_properties() {
    // Test nested properties in component schemas
    let mut user_props = HashMap::new();
    user_props.insert("id".to_string(), Schema::integer());
    user_props.insert("name".to_string(), Schema::string());

    let address_schema = Schema::reference("#/components/schemas/Address");
    user_props.insert("address".to_string(), address_schema);

    let user_schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(user_props),
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

    let props = user_schema.properties.as_ref().unwrap();
    assert_eq!(props.len(), 3);
    assert!(props["address"].reference.is_some());
}

#[test]
fn test_component_required_fields() {
    // Test required fields in component schemas
    let mut props = HashMap::new();
    props.insert("id".to_string(), Schema::integer());
    props.insert("name".to_string(), Schema::string());
    props.insert("email".to_string(), Schema::string());

    let schema = Schema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(props),
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

    let required = schema.required.as_ref().unwrap();
    assert_eq!(required.len(), 2);
    assert!(required.contains(&"id".to_string()));
    assert!(required.contains(&"name".to_string()));
    assert!(!required.contains(&"email".to_string()));
}

#[test]
fn test_component_schema_in_json() {
    // Test that components serialize correctly in JSON
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut schemas = HashMap::new();
    schemas.insert("User".to_string(), Schema::string());

    schema.components = Some(Components {
        schemas: Some(schemas),
        security_schemes: None,
    });

    let json = schema.to_json().expect("Failed to serialize");

    assert!(json.contains("components"));
    assert!(json.contains("schemas"));
    assert!(json.contains("User"));
}
