//! Utility functions for building HTTP responses, handling requests, and testing.
//!
//! This module provides convenient helper functions for:
//! - Building HTTP responses with appropriate status codes and JSON bodies
//! - Extracting and validating common request data patterns in microservices
//! - Testing HTTP handlers and endpoints

pub mod request_helpers;
pub mod response_builders;
pub mod testing;

pub use request_helpers::*;
pub use response_builders::*;
pub use testing::*;
