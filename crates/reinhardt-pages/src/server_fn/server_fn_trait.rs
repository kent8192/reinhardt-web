//! Server Function Trait and Error Types
//!
//! This module defines the core trait and error types for server functions.

use serde::{Deserialize, Serialize};

/// Common trait for all server functions
///
/// This trait is implemented automatically by the `#[server_fn]` macro.
/// Users typically don't need to implement this manually.
pub trait ServerFn {
	/// The input type (function arguments)
	type Input: Serialize + for<'de> Deserialize<'de>;

	/// The output type (function return value)
	type Output: Serialize + for<'de> Deserialize<'de>;

	/// The endpoint path for this server function
	fn endpoint() -> &'static str;

	/// The codec name ("json", "url", "msgpack")
	fn codec() -> &'static str {
		"json"
	}
}

/// Unified error type for server functions
///
/// This error type covers all possible error conditions when calling
/// a server function from the client side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerFnError {
	/// Network error (connection failed, timeout, etc.)
	Network(String),

	/// Serialization error (failed to serialize arguments)
	Serialization(String),

	/// Deserialization error (failed to deserialize response)
	Deserialization(String),

	/// Server-side error (HTTP 4xx, 5xx)
	Server {
		/// HTTP status code
		status: u16,
		/// Error message
		message: String,
	},

	/// Application error (custom error from server function)
	Application(String),
}

impl ServerFnError {
	/// Create a network error
	pub fn network(msg: impl Into<String>) -> Self {
		Self::Network(msg.into())
	}

	/// Create a serialization error
	pub fn serialization(msg: impl Into<String>) -> Self {
		Self::Serialization(msg.into())
	}

	/// Create a deserialization error
	pub fn deserialization(msg: impl Into<String>) -> Self {
		Self::Deserialization(msg.into())
	}

	/// Create a server error
	pub fn server(status: u16, message: impl Into<String>) -> Self {
		Self::Server {
			status,
			message: message.into(),
		}
	}

	/// Create an application error
	pub fn application(msg: impl Into<String>) -> Self {
		Self::Application(msg.into())
	}
}

impl std::fmt::Display for ServerFnError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Network(msg) => write!(f, "Network error: {}", msg),
			Self::Serialization(msg) => write!(f, "Serialization error: {}", msg),
			Self::Deserialization(msg) => write!(f, "Deserialization error: {}", msg),
			Self::Server { status, message } => {
				write!(f, "Server error ({}): {}", status, message)
			}
			Self::Application(msg) => write!(f, "Application error: {}", msg),
		}
	}
}

impl std::error::Error for ServerFnError {}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_server_fn_error_creation() {
		let err = ServerFnError::network("Connection timeout");
		assert!(matches!(err, ServerFnError::Network(_)));

		let err = ServerFnError::server(404, "Not found");
		assert!(matches!(err, ServerFnError::Server { status: 404, .. }));
	}

	#[rstest]
	fn test_server_fn_error_display() {
		let err = ServerFnError::network("Connection timeout");
		assert_eq!(err.to_string(), "Network error: Connection timeout");

		let err = ServerFnError::server(500, "Internal error");
		assert_eq!(err.to_string(), "Server error (500): Internal error");
	}
}
