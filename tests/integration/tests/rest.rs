//! REST API integration tests module
//!
//! Cross-crate integration tests for REST API components.

mod rest {
	mod openapi_derive_schema_tests;
	mod openapi_macro_integration_tests;
	mod openapi_schema_generation_integration;
	mod schema_integration_tests;
	mod serializers_filters_integration;
	mod versioning_routers_integration;
}
