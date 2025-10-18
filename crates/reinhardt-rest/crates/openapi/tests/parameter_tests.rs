//! Parameter Tests
//!
//! Tests for path parameters, query parameters, and parameter inclusion.

use reinhardt_openapi::{OpenApiSchema, Operation, Parameter, ParameterLocation, PathItem, Schema};

#[test]
fn test_path_without_parameters() {
    // Test paths without parameters
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let path_item = PathItem::default();
    schema.add_path("/items/".to_string(), path_item);

    let path = &schema.paths["/items/"];
    assert!(path.parameters.is_none());
}

#[test]
fn test_path_with_id_parameter() {
    // Test ID parameters in path
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path_item = PathItem::default();
    let id_param = Parameter {
        name: "id".to_string(),
        location: ParameterLocation::Path,
        description: Some("Item ID".to_string()),
        required: Some(true),
        schema: Some(Schema::integer()),
    };
    path_item.parameters = Some(vec![id_param]);

    schema.add_path("/items/{id}/".to_string(), path_item);

    let path = &schema.paths["/items/{id}/"];
    assert!(path.parameters.is_some());
    let params = path.parameters.as_ref().unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "id");
}

#[test]
fn test_param_include_in_schema() {
    // Test parameter inclusion control
    let mut operation = Operation::new();

    // Add a query parameter
    let search_param = Parameter {
        name: "search".to_string(),
        location: ParameterLocation::Query,
        description: Some("Search query".to_string()),
        required: Some(false),
        schema: Some(Schema::string()),
    };

    operation.parameters = Some(vec![search_param]);

    let params = operation.parameters.as_ref().unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "search");
}

#[test]
fn test_parameter_locations() {
    // Test all parameter locations
    let path_param = Parameter {
        name: "id".to_string(),
        location: ParameterLocation::Path,
        description: None,
        required: Some(true),
        schema: Some(Schema::integer()),
    };

    let query_param = Parameter {
        name: "filter".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: Some(false),
        schema: Some(Schema::string()),
    };

    let header_param = Parameter {
        name: "X-Api-Key".to_string(),
        location: ParameterLocation::Header,
        description: None,
        required: Some(true),
        schema: Some(Schema::string()),
    };

    let cookie_param = Parameter {
        name: "session".to_string(),
        location: ParameterLocation::Cookie,
        description: None,
        required: Some(false),
        schema: Some(Schema::string()),
    };

    // Verify they serialize correctly
    let serialized = serde_json::to_value(&path_param).unwrap();
    assert_eq!(serialized["in"], "path");

    let serialized = serde_json::to_value(&query_param).unwrap();
    assert_eq!(serialized["in"], "query");

    let serialized = serde_json::to_value(&header_param).unwrap();
    assert_eq!(serialized["in"], "header");

    let serialized = serde_json::to_value(&cookie_param).unwrap();
    assert_eq!(serialized["in"], "cookie");
}

#[test]
fn test_openapi_multiple_parameters() {
    // Test multiple parameters in a single operation
    let mut operation = Operation::new();

    let params = vec![
        Parameter {
            name: "id".to_string(),
            location: ParameterLocation::Path,
            description: Some("Resource ID".to_string()),
            required: Some(true),
            schema: Some(Schema::integer()),
        },
        Parameter {
            name: "include".to_string(),
            location: ParameterLocation::Query,
            description: Some("Related resources to include".to_string()),
            required: Some(false),
            schema: Some(Schema::string()),
        },
        Parameter {
            name: "Authorization".to_string(),
            location: ParameterLocation::Header,
            description: Some("Bearer token".to_string()),
            required: Some(true),
            schema: Some(Schema::string()),
        },
    ];

    operation.parameters = Some(params);

    let op_params = operation.parameters.as_ref().unwrap();
    assert_eq!(op_params.len(), 3);
}

#[test]
fn test_parameter_required_optional() {
    // Test required and optional parameters
    let required_param = Parameter {
        name: "required_field".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: Some(true),
        schema: Some(Schema::string()),
    };

    let optional_param = Parameter {
        name: "optional_field".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: Some(false),
        schema: Some(Schema::string()),
    };

    assert_eq!(required_param.required, Some(true));
    assert_eq!(optional_param.required, Some(false));
}

#[test]
fn test_parameter_with_description() {
    // Test parameters with descriptions
    let param = Parameter {
        name: "page".to_string(),
        location: ParameterLocation::Query,
        description: Some("Page number for pagination".to_string()),
        required: Some(false),
        schema: Some(Schema::integer()),
    };

    assert_eq!(
        param.description,
        Some("Page number for pagination".to_string())
    );
}

#[test]
fn test_parameter_schema_types() {
    // Test different parameter schema types
    let int_param = Parameter {
        name: "count".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: None,
        schema: Some(Schema::integer()),
    };

    let str_param = Parameter {
        name: "name".to_string(),
        location: ParameterLocation::Query,
        description: None,
        required: None,
        schema: Some(Schema::string()),
    };

    assert_eq!(
        int_param.schema.as_ref().unwrap().schema_type,
        Some("integer".to_string())
    );
    assert_eq!(
        str_param.schema.as_ref().unwrap().schema_type,
        Some("string".to_string())
    );
}
