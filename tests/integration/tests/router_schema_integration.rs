// Router and OpenAPI Schema integration tests
// Inspired by FastAPI's OpenAPI generation and Django REST Framework's schema generation

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_apps::{Handler, Request, Response, Result};
use reinhardt_openapi::{OpenApiSchema, Operation, PathItem, SchemaGenerator};
use reinhardt_routers::{DefaultRouter, Route, Router};
use reinhardt_viewsets::{ModelViewSet, ViewSet};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TestModel {
    id: i64,
    name: String,
}

#[derive(Debug, Clone)]
struct TestSerializer;

// Mock handler for testing
#[derive(Clone)]
struct MockHandler {
    response_text: String,
}

impl MockHandler {
    fn new(response_text: impl Into<String>) -> Self {
        Self {
            response_text: response_text.into(),
        }
    }
}

#[async_trait]
impl Handler for MockHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_body(Bytes::from(self.response_text.clone())))
    }
}

// Helper: Create RouteInspector for schema generation
struct RouteInspector {
    router: DefaultRouter,
}

impl RouteInspector {
    fn new(router: DefaultRouter) -> Self {
        Self { router }
    }

    fn generate_schema(&self, title: &str, version: &str) -> OpenApiSchema {
        let mut schema = OpenApiSchema::new(title, version);

        // Inspect all routes and generate path items
        for route in self.router.get_routes() {
            let path = route.path.clone();
            let mut path_item = PathItem::default();

            // Create a basic GET operation for simplicity
            let mut operation = Operation {
                tags: None,
                summary: Some(format!("Operation for {}", path)),
                description: route.name.clone(),
                operation_id: route.name.clone(),
                parameters: None,
                request_body: None,
                responses: HashMap::new(),
                security: None,
            };

            // Add 200 response
            operation.responses.insert(
                "200".to_string(),
                reinhardt_openapi::Response {
                    description: "Successful response".to_string(),
                    headers: None,
                    content: None,
                },
            );

            path_item.get = Some(operation);
            schema.add_path(path, path_item);
        }

        schema
    }
}

// Test 1: Basic OpenAPI schema generation from routes
#[tokio::test]
async fn test_basic_openapi_schema_generation() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(MockHandler::new("list"));

    router.add_route(Route::new("/users/", handler.clone()).with_name("users-list"));
    router.add_route(Route::new("/users/{id}/", handler.clone()).with_name("users-detail"));

    let inspector = RouteInspector::new(router);
    let schema = inspector.generate_schema("Test API", "1.0.0");

    // Verify schema structure
    assert_eq!(schema.info.title, "Test API");
    assert_eq!(schema.info.version, "1.0.0");
    assert_eq!(schema.openapi, "3.0.3");

    // Verify paths
    assert!(schema.paths.contains_key("/users/"));
    assert!(schema.paths.contains_key("/users/{id}/"));

    // Verify path items have operations
    let users_path = schema.paths.get("/users/").unwrap();
    assert!(users_path.get.is_some());

    let detail_path = schema.paths.get("/users/{id}/").unwrap();
    assert!(detail_path.get.is_some());
}

// Test 2: OpenAPI schema with route metadata (names, descriptions)
#[tokio::test]
async fn test_openapi_schema_with_route_metadata() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(MockHandler::new("response"));

    router.add_route(Route::new("/items/", handler.clone()).with_name("items-list"));
    router.add_route(Route::new("/items/{id}/", handler.clone()).with_name("items-detail"));

    let inspector = RouteInspector::new(router);
    let schema = inspector.generate_schema("Items API", "2.0.0");

    // Verify operations have proper metadata
    let list_path = schema.paths.get("/items/").unwrap();
    let list_op = list_path.get.as_ref().unwrap();
    assert_eq!(list_op.operation_id, Some("items-list".to_string()));
    assert!(list_op.summary.is_some());

    let detail_path = schema.paths.get("/items/{id}/").unwrap();
    let detail_op = detail_path.get.as_ref().unwrap();
    assert_eq!(detail_op.operation_id, Some("items-detail".to_string()));
}

// Test 3: OpenAPI schema from ViewSet routes
#[tokio::test]
async fn test_openapi_schema_from_viewset() {
    let mut router = DefaultRouter::new();
    let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
        Arc::new(ModelViewSet::new("users"));

    router.register_viewset("users", viewset);

    let inspector = RouteInspector::new(router);
    let schema = inspector.generate_schema("ViewSet API", "1.0.0");

    // Verify ViewSet generates list and detail routes
    assert!(schema.paths.contains_key("/users/"));
    assert!(schema.paths.contains_key("/users/{id}/"));

    // Verify both routes have operations
    assert!(schema.paths.get("/users/").unwrap().get.is_some());
    assert!(schema.paths.get("/users/{id}/").unwrap().get.is_some());
}

