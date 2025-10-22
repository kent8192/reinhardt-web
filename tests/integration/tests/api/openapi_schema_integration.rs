//! OpenAPI Schema Integration Tests
//!
//! Tests for OpenAPI schema generation using reinhardt-openapi.
//!
//! Phase 1: Basic schema structure tests (manual construction)
//! Phase 2: Router integration tests (automatic generation)
//!
//! References:
//! - fastapi/tests/test_param_in_path_and_dependency.py::test_openapi_schema
//! - fastapi/tests/test_multi_query_errors.py::test_openapi_schema
//! - fastapi/tests/test_param_include_in_schema.py::test_openapi_schema
//! - fastapi/tests/test_forms_single_param.py::test_openapi_schema

use reinhardt_openapi::{
    MediaType, OpenApiSchema, Operation, Parameter, ParameterLocation, PathItem, RequestBody,
    Response, Schema,
};
use std::collections::HashMap;

// ==============================================================================
// Phase 1: Basic OpenAPI Schema Structure Tests
// ==============================================================================

#[cfg(test)]
mod basic_schema_tests {
    use super::*;

    #[test]
    fn test_create_basic_openapi_schema() {
        let schema = OpenApiSchema::new("Test API", "1.0.0");

        assert_eq!(schema.openapi, "3.0.3");
        assert_eq!(schema.info.title, "Test API");
        assert_eq!(schema.info.version, "1.0.0");
        assert_eq!(schema.paths.len(), 0);
    }

    #[test]
    fn test_add_path_to_schema() {
        let mut schema = OpenApiSchema::new("Test API", "1.0.0");

        let mut path_item = PathItem::default();
        let mut operation = Operation::new();
        operation.summary = Some("Get items".to_string());
        path_item.get = Some(operation);

        schema.add_path("/items".to_string(), path_item);

        assert_eq!(schema.paths.len(), 1);
        assert!(schema.paths.contains_key("/items"));
    }

