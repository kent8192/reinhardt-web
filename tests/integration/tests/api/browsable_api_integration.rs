//! Integration tests for Browsable API
//!
//! These tests verify that reinhardt-browsable-api works correctly for rendering
//! interactive HTML API documentation.

use reinhardt_browsable_api::{ApiContext, BrowsableApiRenderer, FormContext, FormField};
use serde_json::json;

// ============================================================================
// API Browser Rendering Tests
// ============================================================================

#[test]
fn test_browsable_api_html_response() {
    let renderer = BrowsableApiRenderer::new();

    let context = ApiContext {
        title: "User List API".to_string(),
        description: Some("Get a list of all users".to_string()),
        endpoint: "/api/users/".to_string(),
        method: "GET".to_string(),
        response_data: json!([
            {"id": 1, "name": "Alice", "email": "alice@example.com"},
            {"id": 2, "name": "Bob", "email": "bob@example.com"}
        ]),
        response_status: 200,
        allowed_methods: vec!["GET".to_string(), "POST".to_string()],
        request_form: None,
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
    };

    let html = renderer.render(&context).unwrap();

    // Verify HTML structure
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<html>"));
    assert!(html.contains("User List API"));
    assert!(html.contains("Get a list of all users"));
    assert!(html.contains("/api/users/"));
    assert!(html.contains("Alice"));
    assert!(html.contains("alice@example.com"));
}

#[test]
fn test_browsable_api_json_data_display() {
    let renderer = BrowsableApiRenderer::new();

    let context = ApiContext {
        title: "Product Detail".to_string(),
        description: None,
        endpoint: "/api/products/42/".to_string(),
        method: "GET".to_string(),
        response_data: json!({
            "id": 42,
            "name": "Laptop",
            "price": 999.99,
            "in_stock": true,
            "specs": {
                "cpu": "Intel i7",
                "ram": "16GB"
            }
        }),
        response_status: 200,
        allowed_methods: vec!["GET".to_string(), "PUT".to_string(), "DELETE".to_string()],
        request_form: None,
        headers: vec![],
    };

    let html = renderer.render(&context).unwrap();

    // Verify JSON data is displayed
    assert!(html.contains("Laptop"));
    assert!(html.contains("999.99"));
    assert!(html.contains("Intel i7"));
    assert!(html.contains("16GB"));
}

// ============================================================================
// Interactive Features Tests
// ============================================================================

#[test]
fn test_browsable_api_form_rendering() {
    let renderer = BrowsableApiRenderer::new();

    let form = FormContext {
        fields: vec![
            FormField {
                name: "title".to_string(),
                label: "Title".to_string(),
                field_type: "text".to_string(),
                required: true,
                help_text: Some("Enter the post title".to_string()),
                initial_value: None,
                options: None,
                initial_label: None,
            },
            FormField {
                name: "content".to_string(),
                label: "Content".to_string(),
                field_type: "textarea".to_string(),
                required: true,
                help_text: None,
                initial_value: None,
                options: None,
                initial_label: None,
            },
        ],
        submit_url: "/api/posts/".to_string(),
        submit_method: "POST".to_string(),
    };

    let context = ApiContext {
        title: "Create Post".to_string(),
        description: Some("Create a new blog post".to_string()),
        endpoint: "/api/posts/".to_string(),
        method: "POST".to_string(),
        response_data: json!({}),
        response_status: 201,
        allowed_methods: vec!["POST".to_string()],
        request_form: Some(form),
        headers: vec![],
    };

    let html = renderer.render(&context).unwrap();

    // Verify form elements are present
    assert!(html.contains("form"));
    assert!(html.contains("title"));
    assert!(html.contains("content"));
    assert!(html.contains("Enter the post title"));
}

#[test]
fn test_browsable_api_method_selection() {
    let renderer = BrowsableApiRenderer::new();

    let context = ApiContext {
        title: "Resource Endpoint".to_string(),
        description: None,
        endpoint: "/api/resource/1/".to_string(),
        method: "GET".to_string(),
        response_data: json!({"id": 1}),
        response_status: 200,
        allowed_methods: vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "PATCH".to_string(),
            "DELETE".to_string(),
        ],
        request_form: None,
        headers: vec![],
    };

    let html = renderer.render(&context).unwrap();

    // Verify all HTTP methods are displayed
    assert!(html.contains("GET"));
    assert!(html.contains("POST"));
    assert!(html.contains("PUT"));
    assert!(html.contains("PATCH"));
    assert!(html.contains("DELETE"));
    assert!(html.contains("Allowed methods"));
}

