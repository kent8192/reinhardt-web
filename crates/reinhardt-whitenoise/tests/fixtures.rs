//! Specialized test fixtures for reinhardt-whitenoise
//!
//! This module provides fixtures that wrap generic reinhardt-test fixtures
//! with whitenoise-specific test data.

mod test_fixtures;

// Re-export public fixture functions
pub use test_fixtures::{large_file, manifest_dir, mixed_extensions_dir, nested_dir, static_dir};
