//! REST API integration tests module
//!
//! Cross-crate integration tests for REST API components.

mod rest {
	mod openapi_schema_generation_integration;
	mod serializers_filters_integration;
	mod versioning_routers_integration;
}