#[test]
fn test_browsable_api_authentication_ui() {
    let renderer = BrowsableApiRenderer::new();

    // Simulate authenticated request
    let context = ApiContext {
        title: "Authenticated Endpoint".to_string(),
        description: Some("Requires authentication".to_string()),
        endpoint: "/api/protected/".to_string(),
        method: "GET".to_string(),
        response_data: json!({"message": "Authenticated"}),
        response_status: 200,
        allowed_methods: vec!["GET".to_string()],
        request_form: None,
        headers: vec![("Authorization".to_string(), "Bearer token123".to_string())],
    };

    let html = renderer.render(&context).unwrap();

    // Verify authentication-related content
    assert!(html.contains("Authenticated Endpoint"));
    assert!(html.contains("Requires authentication"));
}

// ============================================================================
// Content Negotiation Tests
// ============================================================================

#[test]
fn test_browsable_api_accepts_header() {
    let renderer = BrowsableApiRenderer::new();

    // HTML Accept header scenario
    let context = ApiContext {
        title: "Content Negotiation Test".to_string(),
        description: None,
        endpoint: "/api/data/".to_string(),
        method: "GET".to_string(),
        response_data: json!({"data": "test"}),
        response_status: 200,
        allowed_methods: vec!["GET".to_string()],
        request_form: None,
        headers: vec![("Accept".to_string(), "text/html".to_string())],
    };

    let html = renderer.render(&context).unwrap();

    // Should render HTML for browser
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("Content Negotiation Test"));
}

#[test]
fn test_browsable_api_json_response_display() {
    let renderer = BrowsableApiRenderer::new();

    // JSON Accept header - but still display in browsable UI
    let context = ApiContext {
        title: "JSON Response".to_string(),
        description: None,
        endpoint: "/api/json/".to_string(),
        method: "GET".to_string(),
        response_data: json!({
            "status": "success",
            "data": [1, 2, 3, 4, 5]
        }),
        response_status: 200,
        allowed_methods: vec!["GET".to_string()],
        request_form: None,
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
    };

    let html = renderer.render(&context).unwrap();

    // JSON data should be visible in HTML
    assert!(html.contains("success"));
    assert!(html.contains("[") || html.contains("data"));
}

// ============================================================================
// Response Status Tests
// ============================================================================

#[test]
fn test_browsable_api_error_response() {
    let renderer = BrowsableApiRenderer::new();

    let context = ApiContext {
        title: "Error Response".to_string(),
        description: None,
        endpoint: "/api/error/".to_string(),
        method: "POST".to_string(),
        response_data: json!({
            "error": "Validation failed",
            "details": {
                "email": ["This field is required"]
            }
        }),
        response_status: 400,
        allowed_methods: vec!["POST".to_string()],
        request_form: None,
        headers: vec![],
    };

    let html = renderer.render(&context).unwrap();

    // Error details should be visible
    assert!(html.contains("Validation failed"));
    assert!(html.contains("This field is required"));
}

#[test]
fn test_browsable_api_success_created() {
    let renderer = BrowsableApiRenderer::new();

    let context = ApiContext {
        title: "Resource Created".to_string(),
        description: None,
        endpoint: "/api/items/".to_string(),
        method: "POST".to_string(),
        response_data: json!({
            "id": 123,
            "created_at": "2024-01-01T00:00:00Z"
        }),
        response_status: 201,
        allowed_methods: vec!["GET".to_string(), "POST".to_string()],
        request_form: None,
        headers: vec![("Location".to_string(), "/api/items/123/".to_string())],
    };

    let html = renderer.render(&context).unwrap();

    assert!(html.contains("Resource Created"));
    assert!(html.contains("123"));
}

// ============================================================================
// Form Field Types Tests
// ============================================================================

#[test]
fn test_browsable_api_various_field_types() {
    let renderer = BrowsableApiRenderer::new();

    let form = FormContext {
        fields: vec![
            FormField {
                name: "name".to_string(),
                label: "Name".to_string(),
                field_type: "text".to_string(),
                required: true,
                help_text: None,
                initial_value: Some(json!("John Doe")),
                options: None,
                initial_label: None,
            },
            FormField {
                name: "age".to_string(),
                label: "Age".to_string(),
                field_type: "number".to_string(),
                required: false,
                help_text: Some("Must be 18 or older".to_string()),
                initial_value: None,
                options: None,
                initial_label: None,
            },
            FormField {
                name: "bio".to_string(),
                label: "Biography".to_string(),
                field_type: "textarea".to_string(),
                required: false,
                help_text: None,
                initial_value: None,
                options: None,
                initial_label: None,
            },
        ],
        submit_url: "/api/profile/".to_string(),
        submit_method: "PUT".to_string(),
    };

    let context = ApiContext {
        title: "Update Profile".to_string(),
        description: None,
        endpoint: "/api/profile/".to_string(),
        method: "PUT".to_string(),
        response_data: json!({}),
        response_status: 200,
        allowed_methods: vec!["GET".to_string(), "PUT".to_string()],
        request_form: Some(form),
        headers: vec![],
    };

    let html = renderer.render(&context).unwrap();

    // Verify different field types
    assert!(html.contains("name"));
    assert!(html.contains("age"));
    assert!(html.contains("bio"));
    assert!(html.contains("Must be 18 or older"));
}
