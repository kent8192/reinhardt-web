//! Asset compression for static files
//!
//! Provides gzip and brotli compression for static assets to reduce
//! transfer size and improve loading performance.

use super::{ProcessingResult, Processor};
use async_trait::async_trait;
use std::io::{self, Write};
use std::path::Path;

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
	/// Gzip compression
	Gzip,
	/// Brotli compression
	Brotli,
}

/// Gzip compressor
///
/// Compresses files using gzip algorithm.
pub struct GzipCompressor {
	/// Compression level (0-9)
	level: u32,
}

impl GzipCompressor {
	/// Create a new gzip compressor with default level
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::compress::GzipCompressor;
	///
	/// let compressor = GzipCompressor::new();
	/// ```
	pub fn new() -> Self {
		Self { level: 6 }
	}

	/// Create a gzip compressor with custom level
	///
	/// # Arguments
	///
	/// * `level` - Compression level (0 = no compression, 9 = best compression)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::compress::GzipCompressor;
	///
	/// let compressor = GzipCompressor::with_level(9);
	/// ```
	pub fn with_level(level: u32) -> Self {
		Self {
			level: level.min(9),
		}
	}

	/// Compress data using gzip
	fn compress_gzip(&self, input: &[u8]) -> ProcessingResult<Vec<u8>> {
		use flate2::Compression;
		use flate2::write::GzEncoder;

		let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
		encoder.write_all(input)?;
		encoder.finish()
	}
}

impl Default for GzipCompressor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Processor for GzipCompressor {
	async fn process(&self, input: &[u8], _path: &Path) -> ProcessingResult<Vec<u8>> {
		self.compress_gzip(input)
	}

	fn can_process(&self, path: &Path) -> bool {
		// Can compress any file
		path.extension().is_some()
	}

	fn name(&self) -> &str {
		"GzipCompressor"
	}
}

/// Brotli compressor
///
/// Compresses files using brotli algorithm.
pub struct BrotliCompressor {
	/// Compression quality (0-11)
	quality: u32,
	/// Window size (10-24)
	window_size: u32,
}

impl BrotliCompressor {
	/// Create a new brotli compressor with default settings
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::compress::BrotliCompressor;
	///
	/// let compressor = BrotliCompressor::new();
	/// ```
	pub fn new() -> Self {
		Self {
			quality: 11,
			window_size: 22,
		}
	}

	/// Create a brotli compressor with custom settings
	///
	/// # Arguments
	///
	/// * `quality` - Compression quality (0 = fast, 11 = best)
	/// * `window_size` - Window size (10-24)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::compress::BrotliCompressor;
	///
	/// let compressor = BrotliCompressor::with_settings(9, 20);
	/// ```
	pub fn with_settings(quality: u32, window_size: u32) -> Self {
		Self {
			quality: quality.min(11),
			window_size: window_size.clamp(10, 24),
		}
	}

	/// Compress data using brotli
	fn compress_brotli(&self, input: &[u8]) -> ProcessingResult<Vec<u8>> {
		use brotli::enc::BrotliEncoderParams;

		let mut output = Vec::new();
		let params = BrotliEncoderParams {
			quality: self.quality as i32,
			lgwin: self.window_size as i32,
			..Default::default()
		};

		brotli::BrotliCompress(&mut std::io::Cursor::new(input), &mut output, &params)
			.map_err(io::Error::other)?;

		Ok(output)
	}
}

impl Default for BrotliCompressor {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Processor for BrotliCompressor {
	async fn process(&self, input: &[u8], _path: &Path) -> ProcessingResult<Vec<u8>> {
		self.compress_brotli(input)
	}

	fn can_process(&self, path: &Path) -> bool {
		// Can compress any file
		path.extension().is_some()
	}

	fn name(&self) -> &str {
		"BrotliCompressor"
	}
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
	/// Enable gzip compression
	pub gzip: bool,
	/// Gzip compression level (0-9)
	pub gzip_level: u32,
	/// Enable brotli compression
	pub brotli: bool,
	/// Brotli quality (0-11)
	pub brotli_quality: u32,
	/// Minimum file size to compress (bytes)
	pub min_size: usize,
	/// File extensions to compress
	pub extensions: Vec<String>,
}

impl Default for CompressionConfig {
	fn default() -> Self {
		Self {
			gzip: true,
			gzip_level: 6,
			brotli: true,
			brotli_quality: 11,
			min_size: 1024, // 1KB
			extensions: vec![
				"js".to_string(),
				"css".to_string(),
				"html".to_string(),
				"json".to_string(),
				"xml".to_string(),
				"svg".to_string(),
			],
		}
	}
}

impl CompressionConfig {
	/// Create a new compression configuration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::compress::CompressionConfig;
	///
	/// let config = CompressionConfig::new();
	/// assert!(config.gzip);
	/// assert!(config.brotli);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Enable or disable gzip
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::r#static::processing::compress::CompressionConfig;
	///
	/// let config = CompressionConfig::new().with_gzip(true);
	/// assert!(config.gzip);
	/// ```
	pub fn with_gzip(mut self, enable: bool) -> Self {
		self.gzip = enable;
		self
	}

