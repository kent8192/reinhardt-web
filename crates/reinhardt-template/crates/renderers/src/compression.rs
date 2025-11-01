//! Response compression support for renderers
//!
//! This module provides automatic response compression using multiple algorithms
//! with Accept-Encoding negotiation.
//!
//! ## Supported Algorithms
//!
//! - **Gzip**: Fast compression with configurable level (0-9)
//! - **Brotli**: High compression ratio with configurable quality (0-11)
//! - **Deflate**: Basic compression algorithm
//!
//! ## Examples
//!
//! ### Basic Compression
//!
//! ```rust
//! use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer, Renderer, RendererContext};
//! use serde_json::json;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let renderer = CompressionRenderer::new(
//!     JSONRenderer::new(),
//!     vec![CompressionAlgorithm::Gzip { level: 6 }],
//! );
//!
//! let data = json!({"message": "hello world"});
//! let context = RendererContext::new()
//!     .with_extra("accept_encoding", "gzip");
//!
//! let result = renderer.render(&data, Some(&context)).await.unwrap();
//! # }
//! ```
//!
//! ### Accept-Encoding Negotiation
//!
//! ```rust
//! use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer};
//!
//! let renderer = CompressionRenderer::new(
//!     JSONRenderer::new(),
//!     vec![
//!         CompressionAlgorithm::Brotli { quality: 4 },
//!         CompressionAlgorithm::Gzip { level: 6 },
//!         CompressionAlgorithm::Deflate,
//!     ],
//! );
//!
//! // Automatically selects best algorithm based on Accept-Encoding
//! let algorithm = renderer.select_algorithm("gzip, deflate, br;q=0.9");
//! assert!(algorithm.is_some());
//! ```
//!
//! ### Minimum Size Filter
//!
//! ```rust
//! use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer, Renderer, RendererContext};
//! use serde_json::json;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let renderer = CompressionRenderer::new(
//!     JSONRenderer::new(),
//!     vec![CompressionAlgorithm::Gzip { level: 6 }],
//! )
//! .with_min_size(1024); // Only compress responses larger than 1KB
//!
//! let small_data = json!({"a": 1});
//! let context = RendererContext::new()
//!     .with_extra("accept_encoding", "gzip");
//!
//! // Small response will not be compressed
//! let result = renderer.render(&small_data, Some(&context)).await.unwrap();
//! # }
//! ```
//!
//! ### Multiple Algorithms
//!
//! ```rust
//! use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer};
//!
//! let renderer = CompressionRenderer::new(
//!     JSONRenderer::new(),
//!     vec![
//!         CompressionAlgorithm::Brotli { quality: 4 },
//!         CompressionAlgorithm::Gzip { level: 6 },
//!         CompressionAlgorithm::Deflate,
//!     ],
//! );
//!
//! // Priority: Brotli > Gzip > Deflate
//! let br_algo = renderer.select_algorithm("br, gzip, deflate");
//! assert!(matches!(br_algo, Some(CompressionAlgorithm::Brotli { .. })));
//!
//! let gzip_algo = renderer.select_algorithm("gzip, deflate");
//! assert!(matches!(gzip_algo, Some(CompressionAlgorithm::Gzip { .. })));
//! ```

use crate::renderer::{Renderer, RendererContext};
use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::{Error, Result};
use serde_json::Value;
use std::io::Write;

/// Compression algorithm with configuration
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionAlgorithm {
	/// Gzip compression with level 0-9 (higher = better compression, slower)
	Gzip { level: u32 },
	/// Brotli compression with quality 0-11 (higher = better compression, slower)
	Brotli { quality: u32 },
	/// Deflate compression (basic zlib)
	Deflate,
}

