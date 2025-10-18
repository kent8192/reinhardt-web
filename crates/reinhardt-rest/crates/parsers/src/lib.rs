//! # Reinhardt Parsers
//!
//! Request body parsers for the Reinhardt framework, inspired by Django REST Framework.
//!
//! ## Parsers
//!
//! - **JSONParser**: Parse JSON request bodies
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

pub mod file;
pub mod form;
pub mod json;
pub mod multipart;
pub mod parser;

pub use file::FileUploadParser;
pub use form::FormParser;
pub use json::JSONParser;
pub use multipart::MultiPartParser;
pub use parser::{MediaType, ParseError, ParseResult, Parser};
