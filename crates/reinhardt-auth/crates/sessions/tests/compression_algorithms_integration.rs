//! Compression Algorithms Integration Tests
//!
//! This module contains comprehensive tests for session compression algorithms
//! including Zstd, Gzip, and Brotli. Tests verify correctness, compression ratio,
//! and edge cases.
//!
//! # Test Categories
//!
//! - Boundary Value: Different data sizes (empty, small, large)
//! - Edge Cases: Random data, highly compressible data, incompressible data
//! - Roundtrip: Compress-decompress verification
//! - Performance: Compression ratios for different data types

#[cfg(feature = "compression-zstd")]
use reinhardt_sessions::compression::ZstdCompressor;

#[cfg(feature = "compression-gzip")]
use reinhardt_sessions::compression::GzipCompressor;

#[cfg(feature = "compression-brotli")]
use reinhardt_sessions::compression::BrotliCompressor;

use reinhardt_sessions::compression::Compressor;
use rstest::*;

// =============================================================================
// Zstd Compressor Tests
// =============================================================================

#[cfg(feature = "compression-zstd")]
mod zstd_tests {
	use super::*;

	#[fixture]
	fn zstd_compressor() -> ZstdCompressor {
		ZstdCompressor::new()
	}

	// =========================================================================
	// Happy Path Tests
	// =========================================================================

	#[rstest]
	fn test_zstd_roundtrip_simple(zstd_compressor: ZstdCompressor) {
		let data = b"Hello, World! This is test data for compression.";

		let compressed = zstd_compressor.compress(data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed.as_slice(), data);
	}

	#[rstest]
	fn test_zstd_name(zstd_compressor: ZstdCompressor) {
		assert_eq!(zstd_compressor.name(), "zstd");
	}

	#[rstest]
	fn test_zstd_default_level() {
		let compressor = ZstdCompressor::default();
		assert_eq!(compressor.level(), 3, "Default level should be 3");
	}

	// =========================================================================
	// Boundary Value Tests - Data Sizes
	// =========================================================================