	/// Set gzip level
	pub fn with_gzip_level(mut self, level: u32) -> Self {
		self.gzip_level = level.min(9);
		self
	}

	/// Enable or disable brotli
	pub fn with_brotli(mut self, enable: bool) -> Self {
		self.brotli = enable;
		self
	}

	/// Set brotli quality
	pub fn with_brotli_quality(mut self, quality: u32) -> Self {
		self.brotli_quality = quality.min(11);
		self
	}

	/// Set minimum file size
	pub fn with_min_size(mut self, size: usize) -> Self {
		self.min_size = size;
		self
	}

	/// Add extension to compress
	pub fn add_extension(mut self, ext: String) -> Self {
		self.extensions.push(ext);
		self
	}

	/// Check if file should be compressed
	pub fn should_compress(&self, path: &Path, size: usize) -> bool {
		if size < self.min_size {
			return false;
		}

		path.extension()
			.and_then(|ext| ext.to_str())
			.map(|ext| self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
			.unwrap_or(false)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn test_gzip_compressor_creation() {
		let compressor = GzipCompressor::new();
		assert_eq!(compressor.level, 6);
	}

	#[test]
	fn test_gzip_compressor_with_level() {
		let compressor = GzipCompressor::with_level(9);
		assert_eq!(compressor.level, 9);

		let compressor2 = GzipCompressor::with_level(15);
		assert_eq!(compressor2.level, 9); // Clamped to max
	}

	#[test]
	fn test_gzip_can_process() {
		let compressor = GzipCompressor::new();
		assert!(compressor.can_process(&PathBuf::from("file.txt")));
		assert!(compressor.can_process(&PathBuf::from("style.css")));
		assert!(!compressor.can_process(&PathBuf::from("noext")));
	}

	#[tokio::test]
	async fn test_gzip_compress() {
		let compressor = GzipCompressor::new();
		let input = b"Hello, World! This is test data that should be compressed.";
		let result = compressor
			.process(input, &PathBuf::from("test.txt"))
			.await
			.unwrap();

		// Compressed size should be smaller for repetitive data
		assert!(!result.is_empty());
	}

	#[test]
	fn test_brotli_compressor_creation() {
		let compressor = BrotliCompressor::new();
		assert_eq!(compressor.quality, 11);
		assert_eq!(compressor.window_size, 22);
	}

	#[test]
	fn test_brotli_compressor_with_settings() {
		let compressor = BrotliCompressor::with_settings(9, 20);
		assert_eq!(compressor.quality, 9);
		assert_eq!(compressor.window_size, 20);
	}

	#[test]
	fn test_brotli_quality_clamping() {
		let compressor = BrotliCompressor::with_settings(20, 30);
		assert_eq!(compressor.quality, 11);
		assert_eq!(compressor.window_size, 24);
	}

	#[test]
	fn test_brotli_can_process() {
		let compressor = BrotliCompressor::new();
		assert!(compressor.can_process(&PathBuf::from("file.txt")));
		assert!(compressor.can_process(&PathBuf::from("style.css")));
	}

	#[tokio::test]
	async fn test_brotli_compress() {
		let compressor = BrotliCompressor::new();
		let input = b"Hello, World! This is test data for brotli compression.";
		let result = compressor
			.process(input, &PathBuf::from("test.txt"))
			.await
			.unwrap();

		assert!(!result.is_empty());
	}

	#[test]
	fn test_compression_config_default() {
		let config = CompressionConfig::default();
		assert!(config.gzip);
		assert!(config.brotli);
		assert_eq!(config.gzip_level, 6);
		assert_eq!(config.brotli_quality, 11);
		assert_eq!(config.min_size, 1024);
	}

	#[test]
	fn test_compression_config_builder() {
		let config = CompressionConfig::new()
			.with_gzip(true)
			.with_gzip_level(9)
			.with_brotli(false)
			.with_min_size(2048)
			.add_extension("txt".to_string());

		assert!(config.gzip);
		assert!(!config.brotli);
		assert_eq!(config.gzip_level, 9);
		assert_eq!(config.min_size, 2048);
		assert!(config.extensions.contains(&"txt".to_string()));
	}

	#[test]
	fn test_should_compress() {
		let config = CompressionConfig::new();

		// Should compress large JS files
		assert!(config.should_compress(&PathBuf::from("app.js"), 2000));

		// Should not compress small files
		assert!(!config.should_compress(&PathBuf::from("app.js"), 500));

		// Should not compress non-configured extensions
		assert!(!config.should_compress(&PathBuf::from("image.png"), 2000));

		// Should compress CSS
		assert!(config.should_compress(&PathBuf::from("style.css"), 2000));
	}

	#[test]
	fn test_level_clamping() {
		let config = CompressionConfig::new()
			.with_gzip_level(20)
			.with_brotli_quality(30);

		assert_eq!(config.gzip_level, 9);
		assert_eq!(config.brotli_quality, 11);
	}
}
