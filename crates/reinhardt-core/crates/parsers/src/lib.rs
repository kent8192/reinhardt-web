//! # Reinhardt Parsers
//!
//! Request body parsers for the Reinhardt framework, inspired by Django REST Framework.
//!
//! ## Parsers
//!
//! - **JSONParser**: Parse JSON request bodies
//! - **XMLParser**: Parse XML request bodies (application/xml, text/xml)
//! - **YamlParser**: Parse YAML request bodies (application/yaml, application/x-yaml)
//! - **FormParser**: Parse HTML form data (application/x-www-form-urlencoded)
//! - **MultiPartParser**: Handle file uploads (multipart/form-data)
//! - **FileUploadParser**: Raw file upload handling
//! - **CompressedParser**: Wrapper for transparent decompression (gzip, brotli, deflate)
//! - **MessagePackParser**: Parse MessagePack binary format (application/msgpack)
//! - **ProtobufParser**: Parse Protocol Buffers with dynamic schema support (application/protobuf)
//! - **StreamingParser**: Memory-efficient parsing for large uploads
//!
//! ## Validation
//!
//! - **ParserValidator**: Trait for custom validation hooks (before_parse, after_parse)
//! - **SizeLimitValidator**: Enforce maximum body size limits
//! - **ContentTypeValidator**: Validate required content types
//! - **CompositeValidator**: Combine multiple validators
//!
//! ## Example
//!
//! ```rust,no_run
//! use reinhardt_parsers::{JSONParser, Parser};
//! use bytes::Bytes;
//! use http::HeaderMap;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let parser = JSONParser::new();
//! let body = Bytes::from(r#"{"key":"value"}"#);
//! let headers = HeaderMap::new();
//! let data = parser.parse(Some("application/json"), body, &headers).await?;
//! # Ok(())
//! # }
//! ```

pub mod compressed;
pub mod file;
pub mod form;
pub mod json;
pub mod msgpack;
pub mod multipart;
pub mod parser;
pub mod protobuf;
pub mod streaming;
pub mod validator;
pub mod xml;
pub mod yaml;

pub use compressed::{CompressedParser, CompressionEncoding};
pub use file::FileUploadParser;
pub use form::FormParser;
pub use json::JSONParser;
pub use msgpack::MessagePackParser;
pub use multipart::MultiPartParser;
pub use parser::{MediaType, ParseError, ParseResult, Parser};
pub use protobuf::{ProtobufMessage, ProtobufParser};
pub use streaming::{StreamChunk, StreamingParser};
pub use validator::{
	CompositeValidator, ContentTypeValidator, ParserValidator, SizeLimitValidator,
};
pub use xml::{XMLParser, XmlParserConfig, XmlParserConfigBuilder};
pub use yaml::YamlParser;