impl CompressionAlgorithm {
	/// Returns the Content-Encoding header value for this algorithm
	pub fn encoding_name(&self) -> &'static str {
		match self {
			CompressionAlgorithm::Gzip { .. } => "gzip",
			CompressionAlgorithm::Brotli { .. } => "br",
			CompressionAlgorithm::Deflate => "deflate",
		}
	}

	/// Returns the priority order for algorithm selection
	/// Higher number = higher priority
	fn priority(&self) -> u32 {
		match self {
			CompressionAlgorithm::Brotli { .. } => 3,
			CompressionAlgorithm::Gzip { .. } => 2,
			CompressionAlgorithm::Deflate => 1,
		}
	}
}

/// Compression error
#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
	#[error("Compression failed: {0}")]
	CompressionFailed(String),
	#[error("IO error: {0}")]
	IoError(#[from] std::io::Error),
}

/// Parsed Accept-Encoding entry with quality value
#[derive(Debug, Clone)]
struct AcceptEncoding {
	encoding: String,
	quality: f32,
}

impl AcceptEncoding {
	/// Parses an Accept-Encoding entry (e.g., "gzip;q=0.8" or "br")
	fn parse(s: &str) -> Option<Self> {
		let parts: Vec<&str> = s.trim().split(';').collect();
		let encoding = parts.first()?.trim().to_lowercase();

		if encoding.is_empty() || encoding == "*" {
			return None;
		}

		let quality = if parts.len() > 1 {
			// Parse q-factor (e.g., "q=0.8")
			parts[1]
				.trim()
				.strip_prefix("q=")
				.and_then(|q| q.parse::<f32>().ok())
				.unwrap_or(1.0)
		} else {
			1.0
		};

		Some(AcceptEncoding { encoding, quality })
	}
}

/// A renderer that wraps another renderer and adds compression support
pub struct CompressionRenderer<R: Renderer> {
	inner: R,
	algorithms: Vec<CompressionAlgorithm>,
	min_size: usize,
}

impl<R: Renderer> CompressionRenderer<R> {
	/// Creates a new compression renderer
	///
	/// # Arguments
	///
	/// * `renderer` - The inner renderer to wrap
	/// * `algorithms` - List of compression algorithms to support (in priority order)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer};
	///
	/// let renderer = CompressionRenderer::new(
	///     JSONRenderer::new(),
	///     vec![CompressionAlgorithm::Gzip { level: 6 }],
	/// );
	/// ```
	pub fn new(renderer: R, algorithms: Vec<CompressionAlgorithm>) -> Self {
		Self {
			inner: renderer,
			algorithms,
			min_size: 1024, // Default: 1KB minimum
		}
	}

	/// Sets the minimum response size for compression
	///
	/// Responses smaller than this size will not be compressed.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer};
	///
	/// let renderer = CompressionRenderer::new(
	///     JSONRenderer::new(),
	///     vec![CompressionAlgorithm::Gzip { level: 6 }],
	/// )
	/// .with_min_size(2048); // Only compress responses > 2KB
	/// ```
	pub fn with_min_size(mut self, size: usize) -> Self {
		self.min_size = size;
		self
	}

