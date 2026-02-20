//! WebSocket message compression support
//!
//! This module provides compression and decompression for WebSocket messages.
//! It supports three compression algorithms: gzip, deflate, and brotli.
//!
//! ## Usage Example
//!
//! ```
//! use reinhardt_websockets::compression::{CompressionCodec, compress_message, decompress_message};
//! use reinhardt_websockets::Message;
//!
//! # tokio_test::block_on(async {
//! let message = Message::text("Hello, World!".to_string());
//!
//! // Gzip compression
//! let compressed = compress_message(&message, CompressionCodec::Gzip).unwrap();
//! let decompressed = decompress_message(&compressed, CompressionCodec::Gzip).unwrap();
//!
//! // Decompressed message is always returned as Binary
//! match decompressed {
//!     Message::Binary { data } => {
//!         assert_eq!(String::from_utf8(data).unwrap(), "Hello, World!");
//!     },
//!     _ => panic!("Expected binary message"),
//! }
//! # });
//! ```

use crate::{Message, WebSocketError, WebSocketResult};

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompressionCodec {
	/// Gzip compression
	Gzip,
	/// Deflate compression
	Deflate,
	/// Brotli compression
	Brotli,
}

impl CompressionCodec {
	/// Converts a compression codec to a string.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_websockets::compression::CompressionCodec;
	///
	/// assert_eq!(CompressionCodec::Gzip.as_str(), "gzip");
	/// assert_eq!(CompressionCodec::Deflate.as_str(), "deflate");
	/// assert_eq!(CompressionCodec::Brotli.as_str(), "br");
	/// ```
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Gzip => "gzip",
			Self::Deflate => "deflate",
			Self::Brotli => "br",
		}
	}
}

impl std::str::FromStr for CompressionCodec {
	type Err = WebSocketError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"gzip" => Ok(Self::Gzip),
			"deflate" => Ok(Self::Deflate),
			"br" | "brotli" => Ok(Self::Brotli),
			_ => Err(WebSocketError::Protocol(
				"unsupported compression codec".to_string(),
			)),
		}
	}
}

/// Compresses a message.
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::compression::{CompressionCodec, compress_message};
/// use reinhardt_websockets::Message;
///
/// let message = Message::text("Hello, World!".to_string());
/// let compressed = compress_message(&message, CompressionCodec::Gzip).unwrap();
///
/// match compressed {
///     Message::Binary { data } => {
///         assert!(data.len() > 0);
///         // Compressed data may be smaller than the original text
///     },
///     _ => panic!("Expected binary message"),
/// }
/// ```
#[cfg(feature = "compression")]
pub fn compress_message(message: &Message, codec: CompressionCodec) -> WebSocketResult<Message> {
	let data = match message {
		Message::Text { data } => data.as_bytes(),
		Message::Binary { data } => data.as_slice(),
		_ => {
			return Err(WebSocketError::Protocol(
				"Cannot compress non-data messages".to_string(),
			));
		}
	};

	let compressed = match codec {
		CompressionCodec::Gzip => compress_gzip(data)?,
		CompressionCodec::Deflate => compress_deflate(data)?,
		CompressionCodec::Brotli => compress_brotli(data)?,
	};

	Ok(Message::Binary { data: compressed })
}

/// Decompresses a compressed message.
///
/// Always returns a binary message, as the original message type
/// information is not preserved during compression.
///
/// # Examples
///
/// ```
/// use reinhardt_websockets::compression::{CompressionCodec, compress_message, decompress_message};
/// use reinhardt_websockets::Message;
///
/// let original = Message::text("Hello, World!".to_string());
/// let compressed = compress_message(&original, CompressionCodec::Gzip).unwrap();
/// let decompressed = decompress_message(&compressed, CompressionCodec::Gzip).unwrap();
///
/// // Decompressed message is always returned as Binary
/// match decompressed {
///     Message::Binary { data } => {
///         assert_eq!(String::from_utf8(data).unwrap(), "Hello, World!");
///     },
///     _ => panic!("Expected binary message"),
/// }
/// ```
#[cfg(feature = "compression")]
pub fn decompress_message(message: &Message, codec: CompressionCodec) -> WebSocketResult<Message> {
	let data = match message {
		Message::Binary { data } => data.as_slice(),
		_ => {
			return Err(WebSocketError::Protocol(
				"Can only decompress binary messages".to_string(),
			));
		}
	};

	let decompressed = match codec {
		CompressionCodec::Gzip => decompress_gzip(data)?,
		CompressionCodec::Deflate => decompress_deflate(data)?,
		CompressionCodec::Brotli => decompress_brotli(data)?,
	};

	// Return as binary message (original type information is not preserved)
	Ok(Message::Binary { data: decompressed })
}

