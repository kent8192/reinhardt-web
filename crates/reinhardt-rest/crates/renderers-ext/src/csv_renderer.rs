use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::Error;
use serde_json::Value;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// CSV renderer for REST API responses
#[derive(Debug, Clone)]
pub struct CSVRenderer {
    /// CSV delimiter (default: ',')
    pub delimiter: u8,
    /// Include headers in output
    pub headers: bool,
}

impl Default for CSVRenderer {
    fn default() -> Self {
        Self {
            delimiter: b',',
            headers: true,
        }
    }
}

impl CSVRenderer {
    /// Creates a new CSV renderer
    ///
    /// # Examples
    ///
    /// ```
    /// use renderers_ext::CSVRenderer;
    ///
    /// let renderer = CSVRenderer::new();
    /// assert_eq!(renderer.delimiter, b',');
    /// assert!(renderer.headers);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the delimiter character
    ///
    /// # Examples
    ///
    /// ```
    /// use renderers_ext::CSVRenderer;
    ///
    /// let renderer = CSVRenderer::new().delimiter(b';');
    /// assert_eq!(renderer.delimiter, b';');
    /// ```
    pub fn delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Sets whether to include headers
    ///
    /// # Examples
    ///
    /// ```
    /// use renderers_ext::CSVRenderer;
    ///
    /// let renderer = CSVRenderer::new().headers(false);
    /// assert!(!renderer.headers);
    /// ```
    pub fn headers(mut self, headers: bool) -> Self {
        self.headers = headers;
        self
    }
}

#[async_trait]
impl Renderer for CSVRenderer {
    fn media_type(&self) -> String {
        "text/csv; charset=utf-8".to_string()
    }

    fn media_types(&self) -> Vec<String> {
        vec![
            "text/csv".to_string(),
            "text/csv; charset=utf-8".to_string(),
            "application/csv".to_string(),
        ]
    }

    fn format(&self) -> Option<&str> {
        Some("csv")
    }

    async fn render(
        &self,
        data: &Value,
        _context: Option<&RendererContext>,
    ) -> RenderResult<Bytes> {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.headers)
            .from_writer(vec![]);

        match data {
            Value::Array(items) => {
                for item in items {
                    if let Value::Object(map) = item {
                        // Write headers (first row only)
                        if self.headers && wtr.get_ref().is_empty() {
                            let headers: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
                            wtr.write_record(&headers)
                                .map_err(|e| Error::Serialization(e.to_string()))?;
                        }

                        // Write values
                        let values: Vec<String> = map
                            .values()
                            .map(|v| match v {
                                Value::String(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .collect();
                        wtr.write_record(&values)
                            .map_err(|e| Error::Serialization(e.to_string()))?;
                    }
                }
            }
            Value::Object(map) => {
                // Single object
                if self.headers {
                    let headers: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
                    wtr.write_record(&headers)
                        .map_err(|e| Error::Serialization(e.to_string()))?;
                }

                let values: Vec<String> = map
                    .values()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    })
                    .collect();
                wtr.write_record(&values)
                    .map_err(|e| Error::Serialization(e.to_string()))?;
            }
            _ => {
                return Err(Error::Serialization(
                    "CSV renderer expects array or object".to_string(),
                ))
            }
        }

        let data = wtr
            .into_inner()
            .map_err(|e| Error::Serialization(e.to_string()))?;
        Ok(Bytes::from(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_csv_renderer() {
        let renderer = CSVRenderer::new();
        let data = json!([
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]);

        let result = renderer.render(&data, None).await;
        assert!(result.is_ok());

        let bytes = result.unwrap();
        let csv_str = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(csv_str.contains("name"));
        assert!(csv_str.contains("Alice"));
        assert!(csv_str.contains("Bob"));
    }

    #[tokio::test]
    async fn test_csv_renderer_delimiter() {
        let renderer = CSVRenderer::new().delimiter(b';');
        let data = json!([{"name": "Alice", "age": 30}]);

        let result = renderer.render(&data, None).await.unwrap();
        let csv_str = String::from_utf8(result.to_vec()).unwrap();

        assert!(csv_str.contains(';'));
    }

    #[tokio::test]
    async fn test_csv_renderer_no_headers() {
        let renderer = CSVRenderer::new().headers(false);
        let data = json!([{"name": "Alice", "age": 30}]);

        let result = renderer.render(&data, None).await.unwrap();
        let csv_str = String::from_utf8(result.to_vec()).unwrap();

        assert!(!csv_str.contains("name"));
        assert!(csv_str.contains("Alice"));
    }
}
