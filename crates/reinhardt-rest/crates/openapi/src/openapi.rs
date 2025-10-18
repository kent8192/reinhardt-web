//! OpenAPI 3.0 types
//!
//! Re-exports utoipa's OpenAPI types for use in Reinhardt.

// Re-export core utoipa types as Reinhardt's OpenAPI types
pub use utoipa::openapi::{
    Components, Contact, Header, Info, License, OpenApi as OpenApiSchema, PathItem, Paths, RefOr,
    Schema, Server, Tag,
};

// Re-export request/response types
pub use utoipa::openapi::request_body::RequestBody;
pub use utoipa::openapi::response::{Response, Responses};

// Re-export path operation types
pub use utoipa::openapi::path::{Operation, Parameter, ParameterIn};

// Re-export content-related types (MediaType)
pub use utoipa::openapi::Content as MediaType;

// Re-export path-related types
pub use utoipa::openapi::path::ParameterIn as ParameterLocation;

// Re-export security-related types
pub use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};

// Provide convenient type alias for API key location
pub type ApiKeyLocation = utoipa::openapi::security::ApiKeyValue;

// Re-export HttpScheme for convenience
pub type HttpScheme = HttpAuthScheme;