#[cfg(feature = "compression")]
fn compress_gzip(data: &[u8]) -> WebSocketResult<Vec<u8>> {
	use flate2::Compression;
	use flate2::write::GzEncoder;
	use std::io::Write;

	let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
	encoder
		.write_all(data)
		.map_err(|_| WebSocketError::Protocol("compression failed".to_string()))?;
	encoder
		.finish()
		.map_err(|_| WebSocketError::Protocol("compression failed".to_string()))
}

#[cfg(feature = "compression")]
fn decompress_gzip(data: &[u8]) -> WebSocketResult<Vec<u8>> {
	use flate2::read::GzDecoder;
	use std::io::Read;

	let mut decoder = GzDecoder::new(data);
	let mut decompressed = Vec::new();
	decoder
		.read_to_end(&mut decompressed)
		.map_err(|_| WebSocketError::Protocol("decompression failed".to_string()))?;
	Ok(decompressed)
}

#[cfg(feature = "compression")]
fn compress_deflate(data: &[u8]) -> WebSocketResult<Vec<u8>> {
	use flate2::Compression;
	use flate2::write::DeflateEncoder;
	use std::io::Write;

	let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
	encoder
		.write_all(data)
		.map_err(|_| WebSocketError::Protocol("compression failed".to_string()))?;
	encoder
		.finish()
		.map_err(|_| WebSocketError::Protocol("compression failed".to_string()))
}

#[cfg(feature = "compression")]
fn decompress_deflate(data: &[u8]) -> WebSocketResult<Vec<u8>> {
	use flate2::read::DeflateDecoder;
	use std::io::Read;

	let mut decoder = DeflateDecoder::new(data);
	let mut decompressed = Vec::new();
	decoder
		.read_to_end(&mut decompressed)
		.map_err(|_| WebSocketError::Protocol("decompression failed".to_string()))?;
	Ok(decompressed)
}

#[cfg(feature = "compression")]
fn compress_brotli(data: &[u8]) -> WebSocketResult<Vec<u8>> {
	use std::io::Write;

	let mut compressed = Vec::new();
	let mut compressor = brotli::CompressorWriter::new(&mut compressed, 4096, 11, 22);
	compressor
		.write_all(data)
		.map_err(|_| WebSocketError::Protocol("compression failed".to_string()))?;
	compressor
		.flush()
		.map_err(|_| WebSocketError::Protocol("compression failed".to_string()))?;
	drop(compressor);
	Ok(compressed)
}

#[cfg(feature = "compression")]
fn decompress_brotli(data: &[u8]) -> WebSocketResult<Vec<u8>> {
	use std::io::Read;

	let mut decompressor = brotli::Decompressor::new(data, 4096);
	let mut decompressed = Vec::new();
	decompressor
		.read_to_end(&mut decompressed)
		.map_err(|_| WebSocketError::Protocol("decompression failed".to_string()))?;
	Ok(decompressed)
}

/// Feature-gated no-op: Returns error when compression is disabled.
///
/// To enable compression functionality, add the `compression` feature:
/// ```toml
/// reinhardt-websockets = { version = "...", features = ["compression"] }
/// ```
#[cfg(not(feature = "compression"))]
pub fn compress_message(_message: &Message, _codec: CompressionCodec) -> WebSocketResult<Message> {
	Err(WebSocketError::Protocol(
		"compression not available".to_string(),
	))
}

/// Feature-gated no-op: Returns error when compression is disabled.
///
/// To enable compression functionality, add the `compression` feature:
/// ```toml
/// reinhardt-websockets = { version = "...", features = ["compression"] }
/// ```
#[cfg(not(feature = "compression"))]
pub fn decompress_message(
	_message: &Message,
	_codec: CompressionCodec,
) -> WebSocketResult<Message> {
	Err(WebSocketError::Protocol(
		"compression not available".to_string(),
	))
}

