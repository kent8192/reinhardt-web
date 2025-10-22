use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::Error;
use serde_json::Value;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// YAML renderer
///
/// Renders JSON data as YAML format.
#[derive(Debug, Clone)]
pub struct YAMLRenderer {
    /// Whether to use compact format
    pub compact: bool,
}

impl Default for YAMLRenderer {
    fn default() -> Self {
        Self { compact: false }
    }
}

impl YAMLRenderer {
    /// Creates a new YAML renderer with default settings
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_renderers::YAMLRenderer;
    ///
    /// let renderer = YAMLRenderer::new();
    /// assert!(!renderer.compact);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to use compact format
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_renderers::YAMLRenderer;
    ///
    /// let renderer = YAMLRenderer::new().compact(true);
    /// assert!(renderer.compact);
    /// ```
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

#[async_trait]
impl Renderer for YAMLRenderer {
    fn media_types(&self) -> Vec<String> {
        vec!["application/yaml".to_string(), "text/yaml".to_string()]
    }

    fn format(&self) -> Option<&str> {
        Some("yaml")
    }

    async fn render(
        &self,
        data: &Value,
        _context: Option<&RendererContext>,
    ) -> RenderResult<Bytes> {
        let yaml_string =
            serde_yaml::to_string(data).map_err(|e| Error::Serialization(e.to_string()))?;

        Ok(Bytes::from(yaml_string))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_yaml_renderer_basic() {
        let renderer = YAMLRenderer::new();
        let data = json!({"name": "test", "value": 123});

        let result = renderer.render(&data, None).await.unwrap();
        let yaml_str = String::from_utf8(result.to_vec()).unwrap();

        assert!(yaml_str.contains("name:"));
        assert!(yaml_str.contains("test"));
        assert!(yaml_str.contains("123"));
    }

    #[tokio::test]
    async fn test_yaml_renderer_array() {
        let renderer = YAMLRenderer::new();
        let data = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);

        let result = renderer.render(&data, None).await.unwrap();
        let yaml_str = String::from_utf8(result.to_vec()).unwrap();

        assert!(yaml_str.contains("Alice"));
        assert!(yaml_str.contains("30"));
        assert!(yaml_str.contains("Bob"));
    }

    #[tokio::test]
    async fn test_yaml_renderer_nested() {
        let renderer = YAMLRenderer::new();
        let data = json!({
            "user": {
                "name": "Alice",
                "profile": {
                    "age": 30
                }
            }
        });

        let result = renderer.render(&data, None).await.unwrap();
        let yaml_str = String::from_utf8(result.to_vec()).unwrap();

        assert!(yaml_str.contains("user:"));
        assert!(yaml_str.contains("profile:"));
    }
}
