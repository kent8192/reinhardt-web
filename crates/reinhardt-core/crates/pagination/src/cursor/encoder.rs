//! Cursor encoding strategies for cursor-based pagination

use reinhardt_exception::{Error, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Trait for encoding and decoding pagination cursors
///
/// Custom implementations can provide different encoding strategies,
/// such as encryption, compression, or signed tokens.
///
/// # Examples
///
/// ```
/// use reinhardt_pagination::cursor::{CursorEncoder, Base64CursorEncoder};
///
/// let encoder = Base64CursorEncoder::new();
/// let cursor = encoder.encode(42).unwrap();
/// let position = encoder.decode(&cursor).unwrap();
/// assert_eq!(position, 42);
/// ```
pub trait CursorEncoder: Send + Sync {
	/// Encode a position into a cursor string
	fn encode(&self, position: usize) -> Result<String>;

	/// Decode a cursor string back to a position
	fn decode(&self, cursor: &str) -> Result<usize>;
}

/// Base64 cursor encoder with timestamp and checksum validation
///
/// Encodes cursors as base64(position:timestamp:checksum) to prevent tampering
/// and provide automatic expiration.
///
/// # Examples
///
/// ```
/// use reinhardt_pagination::cursor::{CursorEncoder, Base64CursorEncoder};
///
/// let encoder = Base64CursorEncoder::new();
/// let cursor = encoder.encode(100).unwrap();
///
/// // Cursor can be decoded back
/// let position = encoder.decode(&cursor).unwrap();
/// assert_eq!(position, 100);
/// ```
#[derive(Debug, Clone)]
pub struct Base64CursorEncoder {
	/// Cursor expiry time in seconds (default: 24 hours)
	pub expiry_seconds: u64,
}

impl Base64CursorEncoder {
	/// Create a new Base64 cursor encoder with default expiry (24 hours)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_pagination::cursor::Base64CursorEncoder;
	///
	/// let encoder = Base64CursorEncoder::new();
	/// assert_eq!(encoder.expiry_seconds, 86400);
	/// ```
	pub fn new() -> Self {
		Self {
			expiry_seconds: 86400, // 24 hours
		}
	}

	/// Set custom expiry time in seconds
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_pagination::cursor::Base64CursorEncoder;
	///
	/// let encoder = Base64CursorEncoder::new().expiry_seconds(3600); // 1 hour
	/// assert_eq!(encoder.expiry_seconds, 3600);
	/// ```
	pub fn expiry_seconds(mut self, seconds: u64) -> Self {
		self.expiry_seconds = seconds;
		self
	}
}

impl Default for Base64CursorEncoder {
	fn default() -> Self {
		Self::new()
	}
}

impl CursorEncoder for Base64CursorEncoder {
	fn encode(&self, position: usize) -> Result<String> {
		use base64::{Engine as _, engine::general_purpose};

		let timestamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();

		// Create checksum to prevent tampering
		let mut hasher = DefaultHasher::new();
		position.hash(&mut hasher);
		timestamp.hash(&mut hasher);
		let checksum = hasher.finish();

		let cursor_data = format!("{}:{}:{}", position, timestamp, checksum);
		Ok(general_purpose::STANDARD.encode(cursor_data.as_bytes()))
	}

	fn decode(&self, cursor: &str) -> Result<usize> {
		use base64::{Engine as _, engine::general_purpose};

		let decoded = general_purpose::STANDARD
			.decode(cursor)
			.map_err(|_| Error::InvalidPage("Invalid cursor".to_string()))?;
		let cursor_data = String::from_utf8(decoded)
			.map_err(|_| Error::InvalidPage("Invalid cursor encoding".to_string()))?;

		// Parse cursor components
		let parts: Vec<&str> = cursor_data.split(':').collect();
		if parts.len() != 3 {
			return Err(Error::InvalidPage("Malformed cursor".to_string()));
		}

		let position: usize = parts[0]
			.parse()
			.map_err(|_| Error::InvalidPage("Invalid cursor value".to_string()))?;
		let timestamp: u64 = parts[1]
			.parse()
			.map_err(|_| Error::InvalidPage("Invalid cursor timestamp".to_string()))?;
		let provided_checksum: u64 = parts[2]
			.parse()
			.map_err(|_| Error::InvalidPage("Invalid cursor checksum".to_string()))?;

		// Verify checksum
		let mut hasher = DefaultHasher::new();
		position.hash(&mut hasher);
		timestamp.hash(&mut hasher);
		let expected_checksum = hasher.finish();

		if provided_checksum != expected_checksum {
			return Err(Error::InvalidPage("Cursor checksum mismatch".to_string()));
		}

		// Check if cursor is expired
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();
		if now - timestamp > self.expiry_seconds {
			return Err(Error::Validation("Cursor expired".to_string()));
		}

		Ok(position)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_base64_encoder_encode_decode() {
		let encoder = Base64CursorEncoder::new();
		let position = 42;

		let cursor = encoder.encode(position).unwrap();
		let decoded = encoder.decode(&cursor).unwrap();

		assert_eq!(decoded, position);
	}

	#[test]
	fn test_base64_encoder_invalid_cursor() {
		let encoder = Base64CursorEncoder::new();

		let result = encoder.decode("invalid_cursor");
		assert!(result.is_err());
	}

	#[test]
	fn test_base64_encoder_tampered_cursor() {
		let encoder = Base64CursorEncoder::new();
		let cursor = encoder.encode(42).unwrap();

		// Tamper with the cursor
		let mut tampered = cursor.clone();
		tampered.push('X');

		let result = encoder.decode(&tampered);
		assert!(result.is_err());
	}

	#[test]
	fn test_base64_encoder_custom_expiry() {
		let encoder = Base64CursorEncoder::new().expiry_seconds(1);
		let cursor = encoder.encode(42).unwrap();

		// Wait for expiry
		std::thread::sleep(std::time::Duration::from_secs(2));

		let result = encoder.decode(&cursor);
		assert!(result.is_err());
		if let Err(Error::Validation(msg)) = result {
			assert_eq!(msg, "Cursor expired");
		} else {
			panic!("Expected Validation error");
		}
	}
}
