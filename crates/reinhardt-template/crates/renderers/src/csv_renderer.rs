use async_trait::async_trait;
use bytes::Bytes;
use csv::WriterBuilder;
use reinhardt_exception::{Error, Result};
use serde_json::Value;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// CSV renderer for tabular data
///
/// Renders JSON arrays of objects as CSV format.
/// Each object becomes a row, with keys becoming column headers.
#[derive(Debug, Clone)]
pub struct CSVRenderer {
	/// Column delimiter (default: ',')
	pub delimiter: u8,
	/// Whether to include a header row
	pub include_header: bool,
}

impl Default for CSVRenderer {
	fn default() -> Self {
		Self {
			delimiter: b',',
			include_header: true,
		}
	}
}

impl CSVRenderer {
	/// Creates a new CSV renderer with default settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::CSVRenderer;
	///
	/// let renderer = CSVRenderer::new();
	/// assert_eq!(renderer.delimiter, b',');
	/// assert!(renderer.include_header);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the column delimiter
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::CSVRenderer;
	///
	/// let renderer = CSVRenderer::new().delimiter(b';');
	/// assert_eq!(renderer.delimiter, b';');
	/// ```
	pub fn delimiter(mut self, delimiter: u8) -> Self {
		self.delimiter = delimiter;
		self
	}

	/// Sets whether to include header row
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::CSVRenderer;
	///
	/// let renderer = CSVRenderer::new().include_header(false);
	/// assert!(!renderer.include_header);
	/// ```
	pub fn include_header(mut self, include: bool) -> Self {
		self.include_header = include;
		self
	}

	/// Convert JSON value to CSV string
	fn value_to_csv(&self, data: &Value) -> Result<String> {
		let mut wtr = WriterBuilder::new()
			.delimiter(self.delimiter)
			.from_writer(vec![]);

		match data {
			Value::Array(items) if !items.is_empty() => {
				// Extract headers from first object
				if let Some(Value::Object(first)) = items.first() {
					let headers: Vec<String> = first.keys().cloned().collect();

					// Write header row if enabled
					if self.include_header {
						wtr.write_record(&headers)
							.map_err(|e| Error::Serialization(e.to_string()))?;
					}

					// Write each row
					for item in items {
						if let Value::Object(obj) = item {
							let row: Vec<String> = headers
								.iter()
								.map(|key| {
									obj.get(key)
										.map(|v| match v {
											Value::String(s) => s.clone(),
											Value::Number(n) => n.to_string(),
											Value::Bool(b) => b.to_string(),
											Value::Null => String::new(),
											_ => serde_json::to_string(v).unwrap_or_default(),
										})
										.unwrap_or_default()
								})
								.collect();

							wtr.write_record(&row)
								.map_err(|e| Error::Serialization(e.to_string()))?;
						}
					}
				} else {
					return Err(Error::Serialization(
						"CSV renderer requires an array of objects".to_string(),
					));
				}
			}
			Value::Array(_) => {
				// Empty array - just write header if enabled
				if self.include_header {
					wtr.write_record(&[] as &[String])
						.map_err(|e| Error::Serialization(e.to_string()))?;
				}
			}
			_ => {
				return Err(Error::Serialization(
					"CSV renderer requires an array of objects".to_string(),
				));
			}
		}

		wtr.flush()
			.map_err(|e| Error::Serialization(e.to_string()))?;

		let bytes = wtr
			.into_inner()
			.map_err(|e| Error::Serialization(e.to_string()))?;

		String::from_utf8(bytes).map_err(|e| Error::Serialization(e.to_string()))
	}
}

#[async_trait]
impl Renderer for CSVRenderer {
	fn media_types(&self) -> Vec<String> {
		vec!["text/csv".to_string()]
	}

	fn format(&self) -> Option<&str> {
		Some("csv")
	}

	async fn render(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Bytes> {
		let csv_string = self.value_to_csv(data)?;
		Ok(Bytes::from(csv_string))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[tokio::test]
	async fn test_csv_renderer_basic() {
		let renderer = CSVRenderer::new();
		let data = json!([
			{"name": "Alice", "age": 30},
			{"name": "Bob", "age": 25}
		]);

		let result = renderer.render(&data, None).await.unwrap();
		let csv_str = String::from_utf8(result.to_vec()).unwrap();

		assert!(csv_str.contains("name"));
		assert!(csv_str.contains("Alice"));
		assert!(csv_str.contains("30"));
	}

	#[tokio::test]
	async fn test_csv_renderer_custom_delimiter() {
		let renderer = CSVRenderer::new().delimiter(b';');
		let data = json!([
			{"name": "Alice", "age": 30}
		]);

		let result = renderer.render(&data, None).await.unwrap();
		let csv_str = String::from_utf8(result.to_vec()).unwrap();

		assert!(csv_str.contains(';'));
	}

	#[tokio::test]
	async fn test_csv_renderer_no_header() {
		let renderer = CSVRenderer::new().include_header(false);
		let data = json!([
			{"name": "Alice", "age": 30}
		]);

		let result = renderer.render(&data, None).await.unwrap();
		let csv_str = String::from_utf8(result.to_vec()).unwrap();

		assert!(!csv_str.contains("name"));
		assert!(csv_str.contains("Alice"));
	}

	#[tokio::test]
	async fn test_csv_renderer_empty_array() {
		let renderer = CSVRenderer::new();
		let data = json!([]);

		let result = renderer.render(&data, None).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_csv_renderer_invalid_input() {
		let renderer = CSVRenderer::new();
		let data = json!({"not": "an array"});

		let result = renderer.render(&data, None).await;
		assert!(result.is_err());
	}
}
