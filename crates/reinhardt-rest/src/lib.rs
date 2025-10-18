//! # Reinhardt REST
//!
//! **Unified REST API framework** for Reinhardt, formerly known as reinhardt-drf (Django REST Framework).
//!
//! This crate provides a complete REST API framework by integrating:
//! - **Serializers**: Data serialization and validation (from reinhardt-serializers)
//! - **Parsers**: Request body parsing (from reinhardt-parsers)
//! - **Renderers**: Response rendering (from reinhardt-renderers)
//! - **Authentication**: JWT, Token, Session, Basic auth (from reinhardt-auth)
//! - **Routers**: Automatic URL routing for ViewSets (from reinhardt-routers)
//! - **Browsable API**: HTML interface for API exploration (from reinhardt-browsable-api)
//!
//! ## Features
//!
//! - **default**: Enables serializers, parsers, and renderers
//! - **serializers**: Data serialization and validation components
//! - **parsers**: Request body parsing (JSON, Form, Multipart)
//! - **renderers**: Response rendering (JSON, XML, YAML, CSV, etc.)
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_rest::{
//!     JSONParser, JSONRenderer, Parser, Renderer,
//!     BrowsableApiRenderer, DefaultRouter
//! };
//!
//! // Parser
//! let parser = JSONParser::new();
//! let data = parser.parse(&request).await?;
//!
//! // Renderer
//! let renderer = JSONRenderer::new();
//! let response = renderer.render(&data, None).await?;
//!
//! // Router
//! let mut router = DefaultRouter::new();
//! router.register_viewset("users", Arc::new(UserViewSet));
//! ```
//!
//! ## Testing
//!
//! This crate contains unit tests for the integrated modules.
//! Integration tests are located in `tests/integration/`.

// Note: authentication, response, routers modules have been moved to internal crates
// pub mod schema;  // Temporarily disabled - utoipa API compatibility issues

// Re-export internal crates (2024 edition module system)
// These modules represent the internal crates that are now part of reinhardt-rest

// Serializers module - from crates/serializers
#[cfg(feature = "serializers")]
pub use reinhardt_serializers as serializers_internal;

#[cfg(feature = "serializers")]
pub mod serializers {
    //! Data serialization and validation components
    //!
    //! Provides Django REST Framework-style serializers for data validation,
    //! transformation, and model serialization.

    // Re-export everything from the internal crate
    pub use reinhardt_serializers::*;
}

// Parsers module - from crates/parsers
#[cfg(feature = "parsers")]
pub use reinhardt_parsers as parsers_internal;

#[cfg(feature = "parsers")]
pub mod parsers {
    //! Request body parsers for the Reinhardt framework
    //!
    //! Provides parsers for different content types:
    //! - **JSONParser**: Parse JSON request bodies
    //! - **FormParser**: Parse HTML form data (application/x-www-form-urlencoded)
    //! - **MultiPartParser**: Handle file uploads (multipart/form-data)
    //! - **FileUploadParser**: Raw file upload handling

    // Re-export everything from the internal crate
    pub use reinhardt_parsers::*;
}

// Renderers module
#[cfg(feature = "renderers")]
pub mod renderers {
    //! Response renderers for the Reinhardt framework
    //!
    //! Provides renderers for different output formats:
    //! - **JSONRenderer**: Render responses as JSON
    //! - **XMLRenderer**: Render responses as XML
    //! - **YAMLRenderer**: Render responses as YAML
    //! - **CSVRenderer**: Render responses as CSV
    //! - **BrowsableAPIRenderer**: HTML self-documenting API interface
    //! - **OpenAPIRenderer**: Generate OpenAPI 3.0 specifications
    //! - **AdminRenderer**: Django-like admin interface renderer
    //! - **StaticHTMLRenderer**: Static HTML content renderer
    //! - **DocumentationRenderer**: Render API documentation from OpenAPI schemas
    //! - **SchemaJSRenderer**: Render OpenAPI schemas as JavaScript

    // Re-export from specialized crates
    pub use reinhardt_browsable_api::BrowsableApiRenderer as BrowsableAPIRenderer;
}

// Re-export other internal crates
pub use reinhardt_filters as filters;
pub use reinhardt_metadata as metadata;
pub use reinhardt_negotiation as negotiation;
pub use reinhardt_pagination as pagination;
pub use reinhardt_throttling as throttling;
pub use reinhardt_versioning as versioning;

// Re-export from rest-core
pub use rest_core::authentication;
pub use rest_core::response;
pub use rest_core::routers;

// Re-export authentication types
pub use rest_core::authentication::{
    AllowAny, AnonymousUser, AuthBackend, AuthResult, Claims, IsAdminUser, IsAuthenticated,
    IsAuthenticatedOrReadOnly, JwtAuth, Permission, SimpleUser, User,
};

// Re-export response types
pub use rest_core::response::{ApiResponse, IntoApiResponse, PaginatedResponse, ResponseBuilder};

// Re-export router types
pub use rest_core::routers::{DefaultRouter, Route, Router, UrlPattern};

// Re-export from specialized crates
pub use reinhardt_browsable_api::*;

// Re-export integrated modules at top level for convenience
#[cfg(feature = "serializers")]
pub use serializers::{
    ContentNegotiator, Deserializer, JsonSerializer, ModelSerializer, Serializer, SerializerError,
    UniqueTogetherValidator, UniqueValidator,
};

#[cfg(feature = "parsers")]
pub use parsers::{
    FileUploadParser, FormParser, JSONParser, MediaType, MultiPartParser, ParseError, ParseResult,
    Parser,
};

#[cfg(feature = "renderers")]
pub use renderers::BrowsableAPIRenderer;

// Temporarily disabled - utoipa API compatibility issues
/*
pub use schema::{
    Components, Contact, Info, License, MediaType, OpenApiSchema, Operation, Parameter,
    ParameterLocation, PathItem, RequestBody, Response, Schema, SecurityRequirement,
    SecurityScheme, Server, ServerVariable, Tag, OPENAPI_VERSION,
};
*/

#[cfg(test)]
mod tests {
    #[test]
    fn test_serializers_module_available() {
        #[cfg(feature = "serializers")]
        {
            use crate::serializers::JsonSerializer;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
            struct TestData {
                name: String,
            }

            let _serializer = JsonSerializer::<TestData>::new();
        }
    }

    #[test]
    fn test_parsers_module_available() {
        #[cfg(feature = "parsers")]
        {
            use crate::parsers::JSONParser;
            let _parser = JSONParser::new();
        }
    }

    #[test]
    fn test_renderers_module_available() {
        #[cfg(feature = "renderers")]
        {
            use crate::serializers::content_negotiation::JSONRenderer;
            let _renderer = JSONRenderer::new();
        }
    }
}
