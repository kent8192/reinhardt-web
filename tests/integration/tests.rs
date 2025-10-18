//! Integration Tests for Reinhardt Framework
//!
//! This module contains integration tests that verify various reinhardt crates
//! work correctly with web frameworks and middleware:
//!
//! - **flatpages_http.rs**: Basic HTTP request/response tests (8 tests)
//!   - GET/404 handling
//!   - View vs fallback routing
//!   - Special characters in URLs
//!   - Nested paths
//!
//! - **csrf_tests.rs**: CSRF protection integration (5 tests, 4 pending)
//!   - GET requests with CSRF middleware
//!   - POST without CSRF token (pending middleware)
//!   - POST with valid token (pending middleware)
//!
//! - **auth_tests.rs**: Authentication integration (7 tests, 6 pending)
//!   - Public page access
//!   - Registration-required pages (pending middleware)
//!   - Authenticated user access (pending middleware)
//!
//! - **graphql_integration.rs**: GraphQL integration tests
//!   - Full GraphQL workflow (queries, mutations, subscriptions)
//!   - Concurrent operations
//!   - Schema introspection
//!   - Error handling
//!   - Complex queries and batched operations
//!
//! - **metadata_tests.rs**: Metadata API integration tests (6 tests)
//!   - Metadata with None class
//!   - Global permissions with custom permission checks
//!   - Object permissions with custom permission checks
//!   - Request cloning with versioning (bug 2455)
//!   - Versioning scheme access (bug 2477)
//!   - Read-only PrimaryKeyRelatedField metadata
//!
//! ## Test Organization
//!
//! These tests complement the unit tests in individual crates.
//!
//! ### Unit Tests (in crates/*/tests/ or src/)
//! - Database model operations
//! - Middleware logic (without HTTP)
//! - View logic (without HTTP)
//! - Input validation
//!
//! ### Integration Tests (here in tests/integration/)
//! - HTTP-level functionality
//! - CSRF middleware integration
//! - Authentication middleware integration
//! - GraphQL integration
//!
//! ## Running Tests
//!
//! See [README.md](../README.md) for detailed instructions.
//!
//! ```bash
//! # Run all integration tests
//! cargo test --package reinhardt-integration-tests
//!
//! # Run specific test file
//! cargo test --test graphql_integration
//!
//! # Include ignored tests (pending middleware)
//! cargo test -- --ignored
//! ```
