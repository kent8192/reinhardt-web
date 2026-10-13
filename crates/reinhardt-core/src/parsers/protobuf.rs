//! Protobuf parser for Protocol Buffers binary format.
//!
//! This parser handles `application/protobuf` and `application/x-protobuf` content types,
//! providing dynamic schema support for Protocol Buffers deserialization.

use crate::exception::Error;
use async_trait::async_trait;
use bytes::Bytes;
use http::HeaderMap;
use serde_json::{Value, json};
use std::collections::HashMap;

use super::parser::{ParseResult, ParsedData, Parser};

/// Maximum nesting depth of length-delimited fields accepted by [`ProtobufParser`].
///
/// Without a schema, every length-delimited field is speculatively decoded as
/// an embedded message, so the nesting of untrusted input maps directly onto
/// native recursion. Each level costs only a few bytes on the wire, so the body
/// size limit alone does not bound the stack depth. The value matches the
/// default limit of `reinhardt-grpc`'s `DepthLimitedDecoder`.
const MAX_NESTING_DEPTH: usize = 64;

/// Failure of the internal wire-format decoder.
#[derive(Debug)]
enum WireFormatError {
	/// The bytes are not a well-formed message. A length-delimited field whose
	/// contents fail this way is exposed as a string or bytes instead.
	Malformed(Error),
	/// Length-delimited fields nest deeper than [`MAX_NESTING_DEPTH`]. Unlike
	/// [`WireFormatError::Malformed`], this rejects the whole body instead of
	/// degrading the offending field to a string or bytes.
	NestingTooDeep,
}

impl From<Error> for WireFormatError {
	fn from(error: Error) -> Self {
		Self::Malformed(error)
	}
}

impl From<WireFormatError> for Error {
	fn from(error: WireFormatError) -> Self {
		match error {
			WireFormatError::Malformed(error) => error,
			WireFormatError::NestingTooDeep => Error::Validation(format!(
				"Protobuf message nesting exceeds the maximum depth of {MAX_NESTING_DEPTH}"
			)),
		}
	}
}

/// Parser for Protocol Buffers binary format.
///
/// This parser provides dynamic schema support for deserializing Protobuf messages
/// into JSON-compatible structures. Since Protobuf requires schema information,
/// this implementation converts the binary data into a generic representation.
///
/// # Supported Content Types
///
/// - `application/protobuf`
/// - `application/x-protobuf`
///
/// # Note
///
/// This is a basic implementation that handles Protobuf wire format parsing.
/// For full schema-aware deserialization, you should use the generated code
/// from your `.proto` files with `prost::Message::decode()`.
///
/// # Nesting Limit
///
/// Length-delimited fields may nest at most 64 levels deep; deeper bodies are
/// rejected with [`Error::Validation`]. Because the parser cannot tell embedded
/// messages from strings or bytes without a schema, every length-delimited
/// field counts as a level, including a string or bytes field inside the
/// 64th nested message.
///
/// # Examples
///
/// ```
/// use reinhardt_core::parsers::protobuf::ProtobufParser;
/// use reinhardt_core::parsers::parser::Parser;
/// use bytes::Bytes;
///
/// # tokio_test::block_on(async {
/// let parser = ProtobufParser::new();
///
/// // For actual use, you would decode with a specific message type
/// // let message = MyProtoMessage::decode(body)?;
/// # });
/// ```
#[derive(Debug, Clone, Default)]
pub struct ProtobufParser {
	/// Optional schema registry for type resolution
	// Allow dead_code: schema_registry stored for future schema-aware deserialization; currently only dynamic wire-format parsing is implemented
	#[allow(dead_code)]
	schema_registry: HashMap<String, String>,
}

impl ProtobufParser {
	/// Create a new ProtobufParser instance.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::protobuf::ProtobufParser;
	///
	/// let parser = ProtobufParser::new();
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Create a ProtobufParser with a schema registry.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::protobuf::ProtobufParser;
	/// use std::collections::HashMap;
	///
	/// let mut schemas = HashMap::new();
	/// schemas.insert("User".to_string(), "user.proto".to_string());
	///
	/// let parser = ProtobufParser::with_schema_registry(schemas);
	/// ```
	pub fn with_schema_registry(schema_registry: HashMap<String, String>) -> Self {
		Self { schema_registry }
	}

