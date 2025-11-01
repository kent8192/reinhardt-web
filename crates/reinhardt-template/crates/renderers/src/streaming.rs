//! Streaming renderers for large datasets
//!
//! This module provides streaming renderers that can handle large datasets
//! by streaming data incrementally instead of buffering the entire response.

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream;
use futures::stream::{Stream, StreamExt};
use reinhardt_exception::Error;
use serde_json::Value;
use std::pin::Pin;
use std::time::Duration;

use crate::renderer::{RenderResult, RendererContext};

/// Error type for streaming operations
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
	#[error("JSON serialization error: {0}")]
	JsonError(#[from] serde_json::Error),

	#[error("CSV serialization error: {0}")]
	CsvError(String),

	#[error("Stream error: {0}")]
	StreamError(String),
}

impl From<StreamError> for Error {
	fn from(err: StreamError) -> Self {
		Error::Http(err.to_string())
	}
}

/// Configuration for streaming renderers
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::streaming::StreamingConfig;
/// use std::time::Duration;
///
/// let config = StreamingConfig::new()
///     .with_buffer_size(8192)
///     .with_flush_interval(Duration::from_millis(100));
/// ```
#[derive(Debug, Clone)]
pub struct StreamingConfig {
	/// Buffer size in bytes for accumulating data before flushing
	pub buffer_size: usize,
	/// Optional interval for automatic flushing
	pub flush_interval: Option<Duration>,
}

impl Default for StreamingConfig {
	fn default() -> Self {
		Self {
			buffer_size: 4096,
			flush_interval: None,
		}
	}
}

impl StreamingConfig {
	/// Creates a new StreamingConfig with default values
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::streaming::StreamingConfig;
	///
	/// let config = StreamingConfig::new();
	/// assert_eq!(config.buffer_size, 4096);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the buffer size
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::streaming::StreamingConfig;
	///
	/// let config = StreamingConfig::new().with_buffer_size(8192);
	/// assert_eq!(config.buffer_size, 8192);
	/// ```
	pub fn with_buffer_size(mut self, size: usize) -> Self {
		self.buffer_size = size;
		self
	}

	/// Sets the flush interval
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::streaming::StreamingConfig;
	/// use std::time::Duration;
	///
	/// let config = StreamingConfig::new()
	///     .with_flush_interval(Duration::from_millis(100));
	/// assert!(config.flush_interval.is_some());
	/// ```
	pub fn with_flush_interval(mut self, interval: Duration) -> Self {
		self.flush_interval = Some(interval);
		self
	}
}

/// Trait for streaming renderers
///
/// Streaming renderers return a stream of byte chunks instead of a single
/// buffer, allowing for efficient handling of large datasets.
#[async_trait]
pub trait StreamingRenderer: Send + Sync {
	/// Renders data as a stream of byte chunks
	///
	/// # Arguments
	///
	/// * `data` - The data to render
	/// * `context` - Optional rendering context
	///
	/// # Returns
	///
	/// A pinned boxed stream of byte results
	async fn render_stream(
		&self,
		data: &Value,
		context: Option<&RendererContext>,
	) -> RenderResult<Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>>;

	/// Returns the format identifier (e.g., "json", "csv")
	fn format(&self) -> Option<&str>;

	/// Returns the content type for this renderer
	fn content_type(&self) -> &str;
}

/// Streaming JSON renderer
///
/// Renders JSON arrays by streaming each element incrementally.
/// For non-array values, behaves like a regular JSON renderer.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::streaming::{StreamingJSONRenderer, StreamingRenderer};
/// use serde_json::json;
/// use futures::StreamExt;
///
/// # #[tokio::main]
/// # async fn main() {
/// let renderer = StreamingJSONRenderer::new();
/// let data = json!([{"id": 1}, {"id": 2}, {"id": 3}]);
///
/// let mut stream = renderer.render_stream(&data, None).await.unwrap();
///
/// // Collect all chunks
/// let mut chunks = Vec::new();
/// while let Some(chunk) = stream.next().await {
///     chunks.push(chunk.unwrap());
/// }
///
/// // Verify the streamed JSON is valid
/// let result = chunks.concat();
/// let parsed: serde_json::Value = serde_json::from_slice(&result).unwrap();
/// assert_eq!(parsed.as_array().unwrap().len(), 3);
/// # }
/// ```
pub struct StreamingJSONRenderer {
	#[allow(dead_code)]
	config: StreamingConfig,
}

impl StreamingJSONRenderer {
	/// Creates a new StreamingJSONRenderer with default configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::streaming::StreamingJSONRenderer;
	///
	/// let renderer = StreamingJSONRenderer::new();
	/// ```
	pub fn new() -> Self {
		Self {
			config: StreamingConfig::default(),
		}
	}

	/// Creates a new StreamingJSONRenderer with custom configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::streaming::{StreamingJSONRenderer, StreamingConfig};
	///
	/// let config = StreamingConfig::new().with_buffer_size(8192);
	/// let renderer = StreamingJSONRenderer::with_config(config);
	/// ```
	pub fn with_config(config: StreamingConfig) -> Self {
		Self { config }
	}
}

