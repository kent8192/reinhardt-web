// Auto-generated module file for auth integration tests
// Each test file in auth/ subdirectory is explicitly included with #[path] attribute

#[path = "auth/auth_integration.rs"]
mod auth_integration;

#[path = "auth/auth_security_integration.rs"]
mod auth_security_integration;

#[path = "auth/csrf_protection_integration.rs"]
mod csrf_protection_integration;

#[path = "auth/multi_auth_backend_integration.rs"]
mod multi_auth_backend_integration;

#[path = "auth/password_hasher_integration.rs"]
mod password_hasher_integration;

#[path = "auth/session_authentication_integration.rs"]
mod session_authentication_integration;

#[path = "auth/token_storage_integration.rs"]
mod token_storage_integration;

#[path = "auth/mfa_integration.rs"]
mod mfa_integration;

#[path = "auth/oauth2_flows_integration.rs"]
mod oauth2_flows_integration;

#[path = "auth/rate_limit_redis_integration.rs"]
mod rate_limit_redis_integration;

#[path = "auth/permission_composition_integration.rs"]
mod permission_composition_integration;

#[path = "auth/concurrent_auth_integration.rs"]
mod concurrent_auth_integration;