// Test 4: OpenAPI schema with nested/included routes
#[tokio::test]
async fn test_openapi_schema_with_nested_routes() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(MockHandler::new("response"));

    let sub_routes = vec![
        Route::new("/", handler.clone()).with_name("list"),
        Route::new("/{id}/", handler.clone()).with_name("detail"),
    ];

    router.include("/api/v1/items", sub_routes, Some("items".to_string()));

    let inspector = RouteInspector::new(router);
    let schema = inspector.generate_schema("Nested API", "1.0.0");

    // Verify nested paths are included
    assert!(schema.paths.contains_key("/api/v1/items/"));
    assert!(schema.paths.contains_key("/api/v1/items/{id}/"));
}

// Test 5: OpenAPI schema JSON serialization
#[tokio::test]
async fn test_openapi_schema_json_serialization() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(MockHandler::new("test"));

    router.add_route(Route::new("/test/", handler).with_name("test"));

    let inspector = RouteInspector::new(router);
    let schema = inspector.generate_schema("JSON Test", "1.0.0");

    // Serialize to JSON
    let json = schema.to_json().unwrap();

    // Verify JSON contains expected fields
    assert!(json.contains("\"openapi\""));
    assert!(json.contains("\"info\""));
    assert!(json.contains("\"paths\""));
    assert!(json.contains("\"JSON Test\""));
    assert!(json.contains("\"/test/\""));
}

// Test 6: OpenAPI schema with multiple HTTP methods
#[tokio::test]
async fn test_openapi_schema_with_multiple_methods() {
    let mut schema = OpenApiSchema::new("Multi-Method API", "1.0.0");

    // Manually create a path with multiple methods
    let mut path_item = PathItem::default();

    // GET operation
    let get_op = Operation {
        tags: None,
        summary: Some("Get item".to_string()),
        description: None,
        operation_id: Some("get-item".to_string()),
        parameters: None,
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    // POST operation
    let post_op = Operation {
        tags: None,
        summary: Some("Create item".to_string()),
        description: None,
        operation_id: Some("create-item".to_string()),
        parameters: None,
        request_body: None,
        responses: HashMap::new(),
        security: None,
    };

    path_item.get = Some(get_op);
    path_item.post = Some(post_op);

    schema.add_path("/items/".to_string(), path_item);

    // Verify both methods are present
    let path = schema.paths.get("/items/").unwrap();
    assert!(path.get.is_some());
    assert!(path.post.is_some());
    assert_eq!(
        path.get.as_ref().unwrap().operation_id,
        Some("get-item".to_string())
    );
    assert_eq!(
        path.post.as_ref().unwrap().operation_id,
        Some("create-item".to_string())
    );
}

// Test 7: OpenAPI schema with tags
#[tokio::test]
async fn test_openapi_schema_with_tags() {
    let mut schema = OpenApiSchema::new("Tagged API", "1.0.0");

    // Add tags
    schema.add_tag("users".to_string(), Some("User operations".to_string()));
    schema.add_tag("items".to_string(), Some("Item operations".to_string()));

    // Verify tags
    let tags = schema.tags.as_ref().unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].name, "users");
    assert_eq!(tags[0].description, Some("User operations".to_string()));
    assert_eq!(tags[1].name, "items");
}

// Test 8: OpenAPI schema with server information
#[tokio::test]
async fn test_openapi_schema_with_servers() {
    let schema = SchemaGenerator::new("Server API", "1.0.0")
        .description("API with server information")
        .add_server(
            "https://api.example.com",
            Some("Production server".to_string()),
        )
        .add_server(
            "https://staging.example.com",
            Some("Staging server".to_string()),
        )
        .generate()
        .unwrap();

    // Verify servers
    let servers = schema.servers.as_ref().unwrap();
    assert_eq!(servers.len(), 2);
    assert_eq!(servers[0].url, "https://api.example.com");
    assert_eq!(
        servers[0].description,
        Some("Production server".to_string())
    );
    assert_eq!(servers[1].url, "https://staging.example.com");
    assert_eq!(servers[1].description, Some("Staging server".to_string()));
}
