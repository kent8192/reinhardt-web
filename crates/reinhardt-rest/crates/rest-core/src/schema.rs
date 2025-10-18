//! OpenAPI/Swagger schema generation
//!
//! Re-exports schema types from reinhardt-openapi.
//!
//! TODO: This module is temporarily disabled due to ongoing refactoring
//! of reinhardt-openapi's utoipa integration. The auto-schema generation
//! system requires significant implementation work (see reinhardt-openapi/src/auto_schema.rs).
//!
//! Re-enable this module once reinhardt-openapi provides stable exports.

/*
// Re-export all schema types from reinhardt-openapi
pub use reinhardt_openapi::{
    auto_schema::{SchemaObject, ToSchema},
    generator::SchemaGenerator,
    inspector::ViewSetInspector,
    openapi::{
        Components, Contact, Info, License, MediaType, OpenApiSchema, Operation, Parameter,
        ParameterLocation, PathItem, RequestBody, Response, Schema, SecurityRequirement,
        SecurityScheme, Server, ServerVariable, Tag,
    },
    swagger::SwaggerUI,
    SchemaError, SchemaResult,
};

/// OpenAPI version constant
pub const OPENAPI_VERSION: &str = "3.0.3";
*/
