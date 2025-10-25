//! # Reinhardt Metadata
//!
//! Metadata API for handling OPTIONS requests in Reinhardt framework.
//!
//! ## Features
//!
//! - **BaseMetadata**: Base trait for metadata providers
//! - **SimpleMetadata**: Default metadata implementation that returns view and field information
//! - Automatic field type detection
//! - Action-based metadata (POST, PUT, etc.)
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_metadata::{BaseMetadata, SimpleMetadata, MetadataOptions};
//!
//! let metadata = SimpleMetadata::new();
//! let options = MetadataOptions {
//!     name: "User List".to_string(),
//!     description: "List all users".to_string(),
//!     allowed_methods: vec!["GET".to_string(), "POST".to_string()],
//!     renders: vec!["application/json".to_string()],
//!     parses: vec!["application/json".to_string()],
//! };
//! ```
//!
//! ## Planned Features
//! TODO: OpenAPI 3.0 schema generation from field metadata
//! TODO: Automatic schema inference from Rust types
//! TODO: Schema validation and documentation
//! TODO: Serializer-aware metadata generation
//! TODO: Model-based metadata introspection
//! TODO: Custom metadata class support
//! TODO: Regular expression validation patterns
//! TODO: Field dependencies and conditional requirements

mod base;
mod fields;
mod options;
mod response;
mod types;
mod validators;

// Re-export all public items
pub use base::{BaseMetadata, SimpleMetadata};
pub use fields::{FieldInfo, FieldInfoBuilder};
pub use options::MetadataOptions;
pub use response::{ActionMetadata, MetadataResponse};
pub use types::{ChoiceInfo, FieldType, MetadataError};
pub use validators::FieldValidator;