#[cfg(all(test, feature = "compression"))]
mod tests {
	use super::*;
	use std::str::FromStr;

	#[test]
	fn test_compression_codec_from_str() {
		assert_eq!(
			CompressionCodec::from_str("gzip").unwrap(),
			CompressionCodec::Gzip
		);
		assert_eq!(
			CompressionCodec::from_str("deflate").unwrap(),
			CompressionCodec::Deflate
		);
		assert_eq!(
			CompressionCodec::from_str("br").unwrap(),
			CompressionCodec::Brotli
		);
		assert_eq!(
			CompressionCodec::from_str("brotli").unwrap(),
			CompressionCodec::Brotli
		);
		assert!(CompressionCodec::from_str("unknown").is_err());
	}

	#[test]
	fn test_compression_codec_as_str() {
		assert_eq!(CompressionCodec::Gzip.as_str(), "gzip");
		assert_eq!(CompressionCodec::Deflate.as_str(), "deflate");
		assert_eq!(CompressionCodec::Brotli.as_str(), "br");
	}

	#[test]
	fn test_gzip_compression_text_message() {
		let text = "Hello, World!".to_string();
		let message = Message::text(text.clone());
		let compressed = compress_message(&message, CompressionCodec::Gzip).unwrap();
		let decompressed = decompress_message(&compressed, CompressionCodec::Gzip).unwrap();

		// After decompression, it becomes a binary message
		match decompressed {
			Message::Binary { data } => {
				assert_eq!(String::from_utf8(data).unwrap(), text);
			}
			_ => panic!("Expected binary message"),
		}
	}

	#[test]
	fn test_deflate_compression_text_message() {
		let text = "Hello, World!".to_string();
		let message = Message::text(text.clone());
		let compressed = compress_message(&message, CompressionCodec::Deflate).unwrap();
		let decompressed = decompress_message(&compressed, CompressionCodec::Deflate).unwrap();

		match decompressed {
			Message::Binary { data } => {
				assert_eq!(String::from_utf8(data).unwrap(), text);
			}
			_ => panic!("Expected binary message"),
		}
	}

	#[test]
	fn test_brotli_compression_text_message() {
		let text = "Hello, World!".to_string();
		let message = Message::text(text.clone());
		let compressed = compress_message(&message, CompressionCodec::Brotli).unwrap();
		let decompressed = decompress_message(&compressed, CompressionCodec::Brotli).unwrap();

		match decompressed {
			Message::Binary { data } => {
				assert_eq!(String::from_utf8(data).unwrap(), text);
			}
			_ => panic!("Expected binary message"),
		}
	}

	#[test]
	fn test_gzip_compression_binary_message() {
		let data = vec![1, 2, 3, 4, 5];
		let message = Message::binary(data.clone());
		let compressed = compress_message(&message, CompressionCodec::Gzip).unwrap();
		let decompressed = decompress_message(&compressed, CompressionCodec::Gzip).unwrap();

		match decompressed {
			Message::Binary {
				data: decompressed_data,
			} => {
				assert_eq!(decompressed_data, data);
			}
			_ => panic!("Expected binary message"),
		}
	}

	#[test]
	fn test_compression_reduces_size() {
		// Long repeated text has high compression efficiency
		let long_text = "Hello, World! ".repeat(100);
		let message = Message::text(long_text.clone());
		let compressed = compress_message(&message, CompressionCodec::Gzip).unwrap();

		match compressed {
			Message::Binary { data } => {
				// Compressed data should be smaller than the original text
				assert!(data.len() < long_text.len());
			}
			_ => panic!("Expected binary message"),
		}
	}

	#[test]
	fn test_compress_non_data_message_fails() {
		let message = Message::Ping;
		let result = compress_message(&message, CompressionCodec::Gzip);
		assert!(result.is_err());
	}

	#[test]
	fn test_decompress_text_message_fails() {
		let message = Message::text("Not compressed".to_string());
		let result = decompress_message(&message, CompressionCodec::Gzip);
		assert!(result.is_err());
	}
}