	#[rstest]
	#[case(0, "empty")]
	#[case(1, "single byte")]
	#[case(10, "10 bytes")]
	#[case(100, "100 bytes")]
	#[case(1024, "1 KB")]
	#[case(65536, "64 KB")]
	#[case(1048576, "1 MB")]
	fn test_zstd_various_sizes(
		zstd_compressor: ZstdCompressor,
		#[case] size: usize,
		#[case] desc: &str,
	) {
		let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

		let compressed = zstd_compressor.compress(&data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(
			decompressed.len(),
			size,
			"Decompressed size should match original for {}",
			desc
		);
		assert_eq!(
			decompressed, data,
			"Decompressed data should match original for {}",
			desc
		);
	}

	// =========================================================================
	// Compression Level Tests
	// =========================================================================

	#[rstest]
	#[case(1, "level 1 - fastest")]
	#[case(3, "level 3 - default")]
	#[case(9, "level 9 - balanced")]
	#[case(19, "level 19 - slow")]
	fn test_zstd_compression_levels(#[case] level: i32, #[case] desc: &str) {
		let compressor = ZstdCompressor::with_level(level);
		let data = b"Test data for compression level testing.".repeat(100);

		let compressed = compressor.compress(&data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(compressor.level(), level, "Level should match for {}", desc);
		assert_eq!(decompressed, data, "Roundtrip should work for {}", desc);
	}

	#[rstest]
	fn test_zstd_higher_level_better_compression(_zstd_compressor: ZstdCompressor) {
		let data = b"Repetitive test data. ".repeat(1000);

		let low = ZstdCompressor::with_level(1);
		let high = ZstdCompressor::with_level(19);

		let low_compressed = low.compress(&data).unwrap();
		let high_compressed = high.compress(&data).unwrap();

		// Higher level should produce equal or smaller output
		assert!(
			high_compressed.len() <= low_compressed.len(),
			"Higher level should compress better: low={}, high={}",
			low_compressed.len(),
			high_compressed.len()
		);
	}

	// =========================================================================
	// Edge Cases - Data Patterns
	// =========================================================================

	#[rstest]
	fn test_zstd_highly_compressible_data(zstd_compressor: ZstdCompressor) {
		// All same byte - highly compressible
		let data = vec![b'A'; 10_000];

		let compressed = zstd_compressor.compress(&data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed, data);

		// Should achieve significant compression
		let ratio = compressed.len() as f64 / data.len() as f64;
		assert!(
			ratio < 0.01,
			"Compression ratio should be < 1% for repetitive data, got {}%",
			ratio * 100.0
		);
	}

	#[rstest]
	fn test_zstd_random_data(zstd_compressor: ZstdCompressor) {
		// Pseudo-random data (less compressible)
		let data: Vec<u8> = (0..1000u32)
			.map(|i| ((i.wrapping_mul(1103515245).wrapping_add(12345)) % 256) as u8)
			.collect();

		let compressed = zstd_compressor.compress(&data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed, data);
	}

	#[rstest]
	fn test_zstd_json_like_data(zstd_compressor: ZstdCompressor) {
		// Typical session data
		let data = br#"{"session_id":"abc123","user_id":12345,"created_at":1704067200,"data":{"csrf_token":"xyz789","preferences":{"theme":"dark","language":"en"}}}"#;

		let compressed = zstd_compressor.compress(data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed.as_slice(), data.as_slice());
	}

	#[rstest]
	fn test_zstd_binary_data(zstd_compressor: ZstdCompressor) {
		// Binary data with null bytes
		let data: Vec<u8> = (0..256u16).flat_map(|i| vec![i as u8; 4]).collect();

		let compressed = zstd_compressor.compress(&data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed, data);
	}

	// =========================================================================
	// Error Cases
	// =========================================================================

	#[rstest]
	fn test_zstd_decompress_invalid_data(zstd_compressor: ZstdCompressor) {
		let invalid_data = b"This is not valid zstd compressed data";

		let result = zstd_compressor.decompress(invalid_data);

		assert!(result.is_err(), "Invalid data should return error");
	}

	#[rstest]
	fn test_zstd_decompress_truncated_data(zstd_compressor: ZstdCompressor) {
		let data = b"Valid test data for compression";
		let compressed = zstd_compressor.compress(data).unwrap();

		// Truncate the compressed data
		let truncated = &compressed[..compressed.len() / 2];

		let result = zstd_compressor.decompress(truncated);

		assert!(result.is_err(), "Truncated data should return error");
	}

	// =========================================================================
	// Clone Tests
	// =========================================================================

	#[rstest]
	fn test_zstd_clone(zstd_compressor: ZstdCompressor) {
		let cloned = zstd_compressor.clone();

		assert_eq!(cloned.level(), zstd_compressor.level());
		assert_eq!(cloned.name(), zstd_compressor.name());

		// Both should work independently
		let data = b"Test data";
		let compressed1 = zstd_compressor.compress(data).unwrap();
		let compressed2 = cloned.compress(data).unwrap();

		// Same level should produce same output
		assert_eq!(compressed1, compressed2);
	}
}

// =============================================================================
// Gzip Compressor Tests
// =============================================================================

#[cfg(feature = "compression-gzip")]
mod gzip_tests {
	use super::*;

	#[fixture]
	fn gzip_compressor() -> GzipCompressor {
		GzipCompressor::new()
	}

	#[rstest]
	fn test_gzip_roundtrip(gzip_compressor: GzipCompressor) {
		let data = b"Hello, World! This is test data for gzip compression.";

		let compressed = gzip_compressor.compress(data).unwrap();
		let decompressed = gzip_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed.as_slice(), data);
	}

	#[rstest]
	fn test_gzip_name(gzip_compressor: GzipCompressor) {
		assert_eq!(gzip_compressor.name(), "gzip");
	}

	#[rstest]
	#[case(0, "empty")]
	#[case(1, "single byte")]
	#[case(1024, "1 KB")]
	#[case(65536, "64 KB")]
	fn test_gzip_various_sizes(
		gzip_compressor: GzipCompressor,
		#[case] size: usize,
		#[case] desc: &str,
	) {
		let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

		let compressed = gzip_compressor.compress(&data).unwrap();
		let decompressed = gzip_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed, data, "Roundtrip should work for {}", desc);
	}

