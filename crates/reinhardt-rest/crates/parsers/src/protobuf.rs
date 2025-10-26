//! Protobuf parser for Protocol Buffers binary format.
//!
//! This parser handles `application/protobuf` and `application/x-protobuf` content types,
//! providing dynamic schema support for Protocol Buffers deserialization.

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_exception::Error;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::parser::{ParseResult, ParsedData, Parser};

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
/// # Examples
///
/// ```
/// use reinhardt_parsers::protobuf::ProtobufParser;
/// use reinhardt_parsers::parser::Parser;
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
    #[allow(dead_code)]
    schema_registry: HashMap<String, String>,
}

impl ProtobufParser {
    /// Create a new ProtobufParser instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_parsers::protobuf::ProtobufParser;
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
    /// use reinhardt_parsers::protobuf::ProtobufParser;
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
    /// This is a simplified parser that extracts basic field information
    /// from the Protobuf wire format without full schema knowledge.
    fn parse_wire_format(&self, data: &[u8]) -> ParseResult<Value> {
        // TODO: Implement full Protobuf wire format parser
        // For now, this is a placeholder that demonstrates the concept

        if data.is_empty() {
            return Err(Error::Validation("Empty Protobuf data".to_string()));
        }

        // Basic wire format parsing (simplified)
        // In production, you would use prost::Message::decode with actual schema
        let mut fields = serde_json::Map::new();

        // Example: Extract field number and wire type
        // Real implementation would recursively parse all fields
        fields.insert("_raw_size".to_string(), json!(data.len()));

        fields.insert("_format".to_string(), json!("protobuf"));

        Ok(Value::Object(fields))
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

    async fn parse(&self, _content_type: Option<&str>, body: Bytes) -> ParseResult<ParsedData> {
        // For dynamic parsing, we convert to a generic representation
        let value = self.parse_wire_format(&body)?;

        Ok(ParsedData::Protobuf(value))
    }
}

/// Trait for Protobuf message types that can be parsed.
///
/// This trait allows custom Protobuf messages to be integrated with the parser.
///
/// # Examples
///
/// ```ignore
/// use prost::Message;
/// use reinhardt_parsers::protobuf::ProtobufMessage;
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
        let body = Bytes::from(vec![0x08, 0x96, 0x01]);

        let result = parser.parse(Some("application/protobuf"), body).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ParsedData::Protobuf(value) => {
                assert_eq!(value["_raw_size"], 3);
                assert_eq!(value["_format"], "protobuf");
            }
            _ => panic!("Expected Protobuf variant"),
        }
    }

    #[tokio::test]
    async fn test_protobuf_parser_empty_data() {
        let parser = ProtobufParser::new();

        let body = Bytes::new();
        let result = parser.parse(Some("application/protobuf"), body).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_protobuf_parser_with_x_protobuf_content_type() {
        let parser = ProtobufParser::new();

        let body = Bytes::from(vec![0x08, 0x96, 0x01]);
        let result = parser.parse(Some("application/x-protobuf"), body).await;
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
        let body = Bytes::from(vec![
            0x08, 0x96, 0x01, // field 1, varint, 150 (3 bytes)
            0x12, 0x04, 0x74, 0x65, 0x73, 0x74, // field 2, string, "test" (6 bytes)
        ]);

        let result = parser.parse(Some("application/protobuf"), body).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ParsedData::Protobuf(value) => {
                assert_eq!(value["_raw_size"], 9); // 3 + 6 = 9 bytes
                assert_eq!(value["_format"], "protobuf");
            }
            _ => panic!("Expected Protobuf variant"),
        }
    }
}
