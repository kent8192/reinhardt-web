// Auto-generated module file for server integration tests
// Each test file in server/ subdirectory is explicitly included with #[path] attribute

#[path = "server/e2e_graphql_integration.rs"]
mod e2e_graphql_integration;

#[path = "server/graceful_shutdown_integration.rs"]
mod graceful_shutdown_integration;

#[path = "server/http2_integration.rs"]
mod http2_integration;

#[path = "server/server_advanced_integration.rs"]
mod server_advanced_integration;

#[path = "server/server_middleware_integration_tests.rs"]
mod server_middleware_integration_tests;

#[path = "server/server_test_helpers.rs"]
mod server_test_helpers;

#[path = "server/server_tests.rs"]
mod server_tests;