	/// Parse Protobuf wire format into a generic JSON representation.
	///
	/// This parser extracts field information from the Protobuf wire format
	/// without requiring full schema knowledge. It infers types from wire types.
	///
	/// # Wire Format
	///
	/// Each field consists of:
	/// - Tag (varint): (field_number << 3) | wire_type
	/// - Value (depends on wire_type)
	///
	/// Wire types:
	/// - 0: Varint (int32, int64, uint32, uint64, sint32, sint64, bool, enum)
	/// - 1: 64-bit (fixed64, sfixed64, double)
	/// - 2: Length-delimited (string, bytes, embedded messages, packed repeated fields)
	/// - 3: Start group (deprecated)
	/// - 4: End group (deprecated)
	/// - 5: 32-bit (fixed32, sfixed32, float)
	///
	/// `depth` is the nesting level of `data`: `0` for the request body and one
	/// more for each enclosing length-delimited field. A length-delimited field
	/// found at [`MAX_NESTING_DEPTH`] fails with
	/// [`WireFormatError::NestingTooDeep`] before any further recursion.
	fn parse_wire_format(&self, data: &[u8], depth: usize) -> Result<Value, WireFormatError> {
		if data.is_empty() {
			return Err(Error::Validation("Empty Protobuf data".to_string()).into());
		}

		let mut fields = serde_json::Map::new();
		let mut cursor = 0;

		while cursor < data.len() {
			// Parse tag (field_number << 3 | wire_type)
			let (tag, bytes_read) = self.decode_varint(&data[cursor..])?;
			cursor += bytes_read;

			let field_number = (tag >> 3) as u32;
			let wire_type = (tag & 0x7) as u8;

			// Parse value based on wire type
			let (value, bytes_consumed) = match wire_type {
				0 => {
					// Varint
					let (v, n) = self.decode_varint(&data[cursor..])?;
					(json!(v), n)
				}
				1 => {
					// 64-bit
					if cursor + 8 > data.len() {
						return Err(Error::Validation(
							"Insufficient data for 64-bit field".to_string(),
						)
						.into());
					}
					let bytes: [u8; 8] = data[cursor..cursor + 8].try_into().unwrap();
					let value = u64::from_le_bytes(bytes);
					(json!(value), 8)
				}
				2 => {
					// Length-delimited
					let (len, n) = self.decode_varint(&data[cursor..])?;
					cursor += n;

					if cursor + len as usize > data.len() {
						return Err(Error::Validation(
							"Insufficient data for length-delimited field".to_string(),
						)
						.into());
					}

					let field_data = &data[cursor..cursor + len as usize];

					// Check before descending: the speculative nested parse
					// below recurses once per level of untrusted nesting.
					if depth >= MAX_NESTING_DEPTH {
						return Err(WireFormatError::NestingTooDeep);
					}

					// Try to parse as nested message first
					let value = match self.parse_wire_format(field_data, depth + 1) {
						Ok(nested) => nested,
						// Propagate instead of falling back to a string, which
						// would accept the over-deep body.
						Err(WireFormatError::NestingTooDeep) => {
							return Err(WireFormatError::NestingTooDeep);
						}
						Err(WireFormatError::Malformed(_)) => {
							// If parsing as message fails, treat as string or bytes
							match std::str::from_utf8(field_data) {
								Ok(s) => json!(s),
								Err(_) => json!(field_data.to_vec()),
							}
						}
					};

					(value, len as usize)
				}
				3 | 4 => {
					// Start/End group (deprecated)
					return Err(Error::Validation(
						"Group wire types are deprecated and not supported".to_string(),
					)
					.into());
				}
				5 => {
					// 32-bit
					if cursor + 4 > data.len() {
						return Err(Error::Validation(
							"Insufficient data for 32-bit field".to_string(),
						)
						.into());
					}
					let bytes: [u8; 4] = data[cursor..cursor + 4].try_into().unwrap();
					let value = u32::from_le_bytes(bytes);
					(json!(value), 4)
				}
				_ => {
					return Err(
						Error::Validation(format!("Unknown wire type: {}", wire_type)).into(),
					);
				}
			};

			cursor += bytes_consumed;

			// Store field with field number as key
			let field_key = field_number.to_string();

			// Handle repeated fields
			if let Some(existing) = fields.get(&field_key) {
				let repeated = if let Some(arr) = existing.as_array() {
					let mut new_arr = arr.clone();
					new_arr.push(value);
					json!(new_arr)
				} else {
					json!([existing.clone(), value])
				};
				fields.insert(field_key, repeated);
			} else {
				fields.insert(field_key, value);
			}
		}

		Ok(Value::Object(fields))
	}

