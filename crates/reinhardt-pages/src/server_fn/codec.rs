//! Codec Abstraction for Server Functions (Week 4 Day 3)
//!
//! This module provides pluggable serialization formats for server functions.
//! Different codecs are suitable for different use cases:
//!
//! - **JSON**: Default, widely supported, human-readable
//! - **URL Encoding**: For GET requests with simple data types
//! - **MessagePack**: Binary format for efficiency (optional)
//!
//! ## Architecture
//!
//! The `Codec` trait abstracts serialization/deserialization, allowing
//! server functions to use different formats based on requirements.
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::server_fn::codec::{Codec, JsonCodec};
//!
//! let codec = JsonCodec;
//! let data = MyStruct { id: 42 };
//! let encoded = codec.encode(&data)?;
//! let decoded: MyStruct = codec.decode(&encoded)?;
//! ```

use serde::{Deserialize, Serialize};

/// Codec trait for serialization/deserialization
///
/// This trait abstracts the encoding format used for server function
/// arguments and return values.
///
/// ## Implementation Notes
///
/// - `encode()` should produce a format suitable for HTTP transmission
/// - `decode()` should handle the same format
/// - Content-Type header should match the codec format
pub trait Codec {
	/// Encode a value to bytes
	///
	/// # Arguments
	///
	/// * `value` - Value to encode
	///
	/// # Errors
	///
	/// Returns error if serialization fails
	fn encode<T>(&self, value: &T) -> Result<Vec<u8>, String>
	where
		T: Serialize;

	/// Decode bytes to a value
	///
	/// # Arguments
	///
	/// * `bytes` - Bytes to decode
	///
	/// # Errors
	///
	/// Returns error if deserialization fails
	fn decode<T>(&self, bytes: &[u8]) -> Result<T, String>
	where
		T: for<'de> Deserialize<'de>;

	/// Get the Content-Type header value for this codec
	///
	/// # Returns
	///
	/// MIME type string (e.g., "application/json")
	fn content_type(&self) -> &'static str;

	/// Get the codec name
	///
	/// # Returns
	///
	/// Codec identifier (e.g., "json", "url", "msgpack")
	fn name(&self) -> &'static str;
}

/// JSON codec (default)
///
/// Uses `serde_json` for serialization. This is the default codec
/// for server functions due to its wide support and human-readable format.
///
/// ## Characteristics
///
/// - Content-Type: `application/json`
/// - Human-readable
/// - Widely supported
/// - Suitable for most use cases
///
/// ## Example
///
/// ```ignore
/// let codec = JsonCodec;
/// let user = User { id: 1, name: "Alice".into() };
/// let json = codec.encode(&user)?;
/// // json: b"{\"id\":1,\"name\":\"Alice\"}"
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonCodec;

impl Codec for JsonCodec {
	fn encode<T>(&self, value: &T) -> Result<Vec<u8>, String>
	where
		T: Serialize,
	{
		serde_json::to_vec(value).map_err(|e| format!("JSON encoding failed: {}", e))
	}

	fn decode<T>(&self, bytes: &[u8]) -> Result<T, String>
	where
		T: for<'de> Deserialize<'de>,
	{
		serde_json::from_slice(bytes).map_err(|e| format!("JSON decoding failed: {}", e))
	}

	fn content_type(&self) -> &'static str {
		"application/json"
	}

	fn name(&self) -> &'static str {
		"json"
	}
}

/// URL encoding codec
///
/// Uses `serde_urlencoded` for serialization. Suitable for GET requests
/// where data needs to be in URL query parameters.
///
/// ## Characteristics
///
/// - Content-Type: `application/x-www-form-urlencoded`
/// - URL-safe
/// - Limited to simple data structures (flat key-value pairs)
/// - Suitable for GET requests
///
/// ## Limitations
///
/// - No nested structures (limited by URL encoding format)
/// - Strings, numbers, booleans only
/// - Not suitable for complex data types
///
/// ## Example
///
/// ```ignore
/// let codec = UrlCodec;
/// let params = SearchParams { query: "rust", page: 1 };
/// let encoded = codec.encode(&params)?;
/// // encoded: b"query=rust&page=1"
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct UrlCodec;

impl Codec for UrlCodec {
	fn encode<T>(&self, value: &T) -> Result<Vec<u8>, String>
	where
		T: Serialize,
	{
		serde_urlencoded::to_string(value)
			.map(|s| s.into_bytes())
			.map_err(|e| format!("URL encoding failed: {}", e))
	}

