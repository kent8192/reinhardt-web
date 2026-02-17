//! Gzip compression support
//!
//! This module provides Gzip (DEFLATE) compression, which offers wide compatibility
//! and good compression ratio. Gzip is the most widely supported compression algorithm.
//!
//! ## Characteristics
//!
//! - **Speed**: Moderate
//! - **Compression Ratio**: Good
//! - **CPU Usage**: Moderate
//! - **Compatibility**: Excellent (RFC 1952)
//! - **Recommended Level**: 6 (default)
//!
//! ## Example
//!
//! ```rust,no_run
//! use reinhardt_auth::sessions::compression::{Compressor, GzipCompressor};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let compressor = GzipCompressor::new();
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
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use std::io::Read;

/// Gzip compressor
///
/// Provides wide compatibility and good compression ratio. Use this when
/// you need maximum compatibility across different platforms and languages.
///
/// # Compression Levels
///
/// - Level 0: No compression
/// - Level 1-3: Fast compression, lower compression ratio
/// - Level 4-6: Balanced (recommended: 6)
/// - Level 7-9: Better compression, slower
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_auth::sessions::compression::{Compressor, GzipCompressor};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Default level (6)
/// let compressor = GzipCompressor::new();
///
/// // Custom level
/// let fast_compression = GzipCompressor::with_level(3);
///
/// let data = b"Test data";
/// let compressed = compressor.compress(data)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct GzipCompressor {
	level: u32,
}

impl GzipCompressor {
	/// Create a new Gzip compressor with default level (6)
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::compression::GzipCompressor;
	///
	/// let compressor = GzipCompressor::new();
	/// ```
	pub fn new() -> Self {
		Self { level: 6 }
	}

	/// Create a new Gzip compressor with custom level (0-9)
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::compression::GzipCompressor;
	///
	/// // Fast compression
	/// let compressor = GzipCompressor::with_level(3);
	/// ```
	pub fn with_level(level: u32) -> Self {
		Self { level }
	}

	/// Get the compression level
	pub fn level(&self) -> u32 {
		self.level
	}
}

impl Default for GzipCompressor {
	fn default() -> Self {
		Self::new()
	}
}

impl Compressor for GzipCompressor {
	fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
		let mut encoder = GzEncoder::new(data, Compression::new(self.level));
		let mut compressed = Vec::new();
		encoder
			.read_to_end(&mut compressed)
			.map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
		Ok(compressed)
	}

	fn decompress(&self, compressed: &[u8]) -> Result<Vec<u8>, CompressionError> {
		let mut decoder = GzDecoder::new(compressed);
		let mut decompressed = Vec::new();
		decoder
			.read_to_end(&mut decompressed)
			.map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;
		Ok(decompressed)
	}

	fn name(&self) -> &'static str {
		"gzip"
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_gzip_compress_decompress() {
		let compressor = GzipCompressor::new();
		let data = b"Hello, World! This is test data for Gzip compression.";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}

	#[rstest]
	fn test_gzip_compression_ratio() {
		let compressor = GzipCompressor::new();
		let data = b"A".repeat(1000); // Highly compressible data

		let compressed = compressor.compress(&data).unwrap();

		// Gzip should achieve significant compression on repetitive data
		assert!(compressed.len() < data.len() / 10);
	}

	#[rstest]
	fn test_gzip_with_level() {
		let low_compression = GzipCompressor::with_level(1);
		let high_compression = GzipCompressor::with_level(9);

		let data = b"Test data for compression level comparison.".repeat(10);

		let low_compressed = low_compression.compress(&data).unwrap();
		let high_compressed = high_compression.compress(&data).unwrap();

		// Higher level should produce smaller or equal output
		assert!(high_compressed.len() <= low_compressed.len());

		// Both should decompress correctly
		assert_eq!(low_compression.decompress(&low_compressed).unwrap(), data);
		assert_eq!(high_compression.decompress(&high_compressed).unwrap(), data);
	}

	#[rstest]
	fn test_gzip_empty_data() {
		let compressor = GzipCompressor::new();
		let data = b"";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}

	#[rstest]
	fn test_gzip_large_data() {
		let compressor = GzipCompressor::new();
		let data = vec![b'X'; 100_000]; // 100KB of data

		let compressed = compressor.compress(&data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed);
		// Repetitive data should compress very well
		assert!(compressed.len() < 1000);
	}

	#[rstest]
	fn test_gzip_name() {
		let compressor = GzipCompressor::new();
		assert_eq!(compressor.name(), "gzip");
	}

	#[rstest]
	fn test_gzip_level_getter() {
		let compressor = GzipCompressor::with_level(9);
		assert_eq!(compressor.level(), 9);
	}

	#[rstest]
	fn test_gzip_default() {
		let compressor = GzipCompressor::default();
		assert_eq!(compressor.level(), 6);
	}

	#[rstest]
	fn test_gzip_no_compression() {
		let compressor = GzipCompressor::with_level(0);
		let data = b"Test data";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}
}
