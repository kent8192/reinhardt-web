// Advanced Renderer Integration Tests
// Tests for advanced renderer functionality including CSV, OpenAPI,
// RendererRegistry, content negotiation, and multi-format rendering

use reinhardt_renderers::{
    CSVRenderer, JSONRenderer, OpenAPIRenderer, Renderer, RendererContext, RendererRegistry,
    XMLRenderer, YAMLRenderer,
};
use serde_json::json;

// ============================================================================
// CSV Renderer Tests
// ============================================================================

#[tokio::test]
async fn test_csv_renderer_array_of_objects() {
    let renderer = CSVRenderer::new();
    let data = json!([
        {"name": "Alice", "age": 30},
        {"name": "Bob", "age": 25}
    ]);

    let result = renderer.render(&data, None).await.unwrap();
    let text = String::from_utf8(result.to_vec()).unwrap();

    assert!(text.contains("name"));
    assert!(text.contains("Alice"));
    assert!(text.contains("Bob"));
    assert_eq!(renderer.media_type(), "text/csv");
}

#[tokio::test]
async fn test_csv_renderer_single_object() {
    let renderer = CSVRenderer::new();
    // CSV renderer requires an array, even for single object
    let data = json!([{"name": "Alice", "age": 30, "city": "NYC"}]);

    let result = renderer.render(&data, None).await.unwrap();
    let text = String::from_utf8(result.to_vec()).unwrap();

    // CSV should handle array with single object
    assert!(text.len() > 0);
    assert!(text.contains("Alice"));
}

// ============================================================================
// RendererRegistry Tests
// ============================================================================

#[tokio::test]
async fn test_renderer_registry_json() {
    let registry = RendererRegistry::new()
        .register(JSONRenderer::new())
        .register(XMLRenderer::new());

    let data = json!({"test": "value"});
    let (bytes, content_type) = registry.render(&data, Some("json"), None).await.unwrap();

    assert!(bytes.len() > 0);
    assert!(content_type.contains("application/json"));
}

#[tokio::test]
async fn test_renderer_registry_xml() {
    let registry = RendererRegistry::new()
        .register(JSONRenderer::new())
        .register(XMLRenderer::new());

    let data = json!({"test": "value"});
    let (bytes, content_type) = registry.render(&data, Some("xml"), None).await.unwrap();

    assert!(bytes.len() > 0);
    assert!(content_type.contains("application/xml"));
}

#[tokio::test]
async fn test_renderer_registry_default_format() {
    let registry = RendererRegistry::new()
        .register(JSONRenderer::new())
        .register(XMLRenderer::new());

    let data = json!({"test": "value"});
    let (bytes, content_type) = registry.render(&data, None, None).await.unwrap();

    // Should use first registered renderer (JSON)
    assert!(bytes.len() > 0);
    assert!(content_type.contains("application/json"));
}

#[tokio::test]
async fn test_renderer_registry_unsupported_format() {
    let registry = RendererRegistry::new().register(JSONRenderer::new());

    let data = json!({"test": "value"});
    let result = registry.render(&data, Some("pdf"), None).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_renderer_get_by_media_type() {
    let registry = RendererRegistry::new()
        .register(JSONRenderer::new())
        .register(XMLRenderer::new())
        .register(YAMLRenderer::new());

    let json_renderer = registry.get_renderer_by_media_type("application/json");
    assert!(json_renderer.is_some());

    let xml_renderer = registry.get_renderer_by_media_type("application/xml");
    assert!(xml_renderer.is_some());

    let yaml_renderer = registry.get_renderer_by_media_type("application/yaml");
    assert!(yaml_renderer.is_some());
}

// ============================================================================
// OpenAPI Renderer Tests
// ============================================================================

#[tokio::test]
async fn test_openapi_renderer_basic() {
    let renderer = OpenAPIRenderer::new("Test API", "1.0.0");

    // OpenAPI renderer generates schema from provided title and version
    let data = json!({
        "paths": {
            "/users": {
                "get": {
                    "summary": "List users"
                }
            }
        }
    });

    let result = renderer.render(&data, None).await;
    assert!(result.is_ok());
}

// ============================================================================
// Multi-format Rendering Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_renderers_same_data() {
    let data = json!({"name": "Test", "value": 123});

    let json_renderer = JSONRenderer::new();
    let xml_renderer = XMLRenderer::new();
    let yaml_renderer = YAMLRenderer::new();

    let json_result = json_renderer.render(&data, None).await.unwrap();
    let xml_result = xml_renderer.render(&data, None).await.unwrap();
    let yaml_result = yaml_renderer.render(&data, None).await.unwrap();

    // All should successfully render the same data
    assert!(json_result.len() > 0);
    assert!(xml_result.len() > 0);
    assert!(yaml_result.len() > 0);
}

// ============================================================================
// Unicode and Special Character Tests
// ============================================================================

#[tokio::test]
async fn test_renderer_unicode_data() {
    let renderer = JSONRenderer::new();
    let data = json!({
        "japanese": "こんにちは",
        "emoji": "🚀",
        "chinese": "你好"
    });

    let result = renderer.render(&data, None).await.unwrap();
    let text = String::from_utf8(result.to_vec()).unwrap();

    assert!(text.contains("こんにちは"));
    assert!(text.contains("🚀"));
    assert!(text.contains("你好"));
}