	fn decode<T>(&self, bytes: &[u8]) -> Result<T, String>
	where
		T: for<'de> Deserialize<'de>,
	{
		serde_urlencoded::from_bytes(bytes).map_err(|e| format!("URL decoding failed: {}", e))
	}

	fn content_type(&self) -> &'static str {
		"application/x-www-form-urlencoded"
	}

	fn name(&self) -> &'static str {
		"url"
	}
}

/// MessagePack codec (optional, for binary efficiency)
///
/// Uses `rmp-serde` for binary serialization. More efficient than JSON
/// for large payloads or bandwidth-constrained environments.
///
/// ## Characteristics
///
/// - Content-Type: `application/msgpack`
/// - Binary format (not human-readable)
/// - More compact than JSON (typically 30-50% smaller)
/// - Faster serialization/deserialization
/// - Suitable for large data transfers
///
/// ## Example
///
/// ```ignore
/// let codec = MessagePackCodec;
/// let large_data = vec![1, 2, 3, 4, 5]; // Large dataset
/// let encoded = codec.encode(&large_data)?;
/// // encoded: binary MessagePack format (smaller than JSON)
/// ```
///
/// ## Availability
///
/// This codec requires the `msgpack` feature flag:
///
/// ```toml
/// [dependencies]
/// reinhardt-pages = { version = "0.1", features = ["msgpack"] }
/// ```
#[cfg(feature = "msgpack")]
#[derive(Debug, Clone, Copy, Default)]
pub struct MessagePackCodec;

#[cfg(feature = "msgpack")]
impl Codec for MessagePackCodec {
	fn encode<T>(&self, value: &T) -> Result<Vec<u8>, String>
	where
		T: Serialize,
	{
		rmp_serde::to_vec(value).map_err(|e| format!("MessagePack encoding failed: {}", e))
	}

	fn decode<T>(&self, bytes: &[u8]) -> Result<T, String>
	where
		T: for<'de> Deserialize<'de>,
	{
		rmp_serde::from_slice(bytes).map_err(|e| format!("MessagePack decoding failed: {}", e))
	}

	fn content_type(&self) -> &'static str {
		"application/msgpack"
	}

	fn name(&self) -> &'static str {
		"msgpack"
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct TestData {
		id: u32,
		name: String,
	}

	#[test]
	fn test_json_codec() {
		let codec = JsonCodec;
		let data = TestData {
			id: 42,
			name: "Alice".to_string(),
		};

		// Encode
		let encoded = codec.encode(&data).unwrap();
		let json_str = String::from_utf8(encoded.clone()).unwrap();
		assert!(json_str.contains("42"));
		assert!(json_str.contains("Alice"));

		// Decode
		let decoded: TestData = codec.decode(&encoded).unwrap();
		assert_eq!(decoded, data);

		// Metadata
		assert_eq!(codec.content_type(), "application/json");
		assert_eq!(codec.name(), "json");
	}

	#[test]
	fn test_url_codec() {
		let codec = UrlCodec;

		#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
		struct SimpleData {
			query: String,
			page: u32,
		}

		let data = SimpleData {
			query: "rust".to_string(),
			page: 1,
		};

		// Encode
		let encoded = codec.encode(&data).unwrap();
		let url_str = String::from_utf8(encoded.clone()).unwrap();
		assert!(url_str.contains("query=rust"));
		assert!(url_str.contains("page=1"));

		// Decode
		let decoded: SimpleData = codec.decode(&encoded).unwrap();
		assert_eq!(decoded, data);

		// Metadata
		assert_eq!(codec.content_type(), "application/x-www-form-urlencoded");
		assert_eq!(codec.name(), "url");
	}

	#[cfg(feature = "msgpack")]
	#[test]
	fn test_msgpack_codec() {
		let codec = MessagePackCodec;
		let data = TestData {
			id: 42,
			name: "Alice".to_string(),
		};

		// Encode
		let encoded = codec.encode(&data).unwrap();

		// MessagePack should be more compact than JSON
		let json_codec = JsonCodec;
		let json_encoded = json_codec.encode(&data).unwrap();
		assert!(encoded.len() < json_encoded.len());

		// Decode
		let decoded: TestData = codec.decode(&encoded).unwrap();
		assert_eq!(decoded, data);

		// Metadata
		assert_eq!(codec.content_type(), "application/msgpack");
		assert_eq!(codec.name(), "msgpack");
	}

	#[test]
	fn test_codec_error_handling() {
		let codec = JsonCodec;

		// Invalid JSON
		let invalid_json = b"{ invalid json }";
		let result: Result<TestData, _> = codec.decode(invalid_json);
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("JSON decoding failed"));
	}
}
