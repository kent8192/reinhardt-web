//! DCL Integration Test Module
//!
//! Integration tests for DCL statements using testcontainers-rs.
//!
//! ## Test Organization
//!
//! - **PostgreSQL Tests**: Full DCL support testing
//! - **MySQL Tests**: MySQL-specific feature testing
//! - **CockroachDB Tests**: PostgreSQL compatibility testing

#[path = "dcl/postgres_tests.rs"]
mod postgres_tests;

#[path = "dcl/mysql_tests.rs"]
mod mysql_tests;

#[path = "dcl/cockroachdb_tests.rs"]
mod cockroachdb_tests;

// Re-exports for convenience
pub use cockroachdb_tests::*;
pub use mysql_tests::*;
pub use postgres_tests::*;
