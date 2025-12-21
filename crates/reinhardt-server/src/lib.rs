//! HTTP server implementation for Reinhardt framework
//!
//! This crate provides HTTP server capabilities with support for:
//! - Hyper-based HTTP/1.1 and HTTP/2
//! - GraphQL (optional)
//! - WebSocket (optional)

// Re-export server implementation
#[cfg(feature = "server")]
pub use reinhardt_server_core::*;