	#[rstest]
	fn test_gzip_compression_ratio(gzip_compressor: GzipCompressor) {
		let data = vec![b'A'; 10_000];

		let compressed = gzip_compressor.compress(&data).unwrap();

		let ratio = compressed.len() as f64 / data.len() as f64;
		assert!(
			ratio < 0.1,
			"Gzip should achieve good compression on repetitive data, got {}%",
			ratio * 100.0
		);
	}

	#[rstest]
	fn test_gzip_decompress_invalid(gzip_compressor: GzipCompressor) {
		let result = gzip_compressor.decompress(b"invalid gzip data");
		assert!(result.is_err());
	}
}

// =============================================================================
// Brotli Compressor Tests
// =============================================================================

#[cfg(feature = "compression-brotli")]
mod brotli_tests {
	use super::*;

	#[fixture]
	fn brotli_compressor() -> BrotliCompressor {
		BrotliCompressor::new()
	}

	#[rstest]
	fn test_brotli_roundtrip(brotli_compressor: BrotliCompressor) {
		let data = b"Hello, World! This is test data for brotli compression.";

		let compressed = brotli_compressor.compress(data).unwrap();
		let decompressed = brotli_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed.as_slice(), data);
	}

	#[rstest]
	fn test_brotli_name(brotli_compressor: BrotliCompressor) {
		assert_eq!(brotli_compressor.name(), "brotli");
	}

	#[rstest]
	#[case(0, "empty")]
	#[case(1, "single byte")]
	#[case(1024, "1 KB")]
	#[case(65536, "64 KB")]
	fn test_brotli_various_sizes(
		brotli_compressor: BrotliCompressor,
		#[case] size: usize,
		#[case] desc: &str,
	) {
		let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

		let compressed = brotli_compressor.compress(&data).unwrap();
		let decompressed = brotli_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed, data, "Roundtrip should work for {}", desc);
	}

	#[rstest]
	fn test_brotli_compression_ratio(brotli_compressor: BrotliCompressor) {
		let data = vec![b'A'; 10_000];

		let compressed = brotli_compressor.compress(&data).unwrap();

		let ratio = compressed.len() as f64 / data.len() as f64;
		assert!(
			ratio < 0.1,
			"Brotli should achieve good compression on repetitive data, got {}%",
			ratio * 100.0
		);
	}

	#[rstest]
	fn test_brotli_decompress_invalid(brotli_compressor: BrotliCompressor) {
		let result = brotli_compressor.decompress(b"invalid brotli data");
		assert!(result.is_err());
	}
}

// =============================================================================
// Cross-Algorithm Comparison Tests
// =============================================================================

#[cfg(all(
	feature = "compression-zstd",
	feature = "compression-gzip",
	feature = "compression-brotli"
))]
mod comparison_tests {
	use super::*;

	/// Test that all algorithms produce correct roundtrips
	#[rstest]
	fn test_all_algorithms_roundtrip() {
		let data = b"Test data for cross-algorithm comparison. ".repeat(100);

		let zstd = ZstdCompressor::new();
		let gzip = GzipCompressor::new();
		let brotli = BrotliCompressor::new();

		// Zstd roundtrip
		let zstd_compressed = zstd.compress(&data).unwrap();
		let zstd_decompressed = zstd.decompress(&zstd_compressed).unwrap();
		assert_eq!(zstd_decompressed, data);

		// Gzip roundtrip
		let gzip_compressed = gzip.compress(&data).unwrap();
		let gzip_decompressed = gzip.decompress(&gzip_compressed).unwrap();
		assert_eq!(gzip_decompressed, data);

		// Brotli roundtrip
		let brotli_compressed = brotli.compress(&data).unwrap();
		let brotli_decompressed = brotli.decompress(&brotli_compressed).unwrap();
		assert_eq!(brotli_decompressed, data);
	}

