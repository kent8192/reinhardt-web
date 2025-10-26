//! # Reinhardt Parsers
//!
//! Request body parsers for the Reinhardt framework, inspired by Django REST Framework.
//!
//! ## Parsers
//!
//! - **JSONParser**: Parse JSON request bodies
//! - **XMLParser**: Parse XML request bodies (application/xml, text/xml)
//! - **FormParser**: Parse HTML form data (application/x-www-form-urlencoded)
//! - **MultiPartParser**: Handle file uploads (multipart/form-data)
//! - **FileUploadParser**: Raw file upload handling
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_parsers::{JSONParser, Parser};
//!
//! let parser = JSONParser::new();
//! let data = parser.parse(&request).await?;
//! ```
//!
//! ## Planned Features
//! TODO: YAML Parser - For `application/x-yaml`
//! TODO: MessagePack Parser - For binary message format
//! TODO: Protobuf Parser - For Protocol Buffers
//! TODO: Streaming parsing - For large file uploads without loading entire body into memory
//! TODO: Content negotiation - Automatic parser selection based on Accept header
//! TODO: Custom validators - Per-parser validation hooks
//! TODO: Schema validation - JSON Schema, XML Schema support
//! TODO: Compression support - Gzip, Brotli, Deflate decompression
//! TODO: Zero-copy parsing - Where possible with current parser implementations
//! TODO: Parallel multipart processing - Parse multiple files concurrently
//! TODO: Memory pooling - Reuse buffers for repeated parsing operations

pub mod file;
pub mod form;
pub mod json;
pub mod multipart;
pub mod parser;
pub mod xml;

pub use file::FileUploadParser;
pub use form::FormParser;
pub use json::JSONParser;
pub use multipart::MultiPartParser;
pub use parser::{MediaType, ParseError, ParseResult, Parser};
pub use xml::{XMLParser, XmlParserConfig, XmlParserConfigBuilder};
