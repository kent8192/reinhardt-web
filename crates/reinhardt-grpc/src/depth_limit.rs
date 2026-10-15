//! Protobuf message nesting depth limits
//!
//! This module provides a configurable depth limit for protobuf message
//! parsing to prevent stack overflow attacks from deeply nested messages.
//!
//! # Security
//!
//! Without depth limits, an attacker can craft a protobuf message with
//! extreme nesting (e.g., thousands of levels) causing stack overflow
//! during deserialization. This module provides [`DepthLimitedDecoder`]
//! to enforce a configurable maximum nesting depth.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_grpc::depth_limit::DepthLimitedDecoder;
//! use prost::Message;
//!
//! // Create a decoder with default depth limit (64 levels)
//! let decoder = DepthLimitedDecoder::default();
//! assert_eq!(decoder.max_depth(), 64);
//!
//! // Create a decoder with custom depth limit
//! let decoder = DepthLimitedDecoder::new(128);
//! assert_eq!(decoder.max_depth(), 128);
//! ```

/// Default maximum nesting depth for protobuf messages.
///
/// 64 levels is sufficient for legitimate use cases while preventing
/// stack overflow from maliciously crafted deeply nested messages.
const DEFAULT_MAX_NESTING_DEPTH: u32 = 64;

/// Decoder that enforces a maximum nesting depth for protobuf messages.
///
/// Wraps `prost::Message::decode` with an additional depth check
/// by scanning the wire format for nested message fields before
/// full deserialization.
///
/// # Example
///
/// ```rust
/// use reinhardt_grpc::depth_limit::DepthLimitedDecoder;
///
/// let decoder = DepthLimitedDecoder::new(32);
///
/// // Decode a simple (non-nested) message
/// let data: &[u8] = &[];
/// let result = decoder.decode::<reinhardt_grpc::proto::common::Empty>(data);
/// assert!(result.is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct DepthLimitedDecoder {
	max_depth: u32,
}

impl DepthLimitedDecoder {
	/// Create a new decoder with the specified maximum nesting depth.
	pub fn new(max_depth: u32) -> Self {
		Self { max_depth }
	}

	/// Returns the configured maximum nesting depth.
	pub fn max_depth(&self) -> u32 {
		self.max_depth
	}

	/// Decode a protobuf message with depth limit enforcement.
	///
	/// First scans the wire-format bytes to measure nesting depth,
	/// then decodes the message if the depth is within limits.
	///
	/// # Errors
	///
	/// Returns [`DepthLimitError::ExceededMaxDepth`] if the message
	/// nesting exceeds the configured limit, or [`DepthLimitError::DecodeError`]
	/// if the message fails to decode.
	pub fn decode<M: prost::Message + Default>(&self, buf: &[u8]) -> Result<M, DepthLimitError> {
		let measured_depth = measure_wire_depth(buf, self.max_depth);
		if measured_depth > self.max_depth {
			tracing::warn!(
				measured_depth = measured_depth,
				max_depth = self.max_depth,
				"Protobuf message nesting depth exceeded limit"
			);
			return Err(DepthLimitError::ExceededMaxDepth {
				depth: measured_depth,
				limit: self.max_depth,
			});
		}
		M::decode(buf).map_err(DepthLimitError::DecodeError)
	}
}

impl Default for DepthLimitedDecoder {
	fn default() -> Self {
		Self {
			max_depth: DEFAULT_MAX_NESTING_DEPTH,
		}
	}
}

/// Errors that can occur during depth-limited decoding.
#[derive(Debug, thiserror::Error)]
pub enum DepthLimitError {
	/// The message nesting depth exceeded the configured limit.
	#[error("protobuf nesting depth {depth} exceeds limit of {limit}")]
	ExceededMaxDepth {
		/// The measured nesting depth.
		depth: u32,
		/// The configured maximum depth.
		limit: u32,
	},

