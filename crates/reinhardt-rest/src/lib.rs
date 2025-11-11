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

// Re-export internal crates (2024 edition module system)
// These modules represent the internal crates that are now part of reinhardt-rest

// Serializers module - from crates/serializers
#[cfg(feature = "serializers")]
pub use reinhardt_serializers as serializers;

// Parsers module - now part of reinhardt-rest
#[cfg(feature = "parsers")]
pub use reinhardt_core::parsers;

// Renderers module
#[cfg(feature = "renderers")]
pub use reinhardt_renderers as renderers;

// Re-export other internal crates
pub use reinhardt_core::negotiation;
pub use reinhardt_core::pagination;
pub use reinhardt_filters as filters;
pub use reinhardt_metadata as metadata;
pub use reinhardt_throttling as throttling;
pub use reinhardt_versioning as versioning;

// Re-export from rest-core
pub use rest_core::authentication;
pub use rest_core::response;
pub use rest_core::routers;

// Re-export authentication types
pub use rest_core::authentication::{
	AllowAny, AnonymousUser, AuthBackend, AuthResult, IsAdminUser, IsAuthenticated,
	IsAuthenticatedOrReadOnly, Permission, SimpleUser, User,
};

// Re-export JWT types conditionally
#[cfg(feature = "jwt")]
pub use rest_core::authentication::{Claims, JwtAuth};

// Re-export response types
pub use rest_core::response::{ApiResponse, IntoApiResponse, PaginatedResponse, ResponseBuilder};

// Re-export router types
pub use rest_core::routers::{DefaultRouter, Route, Router, UrlPattern};

// Re-export from specialized crates
pub use reinhardt_browsable_api::*;

// Re-export integrated modules at top level for convenience
#[cfg(feature = "serializers")]
pub use reinhardt_serializers::{
	ContentNegotiator, Deserializer, JsonSerializer, ModelSerializer, Serializer, SerializerError,
	UniqueTogetherValidator, UniqueValidator,
};

#[cfg(feature = "parsers")]
pub use reinhardt_core::parsers::{
	FileUploadParser, FormParser, JSONParser, MediaType, MultiPartParser, ParseError, ParseResult,
	Parser,
};

#[cfg(feature = "renderers")]
pub use reinhardt_browsable_api::BrowsableApiRenderer as BrowsableAPIRenderer;

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
			use crate::JsonSerializer;
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
			use crate::JSONParser;
			let _parser = JSONParser::new();
		}
	}

	#[test]
	fn test_renderers_module_available() {
		#[cfg(feature = "renderers")]
		{
			use crate::renderers::JSONRenderer;
			let _renderer = JSONRenderer::new();
		}
	}
}
