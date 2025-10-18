//! # Reinhardt Schema Generation
//!
//! OpenAPI 3.0 schema generation for Reinhardt REST APIs.
//!
//! ## Features
//!
//! - **OpenAPI 3.0**: Full OpenAPI 3.0 specification support
//! - **Auto-generation**: Automatic schema generation from ViewSets
//! - **Customization**: Override and extend generated schemas
//! - **Swagger UI**: Built-in Swagger UI integration
//! - **YAML/JSON**: Export schemas in both formats
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_schema::{SchemaGenerator, OpenApiSchema};
//!
//! // Generate schema from ViewSets
//! let generator = SchemaGenerator::new()
//!     .title("My API")
//!     .version("1.0.0")
//!     .description("API documentation");
//!
//! let schema = generator.generate()?;
//! let json = schema.to_json()?;
//! ```

pub mod auto_schema;
pub mod generator;
pub mod inspector;
pub mod openapi;
pub mod swagger;
pub mod viewset_inspector;
// pub mod utoipa_compat;  // Temporarily disabled - requires extensive utoipa API updates

use thiserror::Error;

pub use auto_schema::{SchemaObject, ToSchema};
pub use generator::SchemaGenerator;
pub use inspector::ViewSetInspector as LegacyViewSetInspector;
pub use openapi::{
    Header, Info, MediaType, OpenApiSchema, Operation, Parameter, ParameterLocation, PathItem,
    RequestBody, Response, Schema, Server,
};
pub use swagger::SwaggerUI;
pub use viewset_inspector::{InspectorConfig, ViewSetInspector};

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Inspector error: {0}")]
    InspectorError(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type SchemaResult<T> = std::result::Result<T, SchemaError>;
