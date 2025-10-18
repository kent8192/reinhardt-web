//! Integration tests for reinhardt-proxy
//!
//! These tests require integration with other reinhardt crates and are
//! organized by dependency requirements:
//!
//! - `orm_integration`: Tests requiring reinhardt-orm (25 tests)
//! - `advanced_features`: Tests requiring Phases 4-8 features (15 tests)
//!
//! All tests are marked with `#[ignore]` and will be enabled as dependencies
//! become available.

pub mod advanced_features;
pub mod orm_integration;
