//! Integration tests for renderer functionality
//!
//! These tests verify that reinhardt-renderers work correctly.

use reinhardt_renderers::{JSONRenderer, Renderer, RendererContext, XMLRenderer, YAMLRenderer};
use serde_json::json;

// ============================================================================
// JSON Renderer Tests
// ============================================================================

#[tokio::test]
async fn test_json_renderer_basic() {
    let renderer = JSONRenderer::new();
    let data = json!({
        "name": "Alice",
        "age": 30,
        "active": true
    });

    let result = renderer.render(&data, None).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(json_str.contains("Alice"));
    assert!(json_str.contains("30"));
    assert!(json_str.contains("true"));
}

#[tokio::test]
async fn test_json_renderer_array() {
    let renderer = JSONRenderer::new();
    let data = json!([
        {"id": 1, "name": "Item 1"},
        {"id": 2, "name": "Item 2"},
        {"id": 3, "name": "Item 3"}
    ]);

    let result = renderer.render(&data, None).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(json_str.contains("Item 1"));
    assert!(json_str.contains("Item 2"));
    assert!(json_str.contains("Item 3"));
}

#[tokio::test]
async fn test_json_renderer_media_type() {
    let renderer = JSONRenderer::new();
    assert_eq!(renderer.media_type(), "application/json");
    assert_eq!(renderer.format(), Some("json"));
}

#[tokio::test]
async fn test_json_renderer_nested_data() {
    let renderer = JSONRenderer::new();
    let data = json!({
        "user": {
            "name": "Bob",
            "address": {
                "city": "Boston",
                "zip": "02101"
            }
        }
    });

    let result = renderer.render(&data, None).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(json_str.contains("Bob"));
    assert!(json_str.contains("Boston"));
    assert!(json_str.contains("02101"));
}

// ============================================================================
// XML Renderer Tests
// ============================================================================

#[tokio::test]
async fn test_xml_renderer_basic() {
    let renderer = XMLRenderer::new();
    let data = json!({
        "name": "Charlie",
        "age": 35
    });

    let result = renderer.render(&data, None).await.unwrap();
    let xml_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(xml_str.contains("<root>"));
    assert!(xml_str.contains("<name>"));
    assert!(xml_str.contains("Charlie"));
    assert!(xml_str.contains("<age>"));
    assert!(xml_str.contains("35"));
    assert!(xml_str.contains("</root>"));
}

#[tokio::test]
async fn test_xml_renderer_media_type() {
    let renderer = XMLRenderer::new();
    assert_eq!(renderer.media_type(), "application/xml");
    assert_eq!(renderer.format(), Some("xml"));
}

#[tokio::test]
async fn test_xml_renderer_array() {
    let renderer = XMLRenderer::new();
    let data = json!([
        {"id": 1, "name": "Item 1"},
        {"id": 2, "name": "Item 2"}
    ]);

    let result = renderer.render(&data, None).await.unwrap();
    let xml_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(xml_str.contains("<root>"));
    assert!(xml_str.contains("<item>"));
    assert!(xml_str.contains("Item 1"));
    assert!(xml_str.contains("Item 2"));
}

// ============================================================================
// YAML Renderer Tests
// ============================================================================

#[tokio::test]
async fn test_yaml_renderer_basic() {
    let renderer = YAMLRenderer::new();
    let data = json!({
        "name": "Diana",
        "age": 28
    });

    let result = renderer.render(&data, None).await.unwrap();
    let yaml_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(yaml_str.contains("name:"));
    assert!(yaml_str.contains("Diana"));
    assert!(yaml_str.contains("age:"));
    assert!(yaml_str.contains("28"));
}

#[tokio::test]
async fn test_yaml_renderer_media_type() {
    let renderer = YAMLRenderer::new();
    // YAML can have multiple media types
    let media_types = renderer.media_types();
    assert!(
        media_types.contains(&"application/yaml".to_string())
            || media_types.contains(&"text/yaml".to_string())
    );
    assert_eq!(renderer.format(), Some("yaml"));
}