impl Default for StreamingJSONRenderer {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl StreamingRenderer for StreamingJSONRenderer {
	async fn render_stream(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>> {
		match data {
			Value::Array(items) => {
				// Stream array elements
				let items = items.clone();
				let stream = stream::iter(vec![Ok(Bytes::from("["))])
					.chain(stream::iter(items.into_iter().enumerate().flat_map(
						|(i, item)| {
							let separator = if i > 0 { "," } else { "" };
							let json_str = serde_json::to_string(&item)
								.map(|s| format!("{}{}", separator, s))
								.unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e));

							vec![Ok(Bytes::from(json_str))]
						},
					)))
					.chain(stream::iter(vec![Ok(Bytes::from("]"))]));

				Ok(Box::pin(stream))
			}
			_ => {
				// For non-array values, serialize directly
				let json_str = serde_json::to_string(data)
					.map_err(|e| Error::Http(format!("JSON serialization error: {}", e)))?;
				let stream = stream::iter(vec![Ok(Bytes::from(json_str))]);
				Ok(Box::pin(stream))
			}
		}
	}

	fn format(&self) -> Option<&str> {
		Some("json")
	}

	fn content_type(&self) -> &str {
		"application/json"
	}
}

/// Streaming CSV renderer
///
/// Renders data as CSV by streaming rows incrementally.
/// Expects data to be a JSON array of objects.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::streaming::{StreamingCSVRenderer, StreamingRenderer};
/// use serde_json::json;
/// use futures::StreamExt;
///
/// # #[tokio::main]
/// # async fn main() {
/// let renderer = StreamingCSVRenderer::new();
/// let data = json!([
///     {"name": "Alice", "age": 30},
///     {"name": "Bob", "age": 25}
/// ]);
///
/// let mut stream = renderer.render_stream(&data, None).await.unwrap();
///
/// // Collect all chunks
/// let mut chunks = Vec::new();
/// while let Some(chunk) = stream.next().await {
///     chunks.push(chunk.unwrap());
/// }
///
/// let result = String::from_utf8(chunks.concat()).unwrap();
/// // Header order depends on HashMap iteration order
/// assert!(result.contains("name") && result.contains("age"));
/// assert!(result.contains("Alice") && result.contains("30"));
/// assert!(result.contains("Bob") && result.contains("25"));
/// # }
/// ```
pub struct StreamingCSVRenderer {
	#[allow(dead_code)]
	config: StreamingConfig,
}

impl StreamingCSVRenderer {
	/// Creates a new StreamingCSVRenderer with default configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::streaming::StreamingCSVRenderer;
	///
	/// let renderer = StreamingCSVRenderer::new();
	/// ```
	pub fn new() -> Self {
		Self {
			config: StreamingConfig::default(),
		}
	}

	/// Creates a new StreamingCSVRenderer with custom configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::streaming::{StreamingCSVRenderer, StreamingConfig};
	///
	/// let config = StreamingConfig::new().with_buffer_size(8192);
	/// let renderer = StreamingCSVRenderer::with_config(config);
	/// ```
	pub fn with_config(config: StreamingConfig) -> Self {
		Self { config }
	}

	/// Converts a JSON object to a CSV row
	fn object_to_csv_row(obj: &serde_json::Map<String, Value>, headers: &[String]) -> String {
		headers
			.iter()
			.map(|key| {
				obj.get(key)
					.map(|v| match v {
						Value::String(s) => format!("\"{}\"", s.replace('"', "\"\"")),
						Value::Number(n) => n.to_string(),
						Value::Bool(b) => b.to_string(),
						Value::Null => String::new(),
						_ => format!("\"{}\"", v),
					})
					.unwrap_or_default()
			})
			.collect::<Vec<_>>()
			.join(",")
	}
}

impl Default for StreamingCSVRenderer {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl StreamingRenderer for StreamingCSVRenderer {
	async fn render_stream(
		&self,
		data: &Value,
		_context: Option<&RendererContext>,
	) -> RenderResult<Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>> {
		match data {
			Value::Array(items) if !items.is_empty() => {
				// Extract headers from first object
				let headers: Vec<String> = if let Some(Value::Object(first_obj)) = items.first() {
					first_obj.keys().cloned().collect()
				} else {
					return Err(Error::Http(
						"CSV renderer requires array of objects".to_string(),
					));
				};

				// Create header row
				let header_row = format!("{}\n", headers.join(","));
				let items = items.clone();

				// Create stream starting with headers
				let stream = stream::iter(vec![Ok(Bytes::from(header_row))]).chain(stream::iter(
					items.into_iter().map(move |item| {
						if let Value::Object(obj) = item {
							let row = Self::object_to_csv_row(&obj, &headers);
							Ok(Bytes::from(format!("{}\n", row)))
						} else {
							Err(Error::Http(
								"CSV renderer requires array of objects".to_string(),
							))
						}
					}),
				));

				Ok(Box::pin(stream))
			}
			Value::Array(_) => {
				// Empty array
				Ok(Box::pin(stream::iter(vec![Ok(Bytes::from(""))])))
			}
			_ => Err(Error::Http(
				"CSV renderer requires array of objects".to_string(),
			)),
		}
	}