	/// Compare compression ratios (informational)
	#[rstest]
	fn test_compression_ratio_comparison() {
		let data = b"Session data: {\"user_id\": 12345, \"token\": \"abc123xyz\"} ".repeat(100);

		let zstd = ZstdCompressor::new();
		let gzip = GzipCompressor::new();
		let brotli = BrotliCompressor::new();

		let zstd_size = zstd.compress(&data).unwrap().len();
		let gzip_size = gzip.compress(&data).unwrap().len();
		let brotli_size = brotli.compress(&data).unwrap().len();

		// All should achieve meaningful compression
		assert!(zstd_size < data.len(), "Zstd should compress data");
		assert!(gzip_size < data.len(), "Gzip should compress data");
		assert!(brotli_size < data.len(), "Brotli should compress data");

		// Log ratios for comparison (not strict assertions)
		let original = data.len();
		eprintln!("Compression ratios - Original: {} bytes", original);
		eprintln!(
			"  Zstd: {} bytes ({:.1}%)",
			zstd_size,
			zstd_size as f64 / original as f64 * 100.0
		);
		eprintln!(
			"  Gzip: {} bytes ({:.1}%)",
			gzip_size,
			gzip_size as f64 / original as f64 * 100.0
		);
		eprintln!(
			"  Brotli: {} bytes ({:.1}%)",
			brotli_size,
			brotli_size as f64 / original as f64 * 100.0
		);
	}
}

// =============================================================================
// Compressor Trait Object Tests
// =============================================================================

#[cfg(feature = "compression-zstd")]
mod trait_object_tests {
	use super::*;

	/// Test compressor using generic function instead of trait object.
	/// Note: Compressor trait requires Clone which makes it not dyn-compatible,
	/// so we use generics to test polymorphic behavior.
	fn test_compressor_generic<C: Compressor>(compressor: C) {
		let data = b"Test data for generic function";

		let compressed = compressor.compress(data).unwrap();
		let decompressed = compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed.as_slice(), data);
	}

	#[rstest]
	fn test_compressor_polymorphism() {
		// Test ZstdCompressor through generic function
		test_compressor_generic(ZstdCompressor::new());
	}

	#[rstest]
	fn test_compressor_send_sync() {
		// Verify that compressors implement Send + Sync
		fn assert_send_sync<T: Send + Sync>() {}
		assert_send_sync::<ZstdCompressor>();

		#[cfg(feature = "compression-gzip")]
		assert_send_sync::<GzipCompressor>();

		#[cfg(feature = "compression-brotli")]
		assert_send_sync::<BrotliCompressor>();
	}
}

// =============================================================================
// Unicode and Special Data Tests
// =============================================================================

#[cfg(feature = "compression-zstd")]
mod special_data_tests {
	use super::*;

	#[fixture]
	fn zstd_compressor() -> ZstdCompressor {
		ZstdCompressor::new()
	}

	#[rstest]
	#[case("Hello, World!", "ASCII")]
	#[case("こんにちは世界", "Japanese")]
	#[case("Привет мир", "Russian")]
	#[case("مرحبا بالعالم", "Arabic")]
	#[case("🎉🚀💻🔥", "Emoji")]
	fn test_unicode_compression(
		zstd_compressor: ZstdCompressor,
		#[case] text: &str,
		#[case] desc: &str,
	) {
		let data = text.as_bytes();

		let compressed = zstd_compressor.compress(data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		let restored = std::str::from_utf8(&decompressed).unwrap();
		assert_eq!(restored, text, "Unicode should be preserved for {}", desc);
	}

	#[rstest]
	fn test_all_byte_values(zstd_compressor: ZstdCompressor) {
		// Data containing all possible byte values
		let data: Vec<u8> = (0..=255u8).collect();

		let compressed = zstd_compressor.compress(&data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed, data);
	}

	#[rstest]
	fn test_null_byte_data(zstd_compressor: ZstdCompressor) {
		// Data with embedded null bytes
		let data = b"before\0middle\0after";

		let compressed = zstd_compressor.compress(data).unwrap();
		let decompressed = zstd_compressor.decompress(&compressed).unwrap();

		assert_eq!(decompressed.as_slice(), data.as_slice());
	}
}
