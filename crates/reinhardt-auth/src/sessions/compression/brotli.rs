//! Brotli compression support
//!
//! This module provides Brotli compression, which offers the best compression ratio
//! among all supported algorithms. Brotli is ideal when storage size is critical
//! and you can tolerate slightly slower compression.
//!
//! ## Characteristics
//!
//! - **Speed**: Slower compression, fast decompression
//! - **Compression Ratio**: Excellent (best among all algorithms)
//! - **CPU Usage**: Higher for compression, moderate for decompression
//! - **Recommended Level**: 6 (default)
//!
//! ## Example
//!
//! ```rust,no_run
//! use reinhardt_auth::sessions::compression::{Compressor, BrotliCompressor};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let compressor = BrotliCompressor::new();
//!
//! let data = b"Hello, World! This is test data for compression.";
//! let compressed = compressor.compress(data)?;
//! let decompressed = compressor.decompress(&compressed)?;
//!
//! assert_eq!(data, decompressed.as_slice());
//! # Ok(())
//! # }
//! ```

use super::{CompressionError, Compressor};
use std::io::Read;

/// Brotli compressor
///
/// Provides the best compression ratio among all supported algorithms.
/// Use this when storage size is critical and you can tolerate slower compression.
///
/// # Compression Levels
///
/// - Level 0-3: Fast compression, lower compression ratio
/// - Level 4-6: Balanced (recommended: 6)
/// - Level 7-9: Better compression, slower
/// - Level 10-11: Maximum compression, very slow
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_auth::sessions::compression::{Compressor, BrotliCompressor};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Default level (6)
/// let compressor = BrotliCompressor::new();
///
/// // Custom level
/// let max_compression = BrotliCompressor::with_level(11);
///
/// let data = b"Test data";
/// let compressed = compressor.compress(data)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct BrotliCompressor {
	level: u32,
}

impl BrotliCompressor {
	/// Create a new Brotli compressor with default level (6)
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::compression::BrotliCompressor;
	///
	/// let compressor = BrotliCompressor::new();
	/// ```
	pub fn new() -> Self {
		Self { level: 6 }
	}

	/// Create a new Brotli compressor with custom level (0-11)
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::compression::BrotliCompressor;
	///
	/// // Maximum compression
	/// let compressor = BrotliCompressor::with_level(11);
	/// ```
	pub fn with_level(level: u32) -> Self {
		Self { level }
	}

	/// Get the compression level
	pub fn level(&self) -> u32 {
		self.level
	}
}

impl Default for BrotliCompressor {
	fn default() -> Self {
		Self::new()
	}
}

impl Compressor for BrotliCompressor {
	fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
		let mut compressed = Vec::new();
		let mut compressor = brotli::CompressorReader::new(
			data, 4096, // buffer size
			self.level, 22, // window size (default)
		);
		compressor
			.read_to_end(&mut compressed)
			.map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
		Ok(compressed)
	}

	fn decompress(&self, compressed: &[u8]) -> Result<Vec<u8>, CompressionError> {
		let mut decompressed = Vec::new();
		let mut decompressor = brotli::Decompressor::new(compressed, 4096);
		decompressor
			.read_to_end(&mut decompressed)
			.map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;
		Ok(decompressed)
	}

	fn name(&self) -> &'static str {
		"brotli"
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_brotli_compress_decompress() {
		let compressor = BrotliCompressor::new();
		let data = b"Hello, World! This is test data for Brotli compression.";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}

	#[rstest]
	fn test_brotli_compression_ratio() {
		let compressor = BrotliCompressor::new();
		let data = b"A".repeat(1000); // Highly compressible data

		let compressed = compressor.compress(&data).unwrap();

		// Brotli should achieve excellent compression on repetitive data
		assert!(compressed.len() < data.len() / 10);
	}

	#[rstest]
	fn test_brotli_with_level() {
		let low_compression = BrotliCompressor::with_level(1);
		let high_compression = BrotliCompressor::with_level(11);

		let data = b"Test data for compression level comparison.".repeat(10);

		let low_compressed = low_compression.compress(&data).unwrap();
		let high_compressed = high_compression.compress(&data).unwrap();

		// Higher level should produce smaller output
		assert!(high_compressed.len() <= low_compressed.len());

		// Both should decompress correctly
		assert_eq!(low_compression.decompress(&low_compressed).unwrap(), data);
		assert_eq!(high_compression.decompress(&high_compressed).unwrap(), data);
	}

	#[rstest]
	fn test_brotli_empty_data() {
		let compressor = BrotliCompressor::new();
		let data = b"";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}

	#[rstest]
	fn test_brotli_large_data() {
		let compressor = BrotliCompressor::new();
		let data = vec![b'X'; 100_000]; // 100KB of data

		let compressed = compressor.compress(&data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed);
		// Repetitive data should compress very well
		assert!(compressed.len() < 1000);
	}

	#[rstest]
	fn test_brotli_name() {
		let compressor = BrotliCompressor::new();
		assert_eq!(compressor.name(), "brotli");
	}

	#[rstest]
	fn test_brotli_level_getter() {
		let compressor = BrotliCompressor::with_level(9);
		assert_eq!(compressor.level(), 9);
	}

	#[rstest]
	fn test_brotli_default() {
		let compressor = BrotliCompressor::default();
		assert_eq!(compressor.level(), 6);
	}

	#[rstest]
	fn test_brotli_fast_compression() {
		let compressor = BrotliCompressor::with_level(0);
		let data = b"Test data for fast compression";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}
}