	fn format(&self) -> Option<&str> {
		Some("csv")
	}

	fn content_type(&self) -> &str {
		"text/csv"
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use futures::StreamExt;
	use serde_json::json;

	#[tokio::test]
	async fn test_streaming_json_array() {
		let renderer = StreamingJSONRenderer::new();
		let data = json!([{"id": 1}, {"id": 2}, {"id": 3}]);

		let mut stream = renderer.render_stream(&data, None).await.unwrap();

		let mut chunks = Vec::new();
		while let Some(chunk) = stream.next().await {
			chunks.push(chunk.unwrap());
		}

		let result = chunks.concat();
		let parsed: Value = serde_json::from_slice(&result).unwrap();
		assert_eq!(parsed.as_array().unwrap().len(), 3);
	}

	#[tokio::test]
	async fn test_streaming_json_single_object() {
		let renderer = StreamingJSONRenderer::new();
		let data = json!({"id": 1, "name": "test"});

		let mut stream = renderer.render_stream(&data, None).await.unwrap();

		let mut chunks = Vec::new();
		while let Some(chunk) = stream.next().await {
			chunks.push(chunk.unwrap());
		}

		let result = chunks.concat();
		let parsed: Value = serde_json::from_slice(&result).unwrap();
		assert_eq!(parsed["id"], 1);
		assert_eq!(parsed["name"], "test");
	}

	#[tokio::test]
	async fn test_streaming_json_empty_array() {
		let renderer = StreamingJSONRenderer::new();
		let data = json!([]);

		let mut stream = renderer.render_stream(&data, None).await.unwrap();

		let mut chunks = Vec::new();
		while let Some(chunk) = stream.next().await {
			chunks.push(chunk.unwrap());
		}

		let result = chunks.concat();
		let parsed: Value = serde_json::from_slice(&result).unwrap();
		assert_eq!(parsed.as_array().unwrap().len(), 0);
	}

	#[tokio::test]
	async fn test_streaming_csv_basic() {
		let renderer = StreamingCSVRenderer::new();
		let data = json!([
			{"name": "Alice", "age": 30},
			{"name": "Bob", "age": 25}
		]);

		let mut stream = renderer.render_stream(&data, None).await.unwrap();

		let mut chunks = Vec::new();
		while let Some(chunk) = stream.next().await {
			chunks.push(chunk.unwrap());
		}

		let result = String::from_utf8(chunks.concat()).unwrap();

		// Check headers
		assert!(result.contains("name,age") || result.contains("age,name"));

		// Check data rows
		assert!(result.contains("Alice") && result.contains("30"));
		assert!(result.contains("Bob") && result.contains("25"));
	}

	#[tokio::test]
	async fn test_streaming_csv_with_quotes() {
		let renderer = StreamingCSVRenderer::new();
		let data = json!([
			{"name": "Alice, Jr.", "title": "Developer"}
		]);

		let mut stream = renderer.render_stream(&data, None).await.unwrap();

		let mut chunks = Vec::new();
		while let Some(chunk) = stream.next().await {
			chunks.push(chunk.unwrap());
		}

		let result = String::from_utf8(chunks.concat()).unwrap();

		// String with comma should be quoted
		assert!(result.contains("\"Alice, Jr.\""));
	}

	#[tokio::test]
	async fn test_streaming_csv_empty_array() {
		let renderer = StreamingCSVRenderer::new();
		let data = json!([]);

		let mut stream = renderer.render_stream(&data, None).await.unwrap();

		let mut chunks = Vec::new();
		while let Some(chunk) = stream.next().await {
			chunks.push(chunk.unwrap());
		}

		let result = chunks.concat();
		assert_eq!(result.len(), 0);
	}

	#[tokio::test]
	async fn test_streaming_csv_invalid_data() {
		let renderer = StreamingCSVRenderer::new();
		let data = json!({"not": "an array"});

		let result = renderer.render_stream(&data, None).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_streaming_config() {
		let config = StreamingConfig::new()
			.with_buffer_size(8192)
			.with_flush_interval(Duration::from_millis(100));

		assert_eq!(config.buffer_size, 8192);
		assert_eq!(config.flush_interval, Some(Duration::from_millis(100)));
	}

	#[tokio::test]
	async fn test_streaming_json_format() {
		let renderer = StreamingJSONRenderer::new();
		assert_eq!(renderer.format(), Some("json"));
		assert_eq!(renderer.content_type(), "application/json");
	}

	#[tokio::test]
	async fn test_streaming_csv_format() {
		let renderer = StreamingCSVRenderer::new();
		assert_eq!(renderer.format(), Some("csv"));
		assert_eq!(renderer.content_type(), "text/csv");
	}
}