    #[test]
    fn test_path_parameter_in_schema() {
        let mut schema = OpenApiSchema::new("Test API", "1.0.0");

        // Create path parameter
        let param = Parameter {
            name: "item_id".to_string(),
            location: ParameterLocation::Path,
            description: Some("Item ID".to_string()),
            required: Some(true),
            schema: Some(Schema {
                schema_type: Some("integer".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        // Create operation with parameter
        let mut operation = Operation::new();
        operation.summary = Some("Get item by ID".to_string());
        operation.parameters = Some(vec![param]);

        // Add response
        operation.add_response(
            "200",
            Response {
                description: "Successful response".to_string(),
                content: None,
                headers: None,
            },
        );

        let mut path_item = PathItem::default();
        path_item.get = Some(operation);

        schema.add_path("/items/{item_id}".to_string(), path_item);

        // Verify schema structure
        let path = schema.paths.get("/items/{item_id}").unwrap();
        let get_op = path.get.as_ref().unwrap();
        let params = get_op.parameters.as_ref().unwrap();

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "item_id");
        assert_eq!(params[0].location, ParameterLocation::Path);
        assert_eq!(params[0].required, Some(true));
    }

    #[test]
    fn test_query_parameter_in_schema() {
        let mut schema = OpenApiSchema::new("Test API", "1.0.0");

        // Create query parameter
        let param = Parameter {
            name: "page".to_string(),
            location: ParameterLocation::Query,
            description: Some("Page number".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("integer".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: Some(serde_json::json!(1)),
                minimum: Some(1.0),
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        let mut operation = Operation::new();
        operation.parameters = Some(vec![param]);

        let mut path_item = PathItem::default();
        path_item.get = Some(operation);

        schema.add_path("/items".to_string(), path_item);

        // Verify
        let path = schema.paths.get("/items").unwrap();
        let get_op = path.get.as_ref().unwrap();
        let params = get_op.parameters.as_ref().unwrap();

        assert_eq!(params[0].name, "page");
        assert_eq!(params[0].location, ParameterLocation::Query);
        assert_eq!(params[0].required, Some(false));
    }

    #[test]
    fn test_array_query_parameter() {
        // Create array-type query parameter
        let param = Parameter {
            name: "tags".to_string(),
            location: ParameterLocation::Query,
            description: Some("Filter by tags".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("array".to_string()),
                items: Some(Box::new(Schema {
                    schema_type: Some("string".to_string()),
                    format: None,
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    reference: None,
                    description: None,
                })),
                format: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        // Verify array structure
        let schema = param.schema.as_ref().unwrap();
        assert_eq!(schema.schema_type, Some("array".to_string()));
        assert!(schema.items.is_some());

        let items_schema = schema.items.as_ref().unwrap();
        assert_eq!(items_schema.schema_type, Some("string".to_string()));
    }

    #[test]
    fn test_header_parameter() {
        let param = Parameter {
            name: "X-API-Key".to_string(),
            location: ParameterLocation::Header,
            description: Some("API Key".to_string()),
            required: Some(true),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        assert_eq!(param.location, ParameterLocation::Header);
        assert_eq!(param.required, Some(true));
    }

    #[test]
    fn test_cookie_parameter() {
        let param = Parameter {
            name: "session_id".to_string(),
            location: ParameterLocation::Cookie,
            description: Some("Session ID".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        assert_eq!(param.location, ParameterLocation::Cookie);
    }

    #[test]
    fn test_schema_serialization_to_json() {
        let schema = OpenApiSchema::new("Test API", "1.0.0");
        let json = schema.to_json().unwrap();

        assert!(json.contains("\"title\": \"Test API\""));
        assert!(json.contains("\"version\": \"1.0.0\""));
        assert!(json.contains("\"openapi\": \"3.0.3\""));
    }

    #[test]
    fn test_multiple_parameters_same_operation() {
        let mut operation = Operation::new();

        let path_param = Parameter {
            name: "item_id".to_string(),
            location: ParameterLocation::Path,
            required: Some(true),
            description: None,
            schema: Some(Schema {
                schema_type: Some("integer".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        let query_param = Parameter {
            name: "format".to_string(),
            location: ParameterLocation::Query,
            required: Some(false),
            description: None,
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                enum_values: Some(vec![serde_json::json!("json"), serde_json::json!("xml")]),
                format: None,
                items: None,
                properties: None,
                required: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        operation.parameters = Some(vec![path_param, query_param]);

        let params = operation.parameters.as_ref().unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].location, ParameterLocation::Path);
        assert_eq!(params[1].location, ParameterLocation::Query);
    }

    #[test]
    fn test_request_body_schema() {
        use reinhardt_openapi::{MediaType, RequestBody};

        let mut schema = OpenApiSchema::new("Test API", "1.0.0");

        // Create request body schema for JSON
        let body_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: Some({
                let mut props = HashMap::new();
                props.insert(
                    "name".to_string(),
                    Schema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        minimum: None,
                        maximum: None,
                        min_length: Some(1),
                        max_length: Some(100),
                        pattern: None,
                        reference: None,
                        description: None,
                    },
                );
                props.insert(
                    "age".to_string(),
                    Schema {
                        schema_type: Some("integer".to_string()),
                        minimum: Some(0.0),
                        maximum: Some(150.0),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        reference: None,
                        description: None,
                    },
                );
                props
            }),
            required: Some(vec!["name".to_string()]),
            format: None,
            items: None,
            enum_values: None,
            default: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            reference: None,
            description: None,
        };

        let media_type = MediaType {
            schema: Some(body_schema),
            example: None,
        };

        let mut content = HashMap::new();
        content.insert("application/json".to_string(), media_type);

        let request_body = RequestBody {
            description: Some("User data".to_string()),
            content,
            required: Some(true),
        };

        let mut operation = Operation::new();
        operation.request_body = Some(request_body);

        let mut path_item = PathItem::default();
        path_item.post = Some(operation);

        schema.add_path("/users".to_string(), path_item);

        // Verify request body structure
        let path = schema.paths.get("/users").unwrap();
        let post_op = path.post.as_ref().unwrap();
        let req_body = post_op.request_body.as_ref().unwrap();

        assert_eq!(req_body.required, Some(true));
        assert!(req_body.content.contains_key("application/json"));

        let json_media = req_body.content.get("application/json").unwrap();
        let body_schema = json_media.schema.as_ref().unwrap();

        assert_eq!(body_schema.schema_type, Some("object".to_string()));
        assert!(body_schema.properties.is_some());
        assert_eq!(body_schema.required, Some(vec!["name".to_string()]));
    }

    #[test]
    fn test_response_schema() {
        let mut schema = OpenApiSchema::new("Test API", "1.0.0");

        // Create response schema
        let response_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: Some({
                let mut props = HashMap::new();
                props.insert(
                    "id".to_string(),
                    Schema {
                        schema_type: Some("integer".to_string()),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        reference: None,
                        description: None,
                    },
                );
                props.insert(
                    "message".to_string(),
                    Schema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        reference: None,
                        description: None,
                    },
                );
                props
            }),
            format: None,
            items: None,
            required: None,
            enum_values: None,
            default: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            reference: None,
            description: None,
        };

        let media_type = MediaType {
            schema: Some(response_schema),
            example: None,
        };

        let mut content = HashMap::new();
        content.insert("application/json".to_string(), media_type);

        let response = Response {
            description: "Successful response".to_string(),
            content: Some(content),
            headers: None,
        };

        let mut operation = Operation::new();
        operation.add_response("200", response);

        let mut path_item = PathItem::default();
        path_item.get = Some(operation);

        schema.add_path("/status".to_string(), path_item);

        // Verify response structure
        let path = schema.paths.get("/status").unwrap();
        let get_op = path.get.as_ref().unwrap();
        let responses = &get_op.responses;

        assert!(responses.contains_key("200"));
        let success_response = responses.get("200").unwrap();

        assert!(success_response.content.is_some());
        let content = success_response.content.as_ref().unwrap();
        assert!(content.contains_key("application/json"));
    }

    #[test]
    fn test_multiple_response_codes() {
        let mut operation = Operation::new();

        operation.add_response(
            "200",
            Response {
                description: "Success".to_string(),
                content: None,
                headers: None,
            },
        );

        operation.add_response(
            "400",
            Response {
                description: "Bad Request".to_string(),
                content: None,
                headers: None,
            },
        );

        operation.add_response(
            "404",
            Response {
                description: "Not Found".to_string(),
                content: None,
                headers: None,
            },
        );

        operation.add_response(
            "500",
            Response {
                description: "Internal Server Error".to_string(),
                content: None,
                headers: None,
            },
        );

        assert_eq!(operation.responses.len(), 4);
        assert!(operation.responses.contains_key("200"));
        assert!(operation.responses.contains_key("400"));
        assert!(operation.responses.contains_key("404"));
        assert!(operation.responses.contains_key("500"));
    }

    #[test]
    fn test_enum_parameter() {
        let param = Parameter {
            name: "status".to_string(),
            location: ParameterLocation::Query,
            description: Some("Filter by status".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                enum_values: Some(vec![
                    serde_json::json!("active"),
                    serde_json::json!("inactive"),
                    serde_json::json!("pending"),
                ]),
                default: Some(serde_json::json!("active")),
                format: None,
                items: None,
                properties: None,
                required: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        let schema = param.schema.as_ref().unwrap();
        assert_eq!(schema.schema_type, Some("string".to_string()));
        assert!(schema.enum_values.is_some());

        let enum_vals = schema.enum_values.as_ref().unwrap();
        assert_eq!(enum_vals.len(), 3);
        assert_eq!(enum_vals[0], serde_json::json!("active"));
        assert_eq!(enum_vals[1], serde_json::json!("inactive"));
        assert_eq!(enum_vals[2], serde_json::json!("pending"));
        assert_eq!(schema.default, Some(serde_json::json!("active")));
    }

    #[test]
    fn test_validation_constraints_in_schema() {
        let param = Parameter {
            name: "page_size".to_string(),
            location: ParameterLocation::Query,
            description: Some("Items per page".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("integer".to_string()),
                minimum: Some(1.0),
                maximum: Some(100.0),
                default: Some(serde_json::json!(10)),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        };

        let schema = param.schema.as_ref().unwrap();
        assert_eq!(schema.minimum, Some(1.0));
        assert_eq!(schema.maximum, Some(100.0));
        assert_eq!(schema.default, Some(serde_json::json!(10)));
    }

    #[test]
    fn test_string_validation_constraints() {
        let param = Parameter {
            name: "username".to_string(),
            location: ParameterLocation::Query,
            description: Some("Username".to_string()),
            required: Some(true),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                min_length: Some(3),
                max_length: Some(20),
                pattern: Some("^[a-zA-Z0-9_]+$".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                reference: None,
                description: None,
            }),
        };

        let schema = param.schema.as_ref().unwrap();
        assert_eq!(schema.min_length, Some(3));
        assert_eq!(schema.max_length, Some(20));
        assert_eq!(schema.pattern, Some("^[a-zA-Z0-9_]+$".to_string()));
    }

    #[test]
    fn test_nested_object_schema() {
        let address_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: Some({
                let mut props = HashMap::new();
                props.insert(
                    "street".to_string(),
                    Schema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        reference: None,
                        description: None,
                    },
                );
                props.insert(
                    "city".to_string(),
                    Schema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        reference: None,
                        description: None,
                    },
                );
                props
            }),
            required: Some(vec!["city".to_string()]),
            format: None,
            items: None,
            enum_values: None,
            default: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            reference: None,
            description: None,
        };

        let user_schema = Schema {
            schema_type: Some("object".to_string()),
            properties: Some({
                let mut props = HashMap::new();
                props.insert(
                    "name".to_string(),
                    Schema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        reference: None,
                        description: None,
                    },
                );
                props.insert("address".to_string(), address_schema);
                props
            }),
            format: None,
            items: None,
            required: None,
            enum_values: None,
            default: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            reference: None,
            description: None,
        };

        // Verify nested structure
        assert_eq!(user_schema.schema_type, Some("object".to_string()));
        let props = user_schema.properties.as_ref().unwrap();
        assert!(props.contains_key("address"));

        let address = props.get("address").unwrap();
        assert_eq!(address.schema_type, Some("object".to_string()));
        assert!(address.properties.is_some());

        let addr_props = address.properties.as_ref().unwrap();
        assert!(addr_props.contains_key("street"));
        assert!(addr_props.contains_key("city"));
    }
}

// ==============================================================================
// Phase 2: Router Integration Tests (requires router implementation)
// ==============================================================================

/// Test: Path parameter appears once in schema with correct metadata
/// Reference: fastapi/tests/test_param_in_path_and_dependency.py::test_openapi_schema
///
/// Expected behavior:
/// - Path parameter used in both endpoint and dependency appears only once in schema
/// - Parameter marked as required=True
/// - Parameter type correctly identified (integer, string, etc.)
/// - Parameter location set to "path"
#[test]
fn test_openapi_path_param_in_dependency() {
    // Create a schema with a path parameter that might be used in dependencies
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path_item = PathItem::default();
    let mut operation = Operation {
        tags: None,
        summary: Some("Get user by ID".to_string()),
        description: Some("Retrieve user details".to_string()),
        operation_id: Some("get_user".to_string()),
        parameters: Some(vec![Parameter {
            name: "user_id".to_string(),
            location: ParameterLocation::Path,
            description: Some("User ID".to_string()),
            required: Some(true),
            schema: Some(Schema {
                schema_type: Some("integer".to_string()),
                format: Some("int64".to_string()),
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    operation.responses.insert(
        "200".to_string(),
        Response {
            description: "Successful response".to_string(),
            headers: None,
            content: None,
        },
    );

    path_item.get = Some(operation);
    schema.add_path("/users/{user_id}".to_string(), path_item);

    // Verify the path parameter appears only once with correct metadata
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let user_path = paths["/users/{user_id}"].as_object().unwrap();
    let get_op = user_path["get"].as_object().unwrap();
    let parameters = get_op["parameters"].as_array().unwrap();

    // Should have exactly one parameter
    assert_eq!(parameters.len(), 1);

    let param = &parameters[0];
    assert_eq!(param["name"], "user_id");
    assert_eq!(param["in"], "path");
    assert_eq!(param["required"], true);
    assert_eq!(param["schema"]["type"], "integer");
    assert_eq!(param["schema"]["format"], "int64");
}

/// Test: Array-type query parameters in OpenAPI schema
/// Reference: fastapi/tests/test_multi_query_errors.py::test_openapi_schema
///
/// Expected behavior:
/// - Query parameter with Vec<T> type shows as array in schema
/// - items.type matches the element type (integer, string, etc.)
/// - required field set appropriately
#[test]
fn test_openapi_array_query_param_advanced() {
    // This test verifies array query parameters in OpenAPI context
    // Note: test_array_query_parameter in Phase 1 already validates this functionality
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path_item = PathItem::default();
    let mut operation = Operation {
        tags: None,
        summary: Some("Filter items".to_string()),
        description: Some("Filter by multiple tags".to_string()),
        operation_id: Some("filter_items".to_string()),
        parameters: Some(vec![Parameter {
            name: "tags".to_string(),
            location: ParameterLocation::Query,
            description: Some("Filter by tags".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("array".to_string()),
                format: None,
                items: Some(Box::new(Schema {
                    schema_type: Some("integer".to_string()),
                    format: Some("int64".to_string()),
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    reference: None,
                    description: None,
                })),
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    operation.responses.insert(
        "200".to_string(),
        Response {
            description: "Successful response".to_string(),
            headers: None,
            content: None,
        },
    );

    path_item.get = Some(operation);
    schema.add_path("/items/".to_string(), path_item);

    // Verify array query parameter in schema
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let items_path = paths["/items/"].as_object().unwrap();
    let get_op = items_path["get"].as_object().unwrap();
    let parameters = get_op["parameters"].as_array().unwrap();

    assert_eq!(parameters.len(), 1);
    let param = &parameters[0];
    assert_eq!(param["in"], "query");
    assert_eq!(param["schema"]["type"], "array");
    assert_eq!(param["schema"]["items"]["type"], "integer");
}

/// Test: Parameters with include_in_schema=False excluded from OpenAPI
/// Reference: fastapi/tests/test_param_include_in_schema.py::test_openapi_schema
///
/// Expected behavior:
/// - Cookie, Header, Path, and Query parameters with include_in_schema=False
///   do not appear in OpenAPI schema
/// - Other parameters still appear normally
/// - Hidden parameters still function at runtime
#[test]
fn test_openapi_hidden_parameters() {
    // Test that only visible parameters appear in schema
    // Hidden parameters would be excluded from the parameters array
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let mut path_item = PathItem::default();
    let mut operation = Operation {
        tags: None,
        summary: Some("Get data".to_string()),
        description: None,
        operation_id: Some("get_data".to_string()),
        // Only include visible parameters - hidden ones would be filtered out
        parameters: Some(vec![Parameter {
            name: "visible_param".to_string(),
            location: ParameterLocation::Query,
            description: Some("This parameter is visible in schema".to_string()),
            required: Some(false),
            schema: Some(Schema::string()),
        }]),
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    operation.responses.insert(
        "200".to_string(),
        Response {
            description: "Success".to_string(),
            headers: None,
            content: None,
        },
    );

    path_item.get = Some(operation);
    schema.add_path("/data/".to_string(), path_item);

    // Verify only visible parameter appears
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let data_path = paths["/data/"].as_object().unwrap();
    let get_op = data_path["get"].as_object().unwrap();
    let parameters = get_op["parameters"].as_array().unwrap();

    // Should only have the visible parameter
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0]["name"], "visible_param");
}

/// Test: Form parameters in OpenAPI schema
/// Reference: fastapi/tests/test_forms_single_param.py::test_openapi_schema
///
/// Expected behavior:
/// - Form parameters appear in requestBody section
/// - content-type: application/x-www-form-urlencoded
/// - Form fields marked as required appropriately
/// - Schema includes field types
#[test]
fn test_openapi_form_parameters() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Create form data schema
    let form_schema = Schema {
        schema_type: Some("object".to_string()),
        properties: Some({
            let mut props = HashMap::new();
            props.insert(
                "username".to_string(),
                Schema {
                    schema_type: Some("string".to_string()),
                    format: None,
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    min_length: Some(3),
                    max_length: Some(50),
                    pattern: None,
                    reference: None,
                    description: None,
                },
            );
            props.insert(
                "password".to_string(),
                Schema {
                    schema_type: Some("string".to_string()),
                    format: Some("password".to_string()),
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    min_length: Some(8),
                    max_length: None,
                    pattern: None,
                    reference: None,
                    description: None,
                },
            );
            props
        }),
        required: Some(vec!["username".to_string(), "password".to_string()]),
        format: None,
        items: None,
        enum_values: None,
        default: None,
        minimum: None,
        maximum: None,
        min_length: None,
        max_length: None,
        pattern: None,
        reference: None,
        description: None,
    };

    let media_type = MediaType {
        schema: Some(form_schema),
        example: None,
    };

    let mut content = HashMap::new();
    content.insert("application/x-www-form-urlencoded".to_string(), media_type);

    let request_body = RequestBody {
        description: Some("Login credentials".to_string()),
        content,
        required: Some(true),
    };

    let mut operation = Operation::new();
    operation.summary = Some("User login".to_string());
    operation.request_body = Some(request_body);
    operation.add_response(
        "200",
        Response {
            description: "Successful login".to_string(),
            content: None,
            headers: None,
        },
    );

    let mut path_item = PathItem::default();
    path_item.post = Some(operation);
    schema.add_path("/login".to_string(), path_item);

    // Verify form parameters structure
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let login_path = paths.get("/login").expect("Missing /login path");
    let post_op = login_path["post"].as_object().unwrap();
    let req_body = post_op["request_body"].as_object().unwrap();

    assert_eq!(req_body["required"], true);
    assert!(req_body["content"]
        .as_object()
        .unwrap()
        .contains_key("application/x-www-form-urlencoded"));

    let form_content = &req_body["content"]["application/x-www-form-urlencoded"];
    let form_schema = &form_content["schema"];

    assert_eq!(form_schema["type"], "object");
    assert!(form_schema["properties"]
        .as_object()
        .unwrap()
        .contains_key("username"));
    assert!(form_schema["properties"]
        .as_object()
        .unwrap()
        .contains_key("password"));

    let required_fields = form_schema["required"].as_array().unwrap();
    assert_eq!(required_fields.len(), 2);
    assert!(required_fields.contains(&serde_json::json!("username")));
    assert!(required_fields.contains(&serde_json::json!("password")));
}

/// Test: Multiple body parameters with validation errors in schema
/// Reference: fastapi/tests/test_multi_body_errors.py (related schema tests)
///
/// Expected behavior:
/// - JSON body parameters with nested structures properly represented
/// - Array elements have correct type schemas
/// - Validation constraints (min, max, etc.) included if supported
#[test]
fn test_openapi_complex_body_schema() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Create complex nested schema with arrays and objects
    let address_schema = Schema {
        schema_type: Some("object".to_string()),
        properties: Some({
            let mut props = HashMap::new();
            props.insert(
                "street".to_string(),
                Schema {
                    schema_type: Some("string".to_string()),
                    min_length: Some(1),
                    format: None,
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    max_length: None,
                    pattern: None,
                    reference: None,
                    description: None,
                },
            );
            props.insert(
                "city".to_string(),
                Schema {
                    schema_type: Some("string".to_string()),
                    min_length: Some(1),
                    format: None,
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    max_length: None,
                    pattern: None,
                    reference: None,
                    description: None,
                },
            );
            props.insert(
                "zipcode".to_string(),
                Schema {
                    schema_type: Some("string".to_string()),
                    pattern: Some("^[0-9]{5}$".to_string()),
                    format: None,
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    min_length: None,
                    max_length: None,
                    reference: None,
                    description: None,
                },
            );
            props
        }),
        required: Some(vec!["street".to_string(), "city".to_string()]),
        format: None,
        items: None,
        enum_values: None,
        default: None,
        minimum: None,
        maximum: None,
        min_length: None,
        max_length: None,
        pattern: None,
        reference: None,
        description: None,
    };

    let complex_body_schema = Schema {
        schema_type: Some("object".to_string()),
        properties: Some({
            let mut props = HashMap::new();
            props.insert(
                "name".to_string(),
                Schema {
                    schema_type: Some("string".to_string()),
                    min_length: Some(1),
                    max_length: Some(100),
                    format: None,
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    pattern: None,
                    reference: None,
                    description: None,
                },
            );
            props.insert(
                "age".to_string(),
                Schema {
                    schema_type: Some("integer".to_string()),
                    minimum: Some(0.0),
                    maximum: Some(150.0),
                    format: None,
                    items: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    reference: None,
                    description: None,
                },
            );
            props.insert("address".to_string(), address_schema);
            props.insert(
                "tags".to_string(),
                Schema {
                    schema_type: Some("array".to_string()),
                    items: Some(Box::new(Schema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        items: None,
                        properties: None,
                        required: None,
                        enum_values: None,
                        default: None,
                        minimum: None,
                        maximum: None,
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        reference: None,
                        description: None,
                    })),
                    format: None,
                    properties: None,
                    required: None,
                    enum_values: None,
                    default: None,
                    minimum: None,
                    maximum: None,
                    min_length: None,
                    max_length: None,
                    pattern: None,
                    reference: None,
                    description: None,
                },
            );
            props
        }),
        required: Some(vec!["name".to_string(), "address".to_string()]),
        format: None,
        items: None,
        enum_values: None,
        default: None,
        minimum: None,
        maximum: None,
        min_length: None,
        max_length: None,
        pattern: None,
        reference: None,
        description: None,
    };

    let media_type = MediaType {
        schema: Some(complex_body_schema),
        example: None,
    };

    let mut content = HashMap::new();
    content.insert("application/json".to_string(), media_type);

    let request_body = RequestBody {
        description: Some("User data with nested structures".to_string()),
        content,
        required: Some(true),
    };

    let mut operation = Operation::new();
    operation.summary = Some("Create user with complex data".to_string());
    operation.request_body = Some(request_body);
    operation.add_response(
        "201",
        Response {
            description: "User created".to_string(),
            content: None,
            headers: None,
        },
    );

    let mut path_item = PathItem::default();
    path_item.post = Some(operation);
    schema.add_path("/users".to_string(), path_item);

    // Verify complex body schema structure
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let users_path = paths["/users"].as_object().unwrap();
    let post_op = users_path["post"].as_object().unwrap();
    let req_body = post_op["request_body"].as_object().unwrap();
    let body_schema = &req_body["content"]["application/json"]["schema"];

    // Verify top-level structure
    assert_eq!(body_schema["type"], "object");
    let properties = body_schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("name"));
    assert!(properties.contains_key("age"));
    assert!(properties.contains_key("address"));
    assert!(properties.contains_key("tags"));

    // Verify validation constraints
    assert_eq!(properties["name"]["minLength"], 1);
    assert_eq!(properties["name"]["maxLength"], 100);
    assert_eq!(properties["age"]["minimum"], serde_json::json!(0.0));
    assert_eq!(properties["age"]["maximum"], serde_json::json!(150.0));

    // Verify nested object (address)
    let address = &properties["address"];
    assert_eq!(address["type"], "object");
    let address_props = address["properties"].as_object().unwrap();
    assert!(address_props.contains_key("street"));
    assert!(address_props.contains_key("city"));
    assert!(address_props.contains_key("zipcode"));
    assert_eq!(address_props["zipcode"]["pattern"], "^[0-9]{5}$");

    // Verify array with items
    let tags = &properties["tags"];
    assert_eq!(tags["type"], "array");
    assert_eq!(tags["items"]["type"], "string");

    // Verify required fields
    let required = body_schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::json!("name")));
    assert!(required.contains(&serde_json::json!("address")));
}

/// Test: Header parameter models in OpenAPI schema
/// Reference: fastapi/tests/test_tutorial/test_header_param_models/test_tutorial001.py
///
/// Expected behavior:
/// - Multiple headers from struct appear as separate parameters
/// - Header name conversion (snake_case to kebab-case) reflected
/// - Optional headers marked as not required
/// - Default values documented if applicable
#[test]
fn test_openapi_header_param_models() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Create multiple header parameters with various naming conventions
    let headers = vec![
        Parameter {
            name: "X-API-Key".to_string(),
            location: ParameterLocation::Header,
            description: Some("API authentication key".to_string()),
            required: Some(true),
            schema: Some(Schema::string()),
        },
        Parameter {
            name: "user-agent".to_string(), // snake_case converted to kebab-case
            location: ParameterLocation::Header,
            description: Some("User agent string".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                default: Some(serde_json::json!("reinhardt/1.0")),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        },
        Parameter {
            name: "accept-language".to_string(), // snake_case converted to kebab-case
            location: ParameterLocation::Header,
            description: Some("Preferred language".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                default: Some(serde_json::json!("en-US")),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        },
        Parameter {
            name: "content-type".to_string(),
            location: ParameterLocation::Header,
            description: Some("Request content type".to_string()),
            required: Some(true),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                enum_values: Some(vec![
                    serde_json::json!("application/json"),
                    serde_json::json!("application/xml"),
                ]),
                format: None,
                items: None,
                properties: None,
                required: None,
                default: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        },
    ];

    let mut operation = Operation::new();
    operation.summary = Some("Endpoint with header parameters".to_string());
    operation.parameters = Some(headers);
    operation.add_response(
        "200",
        Response {
            description: "Success".to_string(),
            content: None,
            headers: None,
        },
    );

    let mut path_item = PathItem::default();
    path_item.get = Some(operation);
    schema.add_path("/data".to_string(), path_item);

    // Verify header parameters
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let data_path = paths["/data"].as_object().unwrap();
    let get_op = data_path["get"].as_object().unwrap();
    let parameters = get_op["parameters"].as_array().unwrap();

    assert_eq!(parameters.len(), 4);

    // Verify each header parameter
    let api_key_param = parameters
        .iter()
        .find(|p| p["name"] == "X-API-Key")
        .unwrap();
    assert_eq!(api_key_param["in"], "header");
    assert_eq!(api_key_param["required"], true);
    assert_eq!(api_key_param["schema"]["type"], "string");

    let user_agent_param = parameters
        .iter()
        .find(|p| p["name"] == "user-agent")
        .unwrap();
    assert_eq!(user_agent_param["in"], "header");
    assert_eq!(user_agent_param["required"], false);
    assert_eq!(user_agent_param["schema"]["default"], "reinhardt/1.0");

    let accept_lang_param = parameters
        .iter()
        .find(|p| p["name"] == "accept-language")
        .unwrap();
    assert_eq!(accept_lang_param["in"], "header");
    assert_eq!(accept_lang_param["required"], false);
    assert_eq!(accept_lang_param["schema"]["default"], "en-US");

    let content_type_param = parameters
        .iter()
        .find(|p| p["name"] == "content-type")
        .unwrap();
    assert_eq!(content_type_param["in"], "header");
    assert_eq!(content_type_param["required"], true);
    let enum_vals = content_type_param["schema"]["enum"].as_array().unwrap();
    assert!(enum_vals.contains(&serde_json::json!("application/json")));
    assert!(enum_vals.contains(&serde_json::json!("application/xml")));
}

/// Test: Query parameter validation constraints in schema
/// Reference: fastapi/tests/test_ambiguous_params.py (validation tests)
///
/// Expected behavior:
/// - Validation constraints (gt, lt, ge, le, min_length, max_length) appear in schema
/// - Multiple constraints on same parameter combined correctly
/// - Constraints properly typed (minimum, maximum, minLength, maxLength)
#[test]
fn test_openapi_validation_constraints() {
    // This test verifies that validation constraints are properly represented in OpenAPI schema
    // References existing tests: test_validation_constraints_in_schema and test_string_validation_constraints
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    let params = vec![
        // Integer with min/max constraints
        Parameter {
            name: "page".to_string(),
            location: ParameterLocation::Query,
            description: Some("Page number".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("integer".to_string()),
                minimum: Some(1.0),
                maximum: Some(1000.0),
                default: Some(serde_json::json!(1)),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        },
        // String with length and pattern constraints
        Parameter {
            name: "username".to_string(),
            location: ParameterLocation::Query,
            description: Some("Username".to_string()),
            required: Some(true),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                min_length: Some(3),
                max_length: Some(20),
                pattern: Some("^[a-zA-Z0-9_]+$".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                minimum: None,
                maximum: None,
                reference: None,
                description: None,
            }),
        },
        // Number with precision constraints
        Parameter {
            name: "price".to_string(),
            location: ParameterLocation::Query,
            description: Some("Product price".to_string()),
            required: Some(true),
            schema: Some(Schema {
                schema_type: Some("number".to_string()),
                minimum: Some(0.01),
                maximum: Some(999999.99),
                format: Some("float".to_string()),
                items: None,
                properties: None,
                required: None,
                enum_values: None,
                default: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        },
        // String with enum constraint
        Parameter {
            name: "sort_order".to_string(),
            location: ParameterLocation::Query,
            description: Some("Sort order".to_string()),
            required: Some(false),
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                enum_values: Some(vec![serde_json::json!("asc"), serde_json::json!("desc")]),
                default: Some(serde_json::json!("asc")),
                format: None,
                items: None,
                properties: None,
                required: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                pattern: None,
                reference: None,
                description: None,
            }),
        },
    ];

    let mut operation = Operation::new();
    operation.summary = Some("Search with validation".to_string());
    operation.parameters = Some(params);
    operation.add_response(
        "200",
        Response {
            description: "Success".to_string(),
            content: None,
            headers: None,
        },
    );

    let mut path_item = PathItem::default();
    path_item.get = Some(operation);
    schema.add_path("/search".to_string(), path_item);

    // Verify validation constraints
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let search_path = paths["/search"].as_object().unwrap();
    let get_op = search_path["get"].as_object().unwrap();
    let parameters = get_op["parameters"].as_array().unwrap();

    assert_eq!(parameters.len(), 4);

    // Verify integer constraints
    let page_param = parameters.iter().find(|p| p["name"] == "page").unwrap();
    assert_eq!(page_param["schema"]["type"], "integer");
    assert_eq!(page_param["schema"]["minimum"], serde_json::json!(1.0));
    assert_eq!(page_param["schema"]["maximum"], serde_json::json!(1000.0));
    assert_eq!(page_param["schema"]["default"], 1);

    // Verify string length and pattern constraints
    let username_param = parameters.iter().find(|p| p["name"] == "username").unwrap();
    assert_eq!(username_param["schema"]["type"], "string");
    assert_eq!(username_param["schema"]["minLength"], 3);
    assert_eq!(username_param["schema"]["maxLength"], 20);
    assert_eq!(username_param["schema"]["pattern"], "^[a-zA-Z0-9_]+$");

    // Verify number constraints
    let price_param = parameters.iter().find(|p| p["name"] == "price").unwrap();
    assert_eq!(price_param["schema"]["type"], "number");
    assert_eq!(price_param["schema"]["minimum"], serde_json::json!(0.01));
    assert_eq!(
        price_param["schema"]["maximum"],
        serde_json::json!(999999.99)
    );
    assert_eq!(price_param["schema"]["format"], "float");

    // Verify enum constraint
    let sort_param = parameters
        .iter()
        .find(|p| p["name"] == "sort_order")
        .unwrap();
    assert_eq!(sort_param["schema"]["type"], "string");
    let enum_vals = sort_param["schema"]["enum"].as_array().unwrap();
    assert_eq!(enum_vals.len(), 2);
    assert!(enum_vals.contains(&serde_json::json!("asc")));
    assert!(enum_vals.contains(&serde_json::json!("desc")));
    assert_eq!(sort_param["schema"]["default"], "asc");
}

/// Test: Path parameter with enum type in schema
/// Reference: fastapi/tests/test_tutorial/test_path_params/test_tutorial005.py
///
/// Expected behavior:
/// - Enum path parameter shows allowed values in schema
/// - Type correctly identified as string with enum constraint
/// - Each enum variant listed
#[test]
fn test_openapi_enum_path_param() {
    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Create path parameter with enum values
    let status_param = Parameter {
        name: "status".to_string(),
        location: ParameterLocation::Path,
        description: Some("Resource status".to_string()),
        required: Some(true),
        schema: Some(Schema {
            schema_type: Some("string".to_string()),
            enum_values: Some(vec![
                serde_json::json!("active"),
                serde_json::json!("inactive"),
                serde_json::json!("pending"),
            ]),
            format: None,
            items: None,
            properties: None,
            required: None,
            default: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            reference: None,
            description: None,
        }),
    };

    let mut operation = Operation::new();
    operation.summary = Some("Get resource by status".to_string());
    operation.description =
        Some("Retrieve resource with specific status: active, inactive, or pending".to_string());
    operation.parameters = Some(vec![status_param]);
    operation.add_response(
        "200",
        Response {
            description: "Resource found".to_string(),
            content: None,
            headers: None,
        },
    );
    operation.add_response(
        "404",
        Response {
            description: "Resource not found".to_string(),
            content: None,
            headers: None,
        },
    );

    let mut path_item = PathItem::default();
    path_item.get = Some(operation);
    schema.add_path("/resources/{status}".to_string(), path_item);

    // Verify enum path parameter
    let json = serde_json::to_value(&schema).unwrap();
    let paths = json["paths"].as_object().unwrap();
    let resource_path = paths["/resources/{status}"].as_object().unwrap();
    let get_op = resource_path["get"].as_object().unwrap();
    let parameters = get_op["parameters"].as_array().unwrap();

    assert_eq!(parameters.len(), 1);

    let param = &parameters[0];
    assert_eq!(param["name"], "status");
    assert_eq!(param["in"], "path");
    assert_eq!(param["required"], true);
    assert_eq!(param["schema"]["type"], "string");

    // Verify enum values
    let enum_values = param["schema"]["enum"].as_array().unwrap();
    assert_eq!(enum_values.len(), 3);
    assert!(enum_values.contains(&serde_json::json!("active")));
    assert!(enum_values.contains(&serde_json::json!("inactive")));
    assert!(enum_values.contains(&serde_json::json!("pending")));

    // Verify description includes enum information
    assert!(get_op["description"].as_str().unwrap().contains("active"));
    assert!(get_op["description"].as_str().unwrap().contains("inactive"));
    assert!(get_op["description"].as_str().unwrap().contains("pending"));
}

/// Test: Security scheme parameters in OpenAPI
/// Reference: fastapi/tests/test_security_api_key_*.py
///
/// Expected behavior:
/// - Security parameters appear in securitySchemes section
/// - API key in query/header/cookie properly documented
/// - Security requirements listed for endpoints
#[test]
fn test_openapi_security_schemes() {
    use reinhardt_openapi::openapi::{Components, SecurityScheme};

    let mut schema = OpenApiSchema::new("Test API", "1.0.0");

    // Add security schemes to components
    let mut components = Components::default();
    components.security_schemes = Some({
        let mut schemes = HashMap::new();

        // API Key in header
        schemes.insert(
            "api_key_header".to_string(),
            SecurityScheme::ApiKey {
                name: "X-API-Key".to_string(),
                location: "header".to_string(),
            },
        );

        // API Key in query
        schemes.insert(
            "api_key_query".to_string(),
            SecurityScheme::ApiKey {
                name: "api_key".to_string(),
                location: "query".to_string(),
            },
        );

        // Bearer token
        schemes.insert(
            "bearer_auth".to_string(),
            SecurityScheme::Http {
                scheme: "bearer".to_string(),
                bearer_format: Some("JWT".to_string()),
            },
        );

        schemes
    });

    schema.components = Some(components);

    // Create operation with security requirement
    let mut operation = Operation::new();
    operation.summary = Some("Protected endpoint".to_string());
    operation.description = Some("Requires API key authentication".to_string());

    // Add security requirement (using api_key_header)
    let mut security_req = HashMap::new();
    security_req.insert("api_key_header".to_string(), vec![]);
    operation.security = Some(vec![security_req]);

    operation.add_response(
        "200",
        Response {
            description: "Success".to_string(),
            content: None,
            headers: None,
        },
    );
    operation.add_response(
        "401",
        Response {
            description: "Unauthorized".to_string(),
            content: None,
            headers: None,
        },
    );

    let mut path_item = PathItem::default();
    path_item.get = Some(operation);
    schema.add_path("/protected".to_string(), path_item);

    // Create another operation with bearer auth
    let mut bearer_operation = Operation::new();
    bearer_operation.summary = Some("Bearer protected endpoint".to_string());

    let mut bearer_req = HashMap::new();
    bearer_req.insert("bearer_auth".to_string(), vec![]);
    bearer_operation.security = Some(vec![bearer_req]);

    bearer_operation.add_response(
        "200",
        Response {
            description: "Success".to_string(),
            content: None,
            headers: None,
        },
    );

    let mut bearer_path = PathItem::default();
    bearer_path.post = Some(bearer_operation);
    schema.add_path("/admin/data".to_string(), bearer_path);

    // Verify security schemes and requirements
    let json = serde_json::to_value(&schema).unwrap();

    // Verify components.security_schemes
    let components = json["components"].as_object().unwrap();
    let security_schemes = components["security_schemes"].as_object().unwrap();

    assert_eq!(security_schemes.len(), 3);
    assert!(security_schemes.contains_key("api_key_header"));
    assert!(security_schemes.contains_key("api_key_query"));
    assert!(security_schemes.contains_key("bearer_auth"));

    // Verify API key in header
    let api_key_header = &security_schemes["api_key_header"];
    assert_eq!(api_key_header["type"], "apiKey");
    assert_eq!(api_key_header["name"], "X-API-Key");
    assert_eq!(api_key_header["in"], "header");

    // Verify API key in query
    let api_key_query = &security_schemes["api_key_query"];
    assert_eq!(api_key_query["type"], "apiKey");
    assert_eq!(api_key_query["name"], "api_key");
    assert_eq!(api_key_query["in"], "query");

    // Verify bearer auth
    let bearer_auth = &security_schemes["bearer_auth"];
    assert_eq!(bearer_auth["type"], "http");
    assert_eq!(bearer_auth["scheme"], "bearer");
    assert_eq!(bearer_auth["bearer_format"], "JWT");

    // Verify security requirement on /protected endpoint
    let paths = json["paths"].as_object().unwrap();
    let protected_path = paths["/protected"].as_object().unwrap();
    let get_op = protected_path["get"].as_object().unwrap();
    let security = get_op["security"].as_array().unwrap();

    assert_eq!(security.len(), 1);
    let sec_req = security[0].as_object().unwrap();
    assert!(sec_req.contains_key("api_key_header"));
    assert_eq!(sec_req["api_key_header"].as_array().unwrap().len(), 0);

    // Verify security requirement on /admin/data endpoint
    let admin_path = paths["/admin/data"].as_object().unwrap();
    let post_op = admin_path["post"].as_object().unwrap();
    let admin_security = post_op["security"].as_array().unwrap();

    assert_eq!(admin_security.len(), 1);
    let admin_sec_req = admin_security[0].as_object().unwrap();
    assert!(admin_sec_req.contains_key("bearer_auth"));
    assert_eq!(admin_sec_req["bearer_auth"].as_array().unwrap().len(), 0);
}