	/// Decode a varint from bytes.
	///
	/// Returns the decoded value and the number of bytes consumed.
	fn decode_varint(&self, data: &[u8]) -> ParseResult<(u64, usize)> {
		let mut result: u64 = 0;
		let mut shift = 0;

		for (i, &byte) in data.iter().enumerate() {
			if i > 9 {
				// Varints are at most 10 bytes
				return Err(Error::Validation("Varint too long".to_string()));
			}

			result |= ((byte & 0x7F) as u64) << shift;

			if byte & 0x80 == 0 {
				// MSB is 0, varint complete
				return Ok((result, i + 1));
			}

			shift += 7;
		}

		Err(Error::Validation(
			"Incomplete varint at end of data".to_string(),
		))
	}
}

#[async_trait]
impl Parser for ProtobufParser {
	fn media_types(&self) -> Vec<String> {
		vec![
			"application/protobuf".to_string(),
			"application/x-protobuf".to_string(),
		]
	}

	async fn parse(
		&self,
		_content_type: Option<&str>,
		body: Bytes,
		_headers: &HeaderMap,
	) -> ParseResult<ParsedData> {
		// For dynamic parsing, we convert to a generic representation
		let value = self.parse_wire_format(&body, 0)?;

		Ok(ParsedData::Protobuf(value))
	}
}

/// Trait for Protobuf message types that can be parsed.
///
/// This trait allows custom Protobuf messages to be integrated with the parser.
///
/// # Examples
///
/// ```rust,no_run
/// use prost::Message;
/// use reinhardt_core::parsers::protobuf::ProtobufMessage;
///
/// #[derive(Message)]
/// struct User {
///     #[prost(string, tag = "1")]
///     name: String,
///     #[prost(int32, tag = "2")]
///     age: i32,
/// }
///
/// impl ProtobufMessage for User {
///     fn decode_from_bytes(data: &[u8]) -> Result<Self, prost::DecodeError> {
///         User::decode(data)
///     }
/// }
/// ```
pub trait ProtobufMessage: Sized {
	/// Decode a message from bytes.
	fn decode_from_bytes(data: &[u8]) -> Result<Self, prost::DecodeError>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	/// Appends `value` to `out` as a Protobuf base-128 varint.
	fn encode_varint(mut value: usize, out: &mut Vec<u8>) {
		while value >= 0x80 {
			out.push((value as u8 & 0x7F) | 0x80);
			value >>= 7;
		}
		out.push(value as u8);
	}

	/// Builds a body whose innermost message `{1: 1}` is wrapped in `levels`
	/// nested length-delimited fields, each numbered 1.
	fn nested_message(levels: usize) -> Bytes {
		// Built back to front so that each wrap appends instead of copying
		// the whole message, keeping very deep bodies linear to construct.
		let mut reversed = vec![0x01, 0x08];
		let mut prefix = Vec::with_capacity(11);
		for _ in 0..levels {
			prefix.clear();
			prefix.push(0x0A);
			encode_varint(reversed.len(), &mut prefix);
			reversed.extend(prefix.iter().rev());
		}
		reversed.reverse();
		Bytes::from(reversed)
	}

	#[tokio::test]
	async fn test_protobuf_parser_media_types() {
		let parser = ProtobufParser::new();
		let media_types = parser.media_types();

		assert_eq!(media_types.len(), 2);
		assert!(media_types.contains(&"application/protobuf".to_string()));
		assert!(media_types.contains(&"application/x-protobuf".to_string()));
	}

	#[tokio::test]
	async fn test_protobuf_parser_can_parse() {
		let parser = ProtobufParser::new();

		assert!(parser.can_parse(Some("application/protobuf")));
		assert!(parser.can_parse(Some("application/x-protobuf")));
		assert!(!parser.can_parse(Some("application/json")));
		assert!(!parser.can_parse(None));
	}

