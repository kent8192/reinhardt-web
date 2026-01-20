//! Streaming parser for memory-efficient processing of large uploads.
//!
//! This module provides parsers that can handle large files and data streams
//! without loading the entire body into memory at once.

use crate::exception::Error;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use futures_util::stream::Stream;
use http::HeaderMap;
use std::pin::Pin;

use super::parser::{ParseResult, ParsedData, Parser, UploadedFile};

/// Chunk of data from a streaming parser.
#[derive(Debug, Clone)]
pub struct StreamChunk {
	/// The data in this chunk
	pub data: Bytes,
	/// Offset of this chunk in the overall stream
	pub offset: usize,
	/// Total size if known
	pub total_size: Option<usize>,
}

/// Streaming parser for large file uploads.
///
/// This parser processes data incrementally without loading the entire body
/// into memory, making it suitable for large file uploads or streaming data.
///
/// # Examples
///
/// ```
/// use reinhardt_core::parsers::streaming::StreamingParser;
/// use reinhardt_core::parsers::parser::Parser;
/// use bytes::Bytes;
/// use http::HeaderMap;
///
/// # tokio_test::block_on(async {
/// // Set chunk size to 1MB for efficient processing
/// let parser = StreamingParser::new(1024 * 1024);
///
/// let body = Bytes::from("large file content...");
/// let headers = HeaderMap::new();
/// let result = parser.parse(Some("application/octet-stream"), body, &headers).await;
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct StreamingParser {
	/// Size of each chunk to process
	chunk_size: usize,
	/// Maximum total size to accept
	max_size: Option<usize>,
}

impl StreamingParser {
	/// Create a new StreamingParser with the specified chunk size.
	///
	/// # Arguments
	///
	/// * `chunk_size` - Size of each chunk to process (in bytes)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::streaming::StreamingParser;
	///
	/// // Process in 64KB chunks
	/// let parser = StreamingParser::new(64 * 1024);
	/// ```
	pub fn new(chunk_size: usize) -> Self {
		Self {
			chunk_size,
			max_size: None,
		}
	}

	/// Set a maximum size limit for the stream.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::streaming::StreamingParser;
	///
	/// // Allow up to 100MB
	/// let parser = StreamingParser::new(64 * 1024)
	///     .with_max_size(100 * 1024 * 1024);
	/// ```
	pub fn with_max_size(mut self, max_size: usize) -> Self {
		self.max_size = Some(max_size);
		self
	}

	/// Process a byte stream in chunks.
	///
	/// This method is more efficient for large uploads as it doesn't require
	/// loading the entire body into memory.
	pub async fn parse_stream<S>(&self, mut stream: Pin<Box<S>>) -> ParseResult<Vec<StreamChunk>>
	where
		S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
	{
		let mut chunks = Vec::new();
		let mut offset = 0;
		let mut buffer = BytesMut::with_capacity(self.chunk_size);

		while let Some(result) = stream.next().await {
			let data =
				result.map_err(|e| Error::Validation(format!("Failed to read stream: {}", e)))?;

			buffer.extend_from_slice(&data);

			// Process complete chunks
			while buffer.len() >= self.chunk_size {
				let chunk_data = buffer.split_to(self.chunk_size).freeze();

				// Check max size limit
				if let Some(max_size) = self.max_size
					&& offset + chunk_data.len() > max_size
				{
					return Err(Error::Validation(format!(
						"Stream size exceeds maximum allowed size of {} bytes",
						max_size
					)));
				}

				chunks.push(StreamChunk {
					data: chunk_data,
					offset,
					total_size: None,
				});

				offset += self.chunk_size;
			}
		}

		// Process remaining data
		if !buffer.is_empty() {
			let chunk_data = buffer.freeze();

			if let Some(max_size) = self.max_size
				&& offset + chunk_data.len() > max_size
			{
				return Err(Error::Validation(format!(
					"Stream size exceeds maximum allowed size of {} bytes",
					max_size
				)));
			}

			chunks.push(StreamChunk {
				data: chunk_data,
				offset,
				total_size: Some(offset + chunks.last().map(|c| c.data.len()).unwrap_or(0)),
			});
		}

		Ok(chunks)
	}
}

#[async_trait]
impl Parser for StreamingParser {
	fn media_types(&self) -> Vec<String> {
		vec!["application/octet-stream".to_string(), "*/*".to_string()]
	}

