//! Utility functions for building HTTP responses and handling requests.
//!
//! This module provides convenient helper functions for:
//! - Building HTTP responses with appropriate status codes and JSON bodies
//! - Extracting and validating common request data patterns in microservices

pub mod request_helpers;
pub mod response_builders;

pub use request_helpers::*;
pub use response_builders::*;