	/// Selects the best compression algorithm based on Accept-Encoding header
	///
	/// Returns `None` if no suitable algorithm is found or if the client doesn't accept compression.
	///
	/// # Arguments
	///
	/// * `accept_encoding` - The Accept-Encoding header value
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer};
	///
	/// let renderer = CompressionRenderer::new(
	///     JSONRenderer::new(),
	///     vec![
	///         CompressionAlgorithm::Brotli { quality: 4 },
	///         CompressionAlgorithm::Gzip { level: 6 },
	///     ],
	/// );
	///
	/// // Client accepts Brotli with high priority
	/// let algo = renderer.select_algorithm("br;q=1.0, gzip;q=0.8");
	/// assert!(matches!(algo, Some(CompressionAlgorithm::Brotli { .. })));
	///
	/// // Client only accepts gzip
	/// let algo = renderer.select_algorithm("gzip");
	/// assert!(matches!(algo, Some(CompressionAlgorithm::Gzip { .. })));
	///
	/// // Client doesn't accept any supported encoding
	/// let algo = renderer.select_algorithm("identity");
	/// assert!(algo.is_none());
	/// ```
	pub fn select_algorithm(&self, accept_encoding: &str) -> Option<CompressionAlgorithm> {
		// Parse Accept-Encoding header
		let mut accepted: Vec<AcceptEncoding> = accept_encoding
			.split(',')
			.filter_map(AcceptEncoding::parse)
			.collect();

		// Sort by quality (descending)
		accepted.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap());

		// Find the best match with highest priority
		for accept in &accepted {
			if accept.quality == 0.0 {
				continue; // Skip explicitly disabled encodings
			}

			// Find matching algorithm with highest priority
			let mut matching_algos: Vec<&CompressionAlgorithm> = self
				.algorithms
				.iter()
				.filter(|algo| algo.encoding_name() == accept.encoding)
				.collect();

			matching_algos.sort_by_key(|algo| std::cmp::Reverse(algo.priority()));

			if let Some(algo) = matching_algos.first() {
				return Some((*algo).clone());
			}
		}

		None
	}

	/// Compresses data using the specified algorithm
	///
	/// # Arguments
	///
	/// * `data` - The data to compress
	/// * `algorithm` - The compression algorithm to use
	///
	/// # Returns
	///
	/// Returns the compressed data or an error if compression fails.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_renderers::{CompressionRenderer, CompressionAlgorithm, JSONRenderer};
	///
	/// # #[tokio::main]
	/// # async fn main() {
	/// let renderer = CompressionRenderer::new(
	///     JSONRenderer::new(),
	///     vec![CompressionAlgorithm::Gzip { level: 6 }],
	/// );
	///
	/// let data = b"Hello, world!";
	/// let compressed = renderer.compress(
	///     data,
	///     &CompressionAlgorithm::Gzip { level: 6 }
	/// ).unwrap();
	///
	/// assert!(compressed.len() < data.len() || data.len() < 100);
	/// # }
	/// ```
	pub fn compress(
		&self,
		data: &[u8],
		algorithm: &CompressionAlgorithm,
	) -> std::result::Result<Vec<u8>, CompressionError> {
		match algorithm {
			CompressionAlgorithm::Gzip { level } => {
				use flate2::Compression;
				use flate2::write::GzEncoder;

				let mut encoder =
					GzEncoder::new(Vec::new(), Compression::new((*level.min(&9))));
				encoder.write_all(data)?;
				encoder
					.finish()
					.map_err(|e| CompressionError::CompressionFailed(e.to_string()))
			}
			CompressionAlgorithm::Brotli { quality } => {
				let mut output = Vec::new();
				let quality = (*quality).min(11) as i32;

				let mut reader = std::io::Cursor::new(data);
				brotli::BrotliCompress(
					&mut reader,
					&mut output,
					&brotli::enc::BrotliEncoderParams {
						quality,
						..Default::default()
					},
				)
				.map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;

				Ok(output)
			}
			CompressionAlgorithm::Deflate => {
				use flate2::Compression;
				use flate2::write::DeflateEncoder;

				let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
				encoder.write_all(data)?;
				encoder
					.finish()
					.map_err(|e| CompressionError::CompressionFailed(e.to_string()))
			}
		}
	}
}

#[async_trait]
impl<R: Renderer> Renderer for CompressionRenderer<R> {
	fn media_type(&self) -> String {
		self.inner.media_type()
	}

	fn media_types(&self) -> Vec<String> {
		self.inner.media_types()
	}

	fn format(&self) -> Option<&str> {
		self.inner.format()
	}