	/// The message failed to decode.
	#[error("protobuf decode error: {0}")]
	DecodeError(#[from] prost::DecodeError),
}

/// Measure the maximum nesting depth in a protobuf wire-format byte slice.
///
/// Scans for length-delimited fields (wire type 2) and tracks how deeply
/// they nest. Returns the maximum depth observed.
///
/// This is a best-effort scan that works on valid protobuf encoding.
/// For malformed data, it returns a conservative estimate.
fn measure_wire_depth(buf: &[u8], max_allowed_depth: u32) -> u32 {
	if buf.is_empty() {
		return 0;
	}
	let mut max_depth = 0;
	let mut pending = vec![(buf, 0usize, 0u32)];

	while let Some((buf, mut pos, depth)) = pending.pop() {
		while pos < buf.len() {
			let (tag, bytes_read) = match decode_varint(&buf[pos..]) {
				Some(value) => value,
				None => break,
			};
			pos += bytes_read;

			match (tag & 0x07) as u8 {
				0 => match decode_varint(&buf[pos..]) {
					Some((_, bytes_read)) => pos += bytes_read,
					None => break,
				},
				1 => match pos.checked_add(8).filter(|end| *end <= buf.len()) {
					Some(end) => pos = end,
					None => break,
				},
				2 => {
					let (length, bytes_read) = match decode_varint(&buf[pos..]) {
						Some(value) => value,
						None => break,
					};
					pos += bytes_read;
					let end = match usize::try_from(length)
						.ok()
						.and_then(|length| pos.checked_add(length))
						.filter(|end| *end <= buf.len())
					{
						Some(end) => end,
						None => break,
					};

					let child_depth = depth.saturating_add(1);
					max_depth = max_depth.max(child_depth);
					if max_depth > max_allowed_depth {
						return max_depth;
					}
					pending.push((buf, end, depth));
					pending.push((&buf[pos..end], 0, child_depth));
					break;
				}
				3 | 4 => break,
				5 => match pos.checked_add(4).filter(|end| *end <= buf.len()) {
					Some(end) => pos = end,
					None => break,
				},
				_ => break,
			}
		}
	}

	max_depth
}

/// Decode a protobuf varint from the buffer.
///
/// Returns the decoded value and the number of bytes consumed, or `None`
/// if the buffer is too short or the varint is malformed.
fn decode_varint(buf: &[u8]) -> Option<(u64, usize)> {
	let mut value: u64 = 0;
	let mut shift = 0u32;

	for (i, &byte) in buf.iter().enumerate() {
		if shift >= 64 {
			return None;
		}
		value |= ((byte & 0x7F) as u64) << shift;
		if byte & 0x80 == 0 {
			return Some((value, i + 1));
		}
		shift += 7;
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use prost::Message;
	use rstest::rstest;

	#[rstest]
	fn default_depth_limit_is_64() {
		// Arrange & Act
		let decoder = DepthLimitedDecoder::default();

		// Assert
		assert_eq!(decoder.max_depth(), DEFAULT_MAX_NESTING_DEPTH);
		assert_eq!(decoder.max_depth(), 64);
	}

	#[rstest]
	#[case(1)]
	#[case(32)]
	#[case(64)]
	#[case(128)]
	#[case(256)]
	fn custom_depth_limit(#[case] limit: u32) {
		// Arrange & Act
		let decoder = DepthLimitedDecoder::new(limit);

		// Assert
		assert_eq!(decoder.max_depth(), limit);
	}

	#[rstest]
	fn decode_empty_message_succeeds() {
		// Arrange
		let decoder = DepthLimitedDecoder::default();
		let empty = crate::proto::common::Empty {};
		let encoded = empty.encode_to_vec();

		// Act
		let result = decoder.decode::<crate::proto::common::Empty>(&encoded);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn decode_simple_message_succeeds() {
		// Arrange
		let decoder = DepthLimitedDecoder::default();
		let timestamp = crate::proto::common::Timestamp {
			seconds: 1_000_000,
			nanos: 500_000,
		};
		let encoded = timestamp.encode_to_vec();

		// Act
		let result = decoder.decode::<crate::proto::common::Timestamp>(&encoded);

		// Assert
		assert!(result.is_ok());
		let decoded = result.unwrap();
		assert_eq!(decoded.seconds, 1_000_000);
		assert_eq!(decoded.nanos, 500_000);
	}

	#[rstest]
	fn decode_nested_message_within_limit_succeeds() {
		// Arrange
		let decoder = DepthLimitedDecoder::new(10);
		let event = crate::proto::graphql::SubscriptionEvent {
			id: "test".to_string(),
			event_type: "update".to_string(),
			payload: Some(crate::proto::graphql::GraphQlResponse {
				data: Some("{}".to_string()),
				errors: vec![],
				extensions: None,
			}),
			timestamp: Some(crate::proto::common::Timestamp {
				seconds: 100,
				nanos: 0,
			}),
		};
		let encoded = event.encode_to_vec();

		// Act
		let result = decoder.decode::<crate::proto::graphql::SubscriptionEvent>(&encoded);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn decode_rejects_message_exceeding_depth_limit() {
		// Arrange: Create a decoder with depth limit of 1
		let decoder = DepthLimitedDecoder::new(0);

		// Nested message: a BatchResult with errors (nesting depth >= 1)
		let batch = crate::proto::common::BatchResult {
			success_count: 1,
			failure_count: 1,
			errors: vec![crate::proto::common::Error {
				code: "500".to_string(),
				message: "fail".to_string(),
				metadata: Default::default(),
			}],
		};
		let encoded = batch.encode_to_vec();

		// Act
		let result = decoder.decode::<crate::proto::common::BatchResult>(&encoded);

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(
			matches!(err, DepthLimitError::ExceededMaxDepth { .. }),
			"Expected ExceededMaxDepth error, got: {err:?}"
		);
	}

	#[rstest]
	fn depth_limit_error_display_message() {
		// Arrange
		let error = DepthLimitError::ExceededMaxDepth {
			depth: 100,
			limit: 64,
		};

		// Act
		let message = error.to_string();

		// Assert
		assert_eq!(message, "protobuf nesting depth 100 exceeds limit of 64");
	}

	#[rstest]
	fn measure_empty_buffer_returns_zero() {
		// Arrange
		let buf: &[u8] = &[];

		// Act
		let depth = measure_wire_depth(buf, DEFAULT_MAX_NESTING_DEPTH);

		// Assert
		assert_eq!(depth, 0);
	}

	#[rstest]
	fn deeply_nested_unknown_fields_stop_at_configured_limit() {
		let decoder = DepthLimitedDecoder::new(32);
		let mut encoded = vec![0x08, 0x01];
		for _ in 0..2_048 {
			let mut wrapper = vec![0x0a];
			prost::encoding::encode_varint(encoded.len() as u64, &mut wrapper);
			wrapper.extend_from_slice(&encoded);
			encoded = wrapper;
		}

		assert!(matches!(
			decoder.decode::<crate::proto::common::Empty>(&encoded),
			Err(DepthLimitError::ExceededMaxDepth {
				depth: 33,
				limit: 32
			})
		));
	}

	#[rstest]
	fn oversized_length_returns_decode_error_without_panicking() {
		let decoder = DepthLimitedDecoder::default();
		let mut encoded = vec![0x0a];
		prost::encoding::encode_varint(u64::MAX, &mut encoded);

		assert!(matches!(
			decoder.decode::<crate::proto::common::Empty>(&encoded),
			Err(DepthLimitError::DecodeError(_))
		));
	}

	#[rstest]
	// Field 1, length u64::MAX: the 11-byte body overflows `pos + length`.
	#[case::max_length_at_end(vec![
		0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
	])]
	// Field 1 varint 1, then field 2 with length u64::MAX and a trailing byte.
	#[case::max_length_mid_message(vec![
		0x08, 0x01, 0x12, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00,
	])]
	// Field 1, length u64::MAX - 10: `pos + length` wraps to exactly zero.
	#[case::length_wrapping_to_zero(vec![
		0x0A, 0xF5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
	])]
	fn decode_overflowing_length_returns_decode_error(#[case] buf: Vec<u8>) {
		// Arrange
		let decoder = DepthLimitedDecoder::default();

		// Act
		let result = decoder.decode::<crate::proto::common::Empty>(&buf);

		// Assert
		assert!(
			matches!(result, Err(DepthLimitError::DecodeError(_))),
			"Expected DecodeError, got: {result:?}"
		);
	}

	#[rstest]
	fn decode_nested_overflowing_length_is_skipped_as_unknown_field() {
		// Arrange: field 1 wraps the 11-byte overflowing payload as its contents
		let decoder = DepthLimitedDecoder::default();
		let buf: &[u8] = &[
			0x0A, 0x0B, 0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
		];

		// Act
		let depth = measure_wire_depth(buf, DEFAULT_MAX_NESTING_DEPTH);
		let result = decoder.decode::<crate::proto::common::Empty>(buf);

		// Assert
		assert_eq!(depth, 1);
		assert_eq!(result.unwrap(), crate::proto::common::Empty {});
	}

	#[rstest]
	fn decode_counts_length_reaching_end_of_buffer() {
		// Arrange: field 1, length 2, exactly the remaining nested message bytes
		let decoder = DepthLimitedDecoder::new(0);
		let buf: &[u8] = &[0x0A, 0x02, 0x08, 0x01];

		// Act
		let result = decoder.decode::<crate::proto::common::Empty>(buf);

		// Assert
		assert!(
			matches!(
				result,
				Err(DepthLimitError::ExceededMaxDepth { depth: 1, limit: 0 })
			),
			"Expected ExceededMaxDepth, got: {result:?}"
		);
	}

	#[rstest]
	fn decoder_clone_preserves_limit() {
		// Arrange
		let decoder = DepthLimitedDecoder::new(42);

		// Act
		let cloned = decoder.clone();

		// Assert
		assert_eq!(cloned.max_depth(), 42);
	}
}
