//! Comprehensive renderer tests based on Django REST Framework
//!
//! These tests are inspired by DRF's test_renderers.py and test_htmlrenderer.py

use serde_json::json;

/// Test basic renderer functionality
#[cfg(test)]
mod basic_renderer_tests {
    use crate::json::JSONRenderer;
    use crate::renderer::Renderer;

    use super::*;

    #[tokio::test]
    async fn test_expected_results() {
        // Test that renderer produces expected output for known inputs
        let renderer = JSONRenderer::new();
        let data = json!([1, 2, 3]);

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(output, "[1,2,3]");
    }

    #[tokio::test]
    async fn test_json_media_type() {
        let renderer = JSONRenderer::new();
        assert_eq!(
            renderer.media_types(),
            vec!["application/json", "application/json; charset=utf-8"]
        );
    }

    #[tokio::test]
    async fn test_json_format() {
        let renderer = JSONRenderer::new();
        assert_eq!(renderer.format(), Some("json"));
    }
}

/// Test JSON renderer specific functionality
#[cfg(test)]
mod json_renderer_tests {
    use crate::json::JSONRenderer;
    use crate::renderer::Renderer;

    use super::*;

    #[tokio::test]
    async fn test_render_lazy_strings() {
        // In Rust, we don't have lazy strings like Django, but we can test String rendering
        let renderer = JSONRenderer::new();
        let data = json!("test");

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(output, r#""test""#);
    }

    #[tokio::test]
    async fn test_render_dict() {
        let renderer = JSONRenderer::new();
        let data = json!({"foo": ["bar", "baz"]});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains("foo"));
        assert!(output.contains("bar"));
        assert!(output.contains("baz"));
    }

    #[tokio::test]
    async fn test_render_with_numbers() {
        let renderer = JSONRenderer::new();
        let data = json!({"name": "test", "value": 123});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains("test"));
        assert!(output.contains("123"));
    }

    #[tokio::test]
    async fn test_render_array() {
        let renderer = JSONRenderer::new();
        let data = json!([{"id": 1}, {"id": 2}, {"id": 3}]);

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains(r#""id""#));
        assert!(output.contains("1"));
        assert!(output.contains("2"));
        assert!(output.contains("3"));
    }

    #[tokio::test]
    async fn test_renderer_float_strictness() {
        let renderer = JSONRenderer::new();
        let data = json!({"value": 3.14159});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains("3.14159"));
    }

    #[tokio::test]
    async fn test_media_types_list() {
        let renderer = JSONRenderer::new();
        let media_types = renderer.media_types();

        assert_eq!(media_types.len(), 2);
        assert_eq!(media_types[0], "application/json");
        assert_eq!(media_types[1], "application/json; charset=utf-8");
    }

    #[tokio::test]
    async fn test_format_identifier() {
        let renderer = JSONRenderer::new();
        let format = renderer.format();

        assert_eq!(format, Some("json"));
    }
}

/// Test JSON rendering with different formatting options
#[cfg(test)]
mod json_formatting_tests {
    use crate::json::JSONRenderer;
    use crate::renderer::Renderer;

    use super::*;

    #[tokio::test]
    async fn test_indented_json() {
        let renderer = JSONRenderer::new().pretty(true);
        let data = json!({"foo": ["bar", "baz"]});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        // Pretty printed JSON should contain newlines and indentation
        assert!(output.contains("\n"));
        assert!(output.contains("  "));
    }

    #[tokio::test]
    async fn test_compact_json() {
        let renderer = JSONRenderer::new().pretty(false);
        let data = json!({"foo": ["bar", "baz"]});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        // Compact JSON should not contain unnecessary whitespace
        let has_multi_space = output.contains("  ");
        assert!(!has_multi_space || output.trim().is_empty());
    }

    #[tokio::test]
    async fn test_long_form_rendering() {
        let renderer = JSONRenderer::new().pretty(true);
        let data = json!({
            "users": [
                {"id": 1, "name": "Alice", "email": "alice@example.com"},
                {"id": 2, "name": "Bob", "email": "bob@example.com"},
                {"id": 3, "name": "Charlie", "email": "charlie@example.com"}
            ],
            "total": 3,
            "page": 1
        });

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains("users"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
        assert!(output.contains("Charlie"));
        assert!(output.contains("total"));
    }
}

/// Test renderer context functionality
#[cfg(test)]
mod renderer_context_tests {
    use crate::json::JSONRenderer;
    use crate::renderer::{Renderer, RendererContext};

    use super::*;

    #[tokio::test]
    async fn test_renderer_with_context_unit() {
        let renderer = JSONRenderer::new();
        let data = json!({"test": "data"});
        let context = RendererContext::new();

        let result = renderer.render(&data, Some(&context)).await.unwrap();

        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_renderer_without_context() {
        let renderer = JSONRenderer::new();
        let data = json!({"test": "data"});

        let result = renderer.render(&data, None).await.unwrap();

        assert!(!result.is_empty());
        let output = String::from_utf8(result.to_vec()).unwrap();
        assert!(output.contains("test"));
        assert!(output.contains("data"));
    }
}

/// Test Unicode and ASCII encoding
#[cfg(test)]
mod encoding_tests {
    use crate::json::JSONRenderer;
    use crate::renderer::Renderer;

    use super::*;

    #[tokio::test]
    async fn test_unicode_encoding() {
        let renderer = JSONRenderer::new();
        let data = json!({"message": "Hello, 世界! 🌍"});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains("Hello"));
        // UTF-8 encoding should preserve Unicode characters
        assert!(output.contains("世界") || output.contains("\\u"));
    }

    #[tokio::test]
    async fn test_ascii_encoding() {
        let renderer = JSONRenderer::new().ensure_ascii(true);
        let data = json!({"text": "simple ascii text"});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        // All characters should be ASCII
        assert!(output.is_ascii());
    }

    #[tokio::test]
    async fn test_special_unicode_chars() {
        // Test U+2028 (LINE SEPARATOR) and U+2029 (PARAGRAPH SEPARATOR)
        let renderer = JSONRenderer::new();
        let data = json!({"text": "line\u{2028}para\u{2029}graph"});

        let result = renderer.render(&data, None).await;
        assert!(result.is_ok());
    }
}

/// Test edge cases and error handling
#[cfg(test)]
mod edge_case_tests {
    use crate::json::JSONRenderer;
    use crate::renderer::Renderer;

    use super::*;

    #[tokio::test]
    async fn test_empty_object() {
        let renderer = JSONRenderer::new();
        let data = json!({});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(output, "{}");
    }

    #[tokio::test]
    async fn test_empty_array() {
        let renderer = JSONRenderer::new();
        let data = json!([]);

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(output, "[]");
    }

    #[tokio::test]
    async fn test_null_value() {
        let renderer = JSONRenderer::new();
        let data = json!(null);

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert_eq!(output, "null");
    }

    #[tokio::test]
    async fn test_nested_structures() {
        let renderer = JSONRenderer::new();
        let data = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "data": "deep"
                    }
                }
            }
        });

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains("level1"));
        assert!(output.contains("level2"));
        assert!(output.contains("level3"));
        assert!(output.contains("deep"));
    }

    #[tokio::test]
    async fn test_boolean_values() {
        let renderer = JSONRenderer::new();
        let data = json!({"true_val": true, "false_val": false});

        let result = renderer.render(&data, None).await.unwrap();
        let output = String::from_utf8(result.to_vec()).unwrap();

        assert!(output.contains("true"));
        assert!(output.contains("false"));
    }
}