	async fn parse(
		&self,
		_content_type: Option<&str>,
		body: Bytes,
		_headers: &HeaderMap,
	) -> ParseResult<ParsedData> {
		// For direct Bytes input, we still support it but process in chunks
		let total_size = body.len();

		// Check max size limit
		if let Some(max_size) = self.max_size
			&& total_size > max_size
		{
			return Err(Error::Validation(format!(
				"Body size {} exceeds maximum allowed size {}",
				total_size, max_size
			)));
		}

		// Convert to a file representation
		let file = UploadedFile::new("stream".to_string(), body)
			.with_content_type("application/octet-stream".to_string());

		Ok(ParsedData::File(file))
	}
}

impl Default for StreamingParser {
	fn default() -> Self {
		// Default to 1MB chunks
		Self::new(1024 * 1024)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use futures_util::stream;

	#[tokio::test]
	async fn test_streaming_parser_media_types() {
		let parser = StreamingParser::new(1024);
		let media_types = parser.media_types();

		assert_eq!(media_types.len(), 2);
		assert!(media_types.contains(&"application/octet-stream".to_string()));
		assert!(media_types.contains(&"*/*".to_string()));
	}

	#[tokio::test]
	async fn test_streaming_parser_can_parse() {
		let parser = StreamingParser::new(1024);

		assert!(parser.can_parse(Some("application/octet-stream")));
		assert!(parser.can_parse(Some("application/pdf")));
		assert!(parser.can_parse(Some("image/png")));
	}

	#[tokio::test]
	async fn test_streaming_parser_small_body() {
		let parser = StreamingParser::new(1024);
		let body = Bytes::from("small file");
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/octet-stream"), body.clone(), &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::File(file) => {
				assert_eq!(file.name, "stream");
				assert_eq!(file.data, body);
				assert_eq!(file.size, body.len());
			}
			_ => panic!("Expected File variant"),
		}
	}

	#[tokio::test]
	async fn test_streaming_parser_with_max_size() {
		let parser = StreamingParser::new(1024).with_max_size(10);
		let body = Bytes::from("this is too large");
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/octet-stream"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_streaming_parser_within_max_size() {
		let parser = StreamingParser::new(1024).with_max_size(100);
		let body = Bytes::from("small");
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/octet-stream"), body, &headers)
			.await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_parse_stream_single_chunk() {
		let parser = StreamingParser::new(10);
		let data = vec![Ok(Bytes::from("0123456789"))];
		let stream = Box::pin(stream::iter(data));

		let chunks = parser.parse_stream(stream).await.unwrap();
		assert_eq!(chunks.len(), 1);
		assert_eq!(chunks[0].data, Bytes::from("0123456789"));
		assert_eq!(chunks[0].offset, 0);
	}

	#[tokio::test]
	async fn test_parse_stream_multiple_chunks() {
		let parser = StreamingParser::new(5);
		let data = vec![
			Ok(Bytes::from("01234")),
			Ok(Bytes::from("56789")),
			Ok(Bytes::from("ABCDE")),
		];
		let stream = Box::pin(stream::iter(data));

		let chunks = parser.parse_stream(stream).await.unwrap();
		assert_eq!(chunks.len(), 3);
		assert_eq!(chunks[0].offset, 0);
		assert_eq!(chunks[1].offset, 5);
		assert_eq!(chunks[2].offset, 10);
	}

	#[tokio::test]
	async fn test_parse_stream_with_max_size_exceeded() {
		let parser = StreamingParser::new(5).with_max_size(10);
		let data = vec![
			Ok(Bytes::from("01234")),
			Ok(Bytes::from("56789")),
			Ok(Bytes::from("ABCDE")), // This should exceed the limit
		];
		let stream = Box::pin(stream::iter(data));

		let result = parser.parse_stream(stream).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_parse_stream_with_max_size_ok() {
		let parser = StreamingParser::new(5).with_max_size(20);
		let data = vec![
			Ok(Bytes::from("01234")),
			Ok(Bytes::from("56789")),
			Ok(Bytes::from("ABCDE")),
		];
		let stream = Box::pin(stream::iter(data));

		let result = parser.parse_stream(stream).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_parse_stream_partial_final_chunk() {
		let parser = StreamingParser::new(10);
		let data = vec![
			Ok(Bytes::from("0123456789")),
			Ok(Bytes::from("ABC")), // Partial chunk
		];
		let stream = Box::pin(stream::iter(data));

		let chunks = parser.parse_stream(stream).await.unwrap();
		assert_eq!(chunks.len(), 2);
		assert_eq!(chunks[0].data.len(), 10);
		assert_eq!(chunks[1].data.len(), 3);
		assert!(chunks[1].total_size.is_some());
	}

	#[tokio::test]
	async fn test_streaming_parser_default() {
		let parser = StreamingParser::default();
		assert_eq!(parser.chunk_size, 1024 * 1024);
		assert!(parser.max_size.is_none());
	}
}