#[tokio::test]
async fn test_yaml_renderer_nested() {
    let renderer = YAMLRenderer::new();
    let data = json!({
        "user": {
            "name": "Eve",
            "settings": {
                "theme": "dark",
                "notifications": true
            }
        }
    });

    let result = renderer.render(&data, None).await.unwrap();
    let yaml_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(yaml_str.contains("user:"));
    assert!(yaml_str.contains("Eve"));
    assert!(yaml_str.contains("settings:"));
    assert!(yaml_str.contains("theme:"));
}

// ============================================================================
// Renderer Context Tests
// ============================================================================

#[tokio::test]
async fn test_renderer_with_context() {
    let renderer = JSONRenderer::new();
    let data = json!({"message": "Hello"});

    let context = RendererContext::new()
        .with_request("GET", "/api/test")
        .with_view("TestView", "Test view description");

    let result = renderer.render(&data, Some(&context)).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    // Context doesn't affect JSON output, but should not error
    assert!(json_str.contains("Hello"));
}

#[test]
fn test_renderer_context_builder() {
    let context = RendererContext::new()
        .with_request("POST", "/api/users")
        .with_view("UserCreate", "Create a new user")
        .with_extra("custom_key", "custom_value");

    assert_eq!(context.request_method, Some("POST".to_string()));
    assert_eq!(context.request_path, Some("/api/users".to_string()));
    assert_eq!(context.view_name, Some("UserCreate".to_string()));
    assert_eq!(
        context.view_description,
        Some("Create a new user".to_string())
    );
    assert_eq!(
        context.extra.get("custom_key"),
        Some(&"custom_value".to_string())
    );
}

// ============================================================================
// Content Type Tests
// ============================================================================

#[test]
fn test_json_renderer_content_type() {
    let renderer = JSONRenderer::new();
    let content_type = renderer.content_type();

    assert!(content_type.contains("application/json"));
    assert!(content_type.contains("charset=utf-8"));
}

#[test]
fn test_xml_renderer_content_type() {
    let renderer = XMLRenderer::new();
    let content_type = renderer.content_type();

    assert!(content_type.contains("application/xml"));
    assert!(content_type.contains("charset=utf-8"));
}

#[test]
fn test_yaml_renderer_content_type() {
    let renderer = YAMLRenderer::new();
    let content_type = renderer.content_type();

    // YAML content type varies
    assert!(content_type.contains("application/yaml") || content_type.contains("text/yaml"));
    assert!(content_type.contains("charset=utf-8"));
}

// ============================================================================
// Special Data Types Tests
// ============================================================================

#[tokio::test]
async fn test_json_renderer_null_value() {
    let renderer = JSONRenderer::new();
    let data = json!(null);

    let result = renderer.render(&data, None).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    assert_eq!(json_str.trim(), "null");
}

#[tokio::test]
async fn test_json_renderer_boolean_values() {
    let renderer = JSONRenderer::new();

    let data_true = json!(true);
    let result_true = renderer.render(&data_true, None).await.unwrap();
    assert_eq!(
        String::from_utf8(result_true.to_vec()).unwrap().trim(),
        "true"
    );

    let data_false = json!(false);
    let result_false = renderer.render(&data_false, None).await.unwrap();
    assert_eq!(
        String::from_utf8(result_false.to_vec()).unwrap().trim(),
        "false"
    );
}

#[tokio::test]
async fn test_json_renderer_number_values() {
    let renderer = JSONRenderer::new();

    let data = json!({
        "integer": 42,
        "float": 3.14,
        "negative": -10
    });

    let result = renderer.render(&data, None).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    assert!(json_str.contains("42"));
    assert!(json_str.contains("3.14"));
    assert!(json_str.contains("-10"));
}

#[tokio::test]
async fn test_json_renderer_empty_object() {
    let renderer = JSONRenderer::new();
    let data = json!({});

    let result = renderer.render(&data, None).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    assert_eq!(json_str.trim(), "{}");
}

#[tokio::test]
async fn test_json_renderer_empty_array() {
    let renderer = JSONRenderer::new();
    let data = json!([]);

    let result = renderer.render(&data, None).await.unwrap();
    let json_str = String::from_utf8(result.to_vec()).unwrap();

    assert_eq!(json_str.trim(), "[]");
}
