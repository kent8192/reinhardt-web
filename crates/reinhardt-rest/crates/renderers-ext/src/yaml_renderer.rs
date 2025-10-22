use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::Error;
use serde_json::Value;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// YAML renderer for REST API responses
#[derive(Debug, Clone)]
pub struct YAMLRenderer {
    /// Whether to use flow style
    pub flow_style: bool,
}

impl Default for YAMLRenderer {
    fn default() -> Self {
        Self { flow_style: false }
    }
}

impl YAMLRenderer {
    /// Creates a new YAML renderer
    ///
    /// # Examples
    ///
    /// ```
    /// use renderers_ext::YAMLRenderer;
    ///
    /// let renderer = YAMLRenderer::new();
    /// assert!(!renderer.flow_style);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to use flow style
    ///
    /// # Examples
    ///
    /// ```
    /// use renderers_ext::YAMLRenderer;
    ///
    /// let renderer = YAMLRenderer::new().flow_style(true);
    /// assert!(renderer.flow_style);
    /// ```
    pub fn flow_style(mut self, flow: bool) -> Self {
        self.flow_style = flow;
        self
    }
}

#[async_trait]
impl Renderer for YAMLRenderer {
    fn media_type(&self) -> String {
        "application/yaml; charset=utf-8".to_string()
    }

    fn media_types(&self) -> Vec<String> {
        vec![
            "application/yaml".to_string(),
            "application/x-yaml".to_string(),
            "text/yaml".to_string(),
        ]
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
    async fn test_yaml_renderer() {
        let renderer = YAMLRenderer::new();
        let data = json!({
            "name": "test",
            "value": 123
        });

        let result = renderer.render(&data, None).await;
        assert!(result.is_ok());

        let bytes = result.unwrap();
        let yaml_str = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(yaml_str.contains("name"));
        assert!(yaml_str.contains("test"));
        assert!(yaml_str.contains("value"));
        assert!(yaml_str.contains("123"));
    }

    #[tokio::test]
    async fn test_yaml_renderer_array() {
        let renderer = YAMLRenderer::new();
        let data = json!([
            {"name": "Alice"},
            {"name": "Bob"}
        ]);

        let result = renderer.render(&data, None).await.unwrap();
        let yaml_str = String::from_utf8(result.to_vec()).unwrap();

        assert!(yaml_str.contains("Alice"));
        assert!(yaml_str.contains("Bob"));
    }
}
