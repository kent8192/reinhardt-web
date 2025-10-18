//! Format and Validation Tests
//!
//! Tests for date fields, validation constraints, and operation ID handling.

use reinhardt_openapi::{OpenApiSchema, Operation, PathItem, Schema};
use serde_json::json;

#[test]
fn test_serializer_datefield() {
    // Test date field with format: "date"
    let date_schema = Schema::date();

    assert_eq!(date_schema.schema_type, Some("string".to_string()));
    assert_eq!(date_schema.format, Some("date".to_string()));

    // Test that it serializes correctly
    let json = serde_json::to_value(&date_schema).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["format"], "date");
}

#[test]
fn test_serializer_validators() {
    // Test min/max/pattern validation constraints
    let mut validated_string = Schema::string();
    validated_string.min_length = Some(3);
    validated_string.max_length = Some(50);
    validated_string.pattern = Some("^[a-zA-Z]+$".to_string());

    assert_eq!(validated_string.min_length, Some(3));
    assert_eq!(validated_string.max_length, Some(50));
    assert_eq!(validated_string.pattern, Some("^[a-zA-Z]+$".to_string()));

    // Test min/max for numbers
    let mut validated_number = Schema::number();
    validated_number.minimum = Some(0.0);
    validated_number.maximum = Some(100.0);

    assert_eq!(validated_number.minimum, Some(0.0));
    assert_eq!(validated_number.maximum, Some(100.0));

    // Test enum values
    let mut enum_schema = Schema::string();
    enum_schema.enum_values = Some(vec![json!("active"), json!("inactive"), json!("pending")]);

    assert!(enum_schema.enum_values.is_some());
    let enum_vals = enum_schema.enum_values.as_ref().unwrap();
    assert_eq!(enum_vals.len(), 3);

    // Test JSON serialization includes validation fields
    let json = serde_json::to_value(&validated_string).unwrap();
    assert_eq!(json["minLength"], 3);
    assert_eq!(json["maxLength"], 50);
    assert_eq!(json["pattern"], "^[a-zA-Z]+$");
}

#[test]
fn test_operation_id_plural() {
    // Test plural resource names generate appropriate operation IDs
    let mut operation = Operation::new();
    operation.operation_id = Some("listItems".to_string());
    operation.summary = Some("List all items".to_string());

    assert_eq!(operation.operation_id, Some("listItems".to_string()));

    // Test other plural operations
    let mut create_op = Operation::new();
    create_op.operation_id = Some("createItem".to_string());
    assert_eq!(create_op.operation_id, Some("createItem".to_string()));

    let mut update_op = Operation::new();
    update_op.operation_id = Some("updateItem".to_string());
    assert_eq!(update_op.operation_id, Some("updateItem".to_string()));

    let mut delete_op = Operation::new();
    delete_op.operation_id = Some("deleteItem".to_string());
    assert_eq!(delete_op.operation_id, Some("deleteItem".to_string()));
}

#[test]
fn test_duplicate_operation_id() {
    // Test detection of duplicate operation IDs
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path1 = PathItem::default();
    let mut op1 = Operation::new();
    op1.operation_id = Some("getItem".to_string());
    path1.get = Some(op1);

    let mut path2 = PathItem::default();
    let mut op2 = Operation::new();
    op2.operation_id = Some("getItem".to_string()); // Duplicate!
    path2.get = Some(op2);

    schema.add_path("/items/".to_string(), path1);
    schema.add_path("/products/".to_string(), path2);

    // Collect all operation IDs
    let mut operation_ids = Vec::new();
    for path_item in schema.paths.values() {
        if let Some(ref op) = path_item.get {
            if let Some(ref id) = op.operation_id {
                operation_ids.push(id.clone());
            }
        }
    }

    // Check for duplicates
    operation_ids.sort();
    let unique_count = operation_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();

    // We have 2 operation IDs but only 1 unique
    assert_eq!(operation_ids.len(), 2);
    assert_eq!(unique_count, 1); // Duplicate detected!
}

#[test]
fn test_datetime_format() {
    // Test datetime field with format: "date-time"
    let datetime_schema = Schema::datetime();

    assert_eq!(datetime_schema.schema_type, Some("string".to_string()));
    assert_eq!(datetime_schema.format, Some("date-time".to_string()));

    // Test JSON serialization
    let json = serde_json::to_value(&datetime_schema).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["format"], "date-time");
}

#[test]
fn test_default_values() {
    // Test default values in schemas
    let mut bool_schema = Schema::boolean();
    bool_schema.default = Some(json!(true));

    assert_eq!(bool_schema.default, Some(json!(true)));

    let mut string_schema = Schema::string();
    string_schema.default = Some(json!("default value"));

    assert_eq!(string_schema.default, Some(json!("default value")));

    let mut number_schema = Schema::number();
    number_schema.default = Some(json!(42.5));

    assert_eq!(number_schema.default, Some(json!(42.5)));

    // Test JSON serialization includes default
    let json = serde_json::to_value(&bool_schema).unwrap();
    assert_eq!(json["default"], true);
}

#[test]
fn test_enum_with_different_types() {
    // Test enum values with different JSON types
    let mut enum_schema = Schema::string();
    enum_schema.enum_values = Some(vec![json!("option1"), json!("option2"), json!("option3")]);

    let json = serde_json::to_value(&enum_schema).unwrap();
    assert!(json["enum"].is_array());
    assert_eq!(json["enum"].as_array().unwrap().len(), 3);

    // Test integer enum
    let mut int_enum = Schema::integer();
    int_enum.enum_values = Some(vec![json!(1), json!(2), json!(3)]);

    let json = serde_json::to_value(&int_enum).unwrap();
    assert_eq!(json["enum"][0], 1);
    assert_eq!(json["enum"][1], 2);
    assert_eq!(json["enum"][2], 3);
}
