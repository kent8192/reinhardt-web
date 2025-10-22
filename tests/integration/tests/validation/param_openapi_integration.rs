//! Integration tests for parameter OpenAPI schema generation
//!
//! These tests verify that reinhardt-openapi generates correct schemas for parameters.

use reinhardt_openapi::{OpenApiSchema, Operation, Parameter, ParameterLocation, PathItem, Schema};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Query Parameter OpenAPI Tests
// ============================================================================

#[test]
fn test_query_param_basic_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Create operation with basic query parameter
    let mut operation = Operation {
        tags: None,
        summary: Some("Get items".to_string()),
        description: None,
        operation_id: Some("get_items".to_string()),
        parameters: Some(vec![Parameter {
            name: "limit".to_string(),
            location: ParameterLocation::Query,
            description: Some("Maximum number of items".to_string()),
            required: Some(false), // Query params are optional by default
            schema: Some({
                let mut s = Schema::integer();
                s.format = Some("int32".to_string());
                s.default = Some(Value::Number(10.into()));
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/items".to_string(), path_item);

    // Verify schema
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json.get("paths").unwrap();
    let get_op = paths.get("/items").unwrap().get("get").unwrap();
    let parameters = get_op.get("parameters").unwrap().as_array().unwrap();

    assert_eq!(parameters.len(), 1);
    let param = &parameters[0];
    assert_eq!(param["name"], "limit");
    assert_eq!(param["in"], "query");
    assert_eq!(param["required"], false);
    assert_eq!(param["schema"]["type"], "integer");
    assert_eq!(param["schema"]["default"], 10);
}

#[test]
fn test_query_param_array_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Query parameter that accepts array of strings
    let mut operation = Operation {
        tags: None,
        summary: Some("Filter items".to_string()),
        description: None,
        operation_id: Some("filter_items".to_string()),
        parameters: Some(vec![Parameter {
            name: "tags".to_string(),
            location: ParameterLocation::Query,
            description: Some("Filter by tags".to_string()),
            required: Some(false),
            schema: Some(Schema::array(Schema::string())),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/items".to_string(), path_item);

    // Verify array schema
    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/items"]["get"]["parameters"][0];

    assert_eq!(param["name"], "tags");
    assert_eq!(param["schema"]["type"], "array");
    assert_eq!(param["schema"]["items"]["type"], "string");
}

#[test]
fn test_query_param_required_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Required query parameter
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("search".to_string()),
        parameters: Some(vec![Parameter {
            name: "q".to_string(),
            location: ParameterLocation::Query,
            description: Some("Search query".to_string()),
            required: Some(true), // Required query parameter
            schema: Some({
                let mut s = Schema::string();
                s.min_length = Some(1);
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/search".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/search"]["get"]["parameters"][0];

    assert_eq!(param["required"], true);
    assert_eq!(param["schema"]["minLength"], 1);
}

// ============================================================================
// Path Parameter OpenAPI Tests
// ============================================================================

#[test]
fn test_path_param_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Path parameters are always required
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("get_user".to_string()),
        parameters: Some(vec![Parameter {
            name: "user_id".to_string(),
            location: ParameterLocation::Path,
            description: Some("User ID".to_string()),
            required: Some(true), // Path params are always required
            schema: Some({
                let mut s = Schema::integer();
                s.format = Some("int64".to_string());
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/users/{user_id}".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/users/{user_id}"]["get"]["parameters"][0];

    assert_eq!(param["name"], "user_id");
    assert_eq!(param["in"], "path");
    assert_eq!(param["required"], true); // Path params always required
    assert_eq!(param["schema"]["type"], "integer");
}

#[test]
fn test_path_param_string_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // String path parameter with pattern validation
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("get_item".to_string()),
        parameters: Some(vec![Parameter {
            name: "item_id".to_string(),
            location: ParameterLocation::Path,
            description: Some("Item slug".to_string()),
            required: Some(true),
            schema: Some({
                let mut s = Schema::string();
                s.pattern = Some(r"^[a-z0-9-]+$".to_string());
                s.min_length = Some(3);
                s.max_length = Some(50);
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/items/{item_id}".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/items/{item_id}"]["get"]["parameters"][0];

    assert_eq!(param["schema"]["type"], "string");
    assert_eq!(param["schema"]["pattern"], r"^[a-z0-9-]+$");
    assert_eq!(param["schema"]["minLength"], 3);
    assert_eq!(param["schema"]["maxLength"], 50);
}

// ============================================================================
// Header Parameter OpenAPI Tests
// ============================================================================

#[test]
fn test_header_param_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Custom header parameter
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("secure_endpoint".to_string()),
        parameters: Some(vec![Parameter {
            name: "X-API-Key".to_string(),
            location: ParameterLocation::Header,
            description: Some("API authentication key".to_string()),
            required: Some(true),
            schema: Some({
                let mut s = Schema::string();
                s.min_length = Some(20);
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/secure".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/secure"]["get"]["parameters"][0];

    assert_eq!(param["name"], "X-API-Key");
    assert_eq!(param["in"], "header");
    assert_eq!(param["required"], true);
    assert_eq!(param["schema"]["minLength"], 20);
}

#[test]
fn test_header_param_optional_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Optional header for analytics
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("track".to_string()),
        parameters: Some(vec![Parameter {
            name: "X-Request-ID".to_string(),
            location: ParameterLocation::Header,
            description: Some("Optional request tracking ID".to_string()),
            required: Some(false),
            schema: Some({
                let mut s = Schema::string();
                s.format = Some("uuid".to_string());
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.post = Some(operation);
    schema.add_path("/track".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/track"]["post"]["parameters"][0];

    assert_eq!(param["required"], false);
    assert_eq!(param["schema"]["format"], "uuid");
}

// ============================================================================
// Cookie Parameter OpenAPI Tests
// ============================================================================

#[test]
fn test_cookie_param_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Cookie parameter for session
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("dashboard".to_string()),
        parameters: Some(vec![Parameter {
            name: "session_id".to_string(),
            location: ParameterLocation::Cookie,
            description: Some("Session identifier".to_string()),
            required: Some(true),
            schema: Some({
                let mut s = Schema::string();
                s.min_length = Some(32);
                s.max_length = Some(64);
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/dashboard".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/dashboard"]["get"]["parameters"][0];

    assert_eq!(param["name"], "session_id");
    assert_eq!(param["in"], "cookie");
    assert_eq!(param["required"], true);
}

// ============================================================================
// Multiple Parameters Combined Tests
// ============================================================================

#[test]
fn test_multiple_param_types_combined() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Endpoint with path, query, and header parameters
    let mut operation = Operation {
        tags: None,
        summary: Some("Get user details with pagination".to_string()),
        description: None,
        operation_id: Some("get_user_details".to_string()),
        parameters: Some(vec![
            Parameter {
                name: "user_id".to_string(),
                location: ParameterLocation::Path,
                description: Some("User ID".to_string()),
                required: Some(true),
                schema: Some(Schema::integer()),
            },
            Parameter {
                name: "page".to_string(),
                location: ParameterLocation::Query,
                description: Some("Page number".to_string()),
                required: Some(false),
                schema: Some({
                    let mut s = Schema::integer();
                    s.default = Some(Value::Number(1.into()));
                    s.minimum = Some(1.0);
                    s
                }),
            },
            Parameter {
                name: "X-Include-Details".to_string(),
                location: ParameterLocation::Header,
                description: Some("Include detailed information".to_string()),
                required: Some(false),
                schema: Some({
                    let mut s = Schema::boolean();
                    s.default = Some(Value::Bool(false));
                    s
                }),
            },
        ]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/users/{user_id}".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let parameters = json["paths"]["/users/{user_id}"]["get"]["parameters"]
        .as_array()
        .unwrap();

    assert_eq!(parameters.len(), 3);

    // Verify path parameter
    assert_eq!(parameters[0]["name"], "user_id");
    assert_eq!(parameters[0]["in"], "path");
    assert_eq!(parameters[0]["required"], true);

    // Verify query parameter
    assert_eq!(parameters[1]["name"], "page");
    assert_eq!(parameters[1]["in"], "query");
    assert_eq!(parameters[1]["required"], false);
    assert_eq!(parameters[1]["schema"]["minimum"], 1.0);

    // Verify header parameter
    assert_eq!(parameters[2]["name"], "X-Include-Details");
    assert_eq!(parameters[2]["in"], "header");
    assert_eq!(parameters[2]["schema"]["type"], "boolean");
}

#[test]
fn test_enum_query_param_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Query parameter with enum values
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("list_items".to_string()),
        parameters: Some(vec![Parameter {
            name: "sort".to_string(),
            location: ParameterLocation::Query,
            description: Some("Sort order".to_string()),
            required: Some(false),
            schema: Some({
                let mut s = Schema::string();
                s.enum_values = Some(vec![
                    Value::String("asc".to_string()),
                    Value::String("desc".to_string()),
                ]);
                s.default = Some(Value::String("asc".to_string()));
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(operation);
    schema.add_path("/items".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/items"]["get"]["parameters"][0];

    let enum_values = param["schema"]["enum"].as_array().unwrap();
    assert_eq!(enum_values.len(), 2);
    assert_eq!(enum_values[0], "asc");
    assert_eq!(enum_values[1], "desc");
    assert_eq!(param["schema"]["default"], "asc");
}

#[test]
fn test_param_with_example_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");
    let mut path_item = PathItem::default();

    // Parameter with example value
    let mut operation = Operation {
        tags: None,
        summary: None,
        description: None,
        operation_id: Some("create_user".to_string()),
        parameters: Some(vec![Parameter {
            name: "email".to_string(),
            location: ParameterLocation::Query,
            description: Some("User email address".to_string()),
            required: Some(true),
            schema: Some({
                let mut s = Schema::string();
                s.format = Some("email".to_string());
                s
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.post = Some(operation);
    schema.add_path("/users".to_string(), path_item);

    let json = serde_json::to_value(&schema).unwrap();
    let param = &json["paths"]["/users"]["post"]["parameters"][0];

    assert_eq!(param["schema"]["format"], "email");
}