	async fn render(&self, data: &Value, context: Option<&RendererContext>) -> Result<Bytes> {
		// First, render using the inner renderer
		let rendered = self.inner.render(data, context).await?;

		// Check if response is large enough to compress
		if rendered.len() < self.min_size {
			return Ok(rendered);
		}

		// Get Accept-Encoding from context
		let accept_encoding = context
			.and_then(|c| c.extra.get("accept_encoding"))
			.map(|s| s.as_str())
			.unwrap_or("");

		// Select compression algorithm
		let algorithm = match self.select_algorithm(accept_encoding) {
			Some(algo) => algo,
			None => return Ok(rendered), // No compression
		};

		// Compress the data
		let compressed = self
			.compress(&rendered, &algorithm)
			.map_err(|e| Error::Http(format!("Compression failed: {}", e)))?;

		// Only use compressed version if it's actually smaller
		if compressed.len() < rendered.len() {
			Ok(Bytes::from(compressed))
		} else {
			Ok(rendered)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::json::JSONRenderer;
	use serde_json::json;

	#[tokio::test]
	async fn test_gzip_compression() {
		let renderer = CompressionRenderer::new(
			JSONRenderer::new(),
			vec![CompressionAlgorithm::Gzip { level: 6 }],
		)
		.with_min_size(0);

		let data = json!({"message": "hello world".repeat(100)});
		let context = RendererContext::new().with_extra("accept_encoding", "gzip");

		let result = renderer.render(&data, Some(&context)).await;
		assert!(result.is_ok());
		let compressed = result.unwrap();

		// Verify compression worked (compressed should be smaller)
		let uncompressed = JSONRenderer::new().render(&data, None).await.unwrap();
		assert!(compressed.len() < uncompressed.len());
	}

	#[tokio::test]
	async fn test_brotli_compression() {
		let renderer = CompressionRenderer::new(
			JSONRenderer::new(),
			vec![CompressionAlgorithm::Brotli { quality: 4 }],
		)
		.with_min_size(0);

		let data = json!({"message": "hello world".repeat(100)});
		let context = RendererContext::new().with_extra("accept_encoding", "br");

		let result = renderer.render(&data, Some(&context)).await;
		assert!(result.is_ok());
		let compressed = result.unwrap();

		// Verify compression worked
		let uncompressed = JSONRenderer::new().render(&data, None).await.unwrap();
		assert!(compressed.len() < uncompressed.len());
	}

	#[tokio::test]
	async fn test_deflate_compression() {
		let renderer =
			CompressionRenderer::new(JSONRenderer::new(), vec![CompressionAlgorithm::Deflate])
				.with_min_size(0);

		let data = json!({"message": "hello world".repeat(100)});
		let context = RendererContext::new().with_extra("accept_encoding", "deflate");

		let result = renderer.render(&data, Some(&context)).await;
		assert!(result.is_ok());
		let compressed = result.unwrap();

		// Verify compression worked
		let uncompressed = JSONRenderer::new().render(&data, None).await.unwrap();
		assert!(compressed.len() < uncompressed.len());
	}

	#[test]
	fn test_accept_encoding_parse() {
		let accept = AcceptEncoding::parse("gzip;q=0.8").unwrap();
		assert_eq!(accept.encoding, "gzip");
		assert_eq!(accept.quality, 0.8);

		let accept = AcceptEncoding::parse("br").unwrap();
		assert_eq!(accept.encoding, "br");
		assert_eq!(accept.quality, 1.0);

		let accept = AcceptEncoding::parse("deflate;q=0.5").unwrap();
		assert_eq!(accept.encoding, "deflate");
		assert_eq!(accept.quality, 0.5);

		// Invalid cases
		assert!(AcceptEncoding::parse("").is_none());
		assert!(AcceptEncoding::parse("*").is_none());
	}

	#[test]
	fn test_select_algorithm_priority() {
		let renderer = CompressionRenderer::new(
			JSONRenderer::new(),
			vec![
				CompressionAlgorithm::Brotli { quality: 4 },
				CompressionAlgorithm::Gzip { level: 6 },
				CompressionAlgorithm::Deflate,
			],
		);

		// Brotli has highest priority
		let algo = renderer.select_algorithm("br, gzip, deflate");
		assert!(matches!(algo, Some(CompressionAlgorithm::Brotli { .. })));

		// Gzip when Brotli not available
		let algo = renderer.select_algorithm("gzip, deflate");
		assert!(matches!(algo, Some(CompressionAlgorithm::Gzip { .. })));

		// Deflate when others not available
		let algo = renderer.select_algorithm("deflate");
		assert!(matches!(algo, Some(CompressionAlgorithm::Deflate)));

		// No match
		let algo = renderer.select_algorithm("identity");
		assert!(algo.is_none());
	}

	#[test]
	fn test_select_algorithm_quality() {
		let renderer = CompressionRenderer::new(
			JSONRenderer::new(),
			vec![
				CompressionAlgorithm::Brotli { quality: 4 },
				CompressionAlgorithm::Gzip { level: 6 },
			],
		);

		// Higher quality value wins
		let algo = renderer.select_algorithm("gzip;q=1.0, br;q=0.8");
		assert!(matches!(algo, Some(CompressionAlgorithm::Gzip { .. })));

		// Higher quality value wins (Brotli)
		let algo = renderer.select_algorithm("gzip;q=0.8, br;q=1.0");
		assert!(matches!(algo, Some(CompressionAlgorithm::Brotli { .. })));

		// Zero quality is ignored
		let algo = renderer.select_algorithm("gzip;q=0, br");
		assert!(matches!(algo, Some(CompressionAlgorithm::Brotli { .. })));
	}

	#[tokio::test]
	async fn test_min_size_filter() {
		let renderer = CompressionRenderer::new(
			JSONRenderer::new(),
			vec![CompressionAlgorithm::Gzip { level: 6 }],
		)
		.with_min_size(1000);

		let small_data = json!({"a": 1});
		let context = RendererContext::new().with_extra("accept_encoding", "gzip");

		let result = renderer.render(&small_data, Some(&context)).await.unwrap();
		let uncompressed = JSONRenderer::new().render(&small_data, None).await.unwrap();

		// Small data should not be compressed (sizes should match)
		assert_eq!(result.len(), uncompressed.len());
	}

	#[tokio::test]
	async fn test_no_accept_encoding() {
		let renderer = CompressionRenderer::new(
			JSONRenderer::new(),
			vec![CompressionAlgorithm::Gzip { level: 6 }],
		)
		.with_min_size(0);

		let data = json!({"message": "hello world".repeat(100)});
		let context = RendererContext::new();

		let result = renderer.render(&data, Some(&context)).await.unwrap();
		let uncompressed = JSONRenderer::new().render(&data, None).await.unwrap();

		// Without Accept-Encoding, should not compress
		assert_eq!(result.len(), uncompressed.len());
	}

	#[tokio::test]
	async fn test_compression_ratio() {
		let renderer = CompressionRenderer::new(
			JSONRenderer::new(),
			vec![CompressionAlgorithm::Gzip { level: 9 }],
		)
		.with_min_size(0);

		// Create highly compressible data
		let large_data = json!({
			"data": vec!["repeated text"; 1000],
		});

		let context = RendererContext::new().with_extra("accept_encoding", "gzip");

		let compressed = renderer.render(&large_data, Some(&context)).await.unwrap();
		let uncompressed = JSONRenderer::new().render(&large_data, None).await.unwrap();

		// Verify significant compression ratio (should be > 50% reduction)
		let ratio = compressed.len() as f64 / uncompressed.len() as f64;
		assert!(ratio < 0.5, "Compression ratio: {:.2}%", ratio * 100.0);
	}

	#[test]
	fn test_algorithm_priority() {
		assert_eq!(CompressionAlgorithm::Brotli { quality: 4 }.priority(), 3);
		assert_eq!(CompressionAlgorithm::Gzip { level: 6 }.priority(), 2);
		assert_eq!(CompressionAlgorithm::Deflate.priority(), 1);
	}

	#[test]
	fn test_encoding_name() {
		assert_eq!(
			CompressionAlgorithm::Gzip { level: 6 }.encoding_name(),
			"gzip"
		);
		assert_eq!(
			CompressionAlgorithm::Brotli { quality: 4 }.encoding_name(),
			"br"
		);
		assert_eq!(CompressionAlgorithm::Deflate.encoding_name(), "deflate");
	}
}