	#[tokio::test]
	async fn test_protobuf_parser_with_data() {
		let parser = ProtobufParser::new();

		// Simple Protobuf wire format data (field 1, varint, value 150)
		// Wire format: [field_number << 3 | wire_type, value]
		// 0x08 = field 1, wire type 0 (varint)
		// 0x96 0x01 = varint 150 (base-128 encoding: 0x96 = 150 - 128 = 22, 0x01 = 1 * 128 = 128, total = 22 + 128 = 150)
		let body = Bytes::from(vec![0x08, 0x96, 0x01]);
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert_eq!(value["1"], 150);
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[tokio::test]
	async fn test_protobuf_parser_empty_data() {
		let parser = ProtobufParser::new();

		let body = Bytes::new();
		let headers = HeaderMap::new();
		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_with_x_protobuf_content_type() {
		let parser = ProtobufParser::new();

		let body = Bytes::from(vec![0x08, 0x96, 0x01]);
		let headers = HeaderMap::new();
		let result = parser
			.parse(Some("application/x-protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_protobuf_parser_with_schema_registry() {
		let mut schemas = HashMap::new();
		schemas.insert("User".to_string(), "user.proto".to_string());

		let parser = ProtobufParser::with_schema_registry(schemas);
		assert_eq!(parser.schema_registry.len(), 1);
		assert_eq!(
			parser.schema_registry.get("User"),
			Some(&"user.proto".to_string())
		);
	}

	#[tokio::test]
	async fn test_protobuf_parser_larger_message() {
		let parser = ProtobufParser::new();

		// Multiple fields in wire format
		// field 1: varint 150
		// field 2: string "test"
		let body = Bytes::from(vec![
			0x08, 0x96, 0x01, // field 1, wire type 0 (varint), value 150
			0x12, 0x04, 0x74, 0x65, 0x73,
			0x74, // field 2, wire type 2 (length-delimited), length 4, "test"
		]);
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert_eq!(value["1"], 150);
				assert_eq!(value["2"], "test");
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[tokio::test]
	async fn test_protobuf_parser_64bit_field() {
		let parser = ProtobufParser::new();

		// field 1: fixed64 value 0x123456789ABCDEF0
		let body = Bytes::from(vec![
			0x09, // field 1, wire type 1 (64-bit)
			0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12, // little-endian 64-bit value
		]);
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert_eq!(value["1"], 0x123456789ABCDEF0u64);
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[tokio::test]
	async fn test_protobuf_parser_32bit_field() {
		let parser = ProtobufParser::new();

		// field 1: fixed32 value 0x12345678
		let body = Bytes::from(vec![
			0x0D, // field 1, wire type 5 (32-bit)
			0x78, 0x56, 0x34, 0x12, // little-endian 32-bit value
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert_eq!(value["1"], 0x12345678u32);
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[tokio::test]
	async fn test_protobuf_parser_nested_message() {
		let parser = ProtobufParser::new();

		// field 1: varint 42
		// field 2: nested message with field 1: varint 100
		let body = Bytes::from(vec![
			0x08, 0x2A, // field 1, varint 42
			0x12, 0x02, 0x08, 0x64, // field 2, length 2, nested message (field 1, varint 100)
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert_eq!(value["1"], 42);
				assert!(value["2"].is_object());
				assert_eq!(value["2"]["1"], 100);
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[tokio::test]
	async fn test_protobuf_parser_repeated_field() {
		let parser = ProtobufParser::new();

		// field 1: varint 10, varint 20, varint 30 (repeated)
		let body = Bytes::from(vec![
			0x08, 0x0A, // field 1, varint 10
			0x08, 0x14, // field 1, varint 20
			0x08, 0x1E, // field 1, varint 30
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert!(value["1"].is_array());
				let arr = value["1"].as_array().unwrap();
				assert_eq!(arr.len(), 3);
				assert_eq!(arr[0], 10);
				assert_eq!(arr[1], 20);
				assert_eq!(arr[2], 30);
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[tokio::test]
	async fn test_protobuf_parser_bytes_field() {
		let parser = ProtobufParser::new();

		// field 1: bytes [0xFF, 0xFE, 0xFD] (non-UTF8)
		let body = Bytes::from(vec![
			0x0A, 0x03, 0xFF, 0xFE, 0xFD, // field 1, length 3, bytes
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert!(value["1"].is_array());
				let arr = value["1"].as_array().unwrap();
				assert_eq!(arr, &[255, 254, 253]);
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[tokio::test]
	async fn test_protobuf_parser_unknown_wire_type() {
		let parser = ProtobufParser::new();

		// Invalid wire type 6
		let body = Bytes::from(vec![0x0E]); // field 1, wire type 6

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_deprecated_group() {
		let parser = ProtobufParser::new();

		// Wire type 3 (start group) - deprecated
		let body = Bytes::from(vec![0x0B]); // field 1, wire type 3

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_incomplete_varint() {
		let parser = ProtobufParser::new();

		// Incomplete varint (all bytes have MSB set)
		let body = Bytes::from(vec![0x08, 0xFF, 0xFF]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_insufficient_64bit_data() {
		let parser = ProtobufParser::new();

		// 64-bit field but only 7 bytes
		let body = Bytes::from(vec![
			0x09, // field 1, wire type 1 (64-bit)
			0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_insufficient_32bit_data() {
		let parser = ProtobufParser::new();

		// 32-bit field but only 3 bytes
		let body = Bytes::from(vec![
			0x0D, // field 1, wire type 5 (32-bit)
			0x01, 0x02, 0x03,
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_insufficient_length_delimited_data() {
		let parser = ProtobufParser::new();

		// Length-delimited field claims 10 bytes but only 5 available
		let body = Bytes::from(vec![
			0x0A, 0x0A, // field 1, length 10
			0x01, 0x02, 0x03, 0x04, 0x05,
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_varint_too_long() {
		let parser = ProtobufParser::new();

		// Varint with 11 bytes (max is 10)
		let body = Bytes::from(vec![
			0x08, // field 1, wire type 0
			0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_protobuf_parser_complex_message() {
		let parser = ProtobufParser::new();

		// Complex message:
		// field 1: varint 42
		// field 2: string "hello"
		// field 3: fixed64 0x1234567890ABCDEF
		// field 4: nested message (field 1: varint 100, field 2: string "world")
		// field 5: repeated varint [1, 2, 3]
		let body = Bytes::from(vec![
			0x08, 0x2A, // field 1, varint 42
			0x12, 0x05, 0x68, 0x65, 0x6C, 0x6C, 0x6F, // field 2, string "hello"
			0x19, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12, // field 3, fixed64
			0x22, 0x09, 0x08, 0x64, 0x12, 0x05, 0x77, 0x6F, 0x72, 0x6C,
			0x64, // field 4, nested (length 9)
			0x28, 0x01, // field 5, varint 1
			0x28, 0x02, // field 5, varint 2
			0x28, 0x03, // field 5, varint 3
		]);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;
		assert!(result.is_ok());

		match result.unwrap() {
			ParsedData::Protobuf(value) => {
				assert_eq!(value["1"], 42);
				assert_eq!(value["2"], "hello");
				assert_eq!(value["3"], 0x1234567890ABCDEFu64);
				assert!(value["4"].is_object());
				assert_eq!(value["4"]["1"], 100);
				assert_eq!(value["4"]["2"], "world");
				assert!(value["5"].is_array());
				let arr = value["5"].as_array().unwrap();
				assert_eq!(arr.len(), 3);
				assert_eq!(arr[0], 1);
				assert_eq!(arr[1], 2);
				assert_eq!(arr[2], 3);
			}
			_ => panic!("Expected Protobuf variant"),
		}
	}

	#[rstest]
	#[case::single_level(1)]
	#[case::at_limit(64)]
	#[tokio::test]
	async fn test_protobuf_parser_accepts_nesting_within_depth_limit(#[case] levels: usize) {
		// Arrange
		let parser = ProtobufParser::new();
		let body = nested_message(levels);
		let headers = HeaderMap::new();

		// Act
		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;

		// Assert
		let ParsedData::Protobuf(value) = result.expect("nesting within the limit must parse")
		else {
			panic!("Expected Protobuf variant");
		};
		let innermost_field = "/1".repeat(levels + 1);
		assert_eq!(value.pointer(&innermost_field), Some(&json!(1)));
	}

	#[rstest]
	#[case::one_past_limit(65)]
	// Deep enough to overflow the stack if the recursion were unbounded.
	#[case::stack_exhausting(100_000)]
	#[tokio::test]
	async fn test_protobuf_parser_rejects_nesting_beyond_depth_limit(#[case] levels: usize) {
		// Arrange
		let parser = ProtobufParser::new();
		let body = nested_message(levels);
		let headers = HeaderMap::new();

		// Act
		let result = parser
			.parse(Some("application/protobuf"), body, &headers)
			.await;

		// Assert
		match result {
			Err(Error::Validation(message)) => assert_eq!(
				message,
				"Protobuf message nesting exceeds the maximum depth of 64"
			),
			other => panic!("Expected nesting validation error, got {other:?}"),
		}
	}
}
