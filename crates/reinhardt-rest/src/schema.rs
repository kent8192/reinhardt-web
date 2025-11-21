//! OpenAPI/Swagger schema generation
//!
//! Re-exports schema types from openapi crate.

// Re-export all types from openapi crate
#[cfg(feature = "openapi")]
pub use reinhardt_openapi::*;

/// OpenAPI version constant
pub const OPENAPI_VERSION: &str = "3.0.3";
