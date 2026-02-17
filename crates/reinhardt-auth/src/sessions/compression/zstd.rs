//! Zstandard (Zstd) compression support
//!
//! This module provides Zstd compression, which offers the best balance between
//! speed and compression ratio. Zstd is the recommended compression algorithm
//! for most use cases.
//!
//! ## Characteristics
//!
//! - **Speed**: Very fast compression and decompression
//! - **Compression Ratio**: Excellent (similar to gzip level 9)
//! - **CPU Usage**: Moderate
//! - **Recommended Level**: 3 (default)
//!
//! ## Example
//!
//! ```rust,no_run
//! use reinhardt_auth::sessions::compression::{Compressor, ZstdCompressor};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let compressor = ZstdCompressor::new();
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

/// Zstandard (Zstd) compressor
///
/// Provides balanced speed and compression ratio. This is the recommended
/// compression algorithm for most use cases.
///
/// # Compression Levels
///
/// - Level 1-3: Fast compression, moderate compression ratio (recommended: 3)
/// - Level 4-9: Balanced
/// - Level 10-19: Better compression, slower
/// - Level 20-22: Maximum compression, very slow
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_auth::sessions::compression::{Compressor, ZstdCompressor};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Default level (3)
/// let compressor = ZstdCompressor::new();
///
/// // Custom level
/// let high_compression = ZstdCompressor::with_level(9);
///
/// let data = b"Test data";
/// let compressed = compressor.compress(data)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ZstdCompressor {
	level: i32,
}

impl ZstdCompressor {
	/// Create a new Zstd compressor with default level (3)
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::compression::ZstdCompressor;
	///
	/// let compressor = ZstdCompressor::new();
	/// ```
	pub fn new() -> Self {
		Self { level: 3 }
	}

	/// Create a new Zstd compressor with custom level (1-22)
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::compression::ZstdCompressor;
	///
	/// // High compression
	/// let compressor = ZstdCompressor::with_level(9);
	/// ```
	pub fn with_level(level: i32) -> Self {
		Self { level }
	}

	/// Get the compression level
	pub fn level(&self) -> i32 {
		self.level
	}
}

impl Default for ZstdCompressor {
	fn default() -> Self {
		Self::new()
	}
}

impl Compressor for ZstdCompressor {
	fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
		zstd::encode_all(data, self.level)
			.map_err(|e| CompressionError::CompressionFailed(e.to_string()))
	}

	fn decompress(&self, compressed: &[u8]) -> Result<Vec<u8>, CompressionError> {
		zstd::decode_all(compressed)
			.map_err(|e| CompressionError::DecompressionFailed(e.to_string()))
	}

	fn name(&self) -> &'static str {
		"zstd"
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_zstd_compress_decompress() {
		let compressor = ZstdCompressor::new();
		let data = b"Hello, World! This is test data for Zstd compression.";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}

	#[rstest]
	fn test_zstd_compression_ratio() {
		let compressor = ZstdCompressor::new();
		let data = b"A".repeat(1000); // Highly compressible data

		let compressed = compressor.compress(&data).unwrap();

		// Zstd should achieve significant compression on repetitive data
		assert!(compressed.len() < data.len() / 10);
	}

	#[rstest]
	fn test_zstd_with_level() {
		let low_compression = ZstdCompressor::with_level(1);
		let high_compression = ZstdCompressor::with_level(19);

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
	fn test_zstd_empty_data() {
		let compressor = ZstdCompressor::new();
		let data = b"";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed.as_slice());
	}

	#[rstest]
	fn test_zstd_large_data() {
		let compressor = ZstdCompressor::new();
		let data = vec![b'X'; 100_000]; // 100KB of data

		let compressed = compressor.compress(&data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(data, decompressed);
		// Repetitive data should compress very well
		assert!(compressed.len() < 1000);
	}

	#[rstest]
	fn test_zstd_name() {
		let compressor = ZstdCompressor::new();
		assert_eq!(compressor.name(), "zstd");
	}

	#[rstest]
	fn test_zstd_level_getter() {
		let compressor = ZstdCompressor::with_level(9);
		assert_eq!(compressor.level(), 9);
	}

	#[rstest]
	fn test_zstd_default() {
		let compressor = ZstdCompressor::default();
		assert_eq!(compressor.level(), 3);
	}
}
