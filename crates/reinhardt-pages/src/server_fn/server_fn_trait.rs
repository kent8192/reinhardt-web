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

	/// Returns the human-readable message without the variant prefix.
	///
	/// Use this when surfacing the error text directly to end users;
	/// use `to_string()` (`Display`) for the developer-facing form
	/// that includes the variant tag.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_pages::ServerFnError;
	///
	/// let err = ServerFnError::application("Invalid choice_id");
	/// assert_eq!(err.message(), "Invalid choice_id");
	/// assert_eq!(err.to_string(), "Application error: Invalid choice_id");
	/// ```
	pub fn message(&self) -> &str {
		match self {
			Self::Network(msg)
			| Self::Serialization(msg)
			| Self::Deserialization(msg)
			| Self::Application(msg) => msg,
			Self::Server { message, .. } => message,
		}
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

/// Extract the human-readable message from a `ServerFnError` string,
/// regardless of format.
///
/// Accepts three representations:
///
/// 1. **JSON wire format** — serde's externally-tagged envelope
///    (e.g., `{"Application":"Invalid choice_id"}`).
/// 2. **`Display` format** — the variant-prefixed string produced by
///    `ServerFnError::to_string()` (e.g., `"Application error: msg"`).
/// 3. **Plain text** — returned unchanged as a fallback.
///
/// # Examples
///
/// ```
/// use reinhardt_pages::parse_server_error_message;
///
/// // JSON wire format
/// let msg = parse_server_error_message(r#"{"Application":"Invalid choice_id"}"#);
/// assert_eq!(msg, "Invalid choice_id");
///
/// // Display format (from .to_string())
/// let msg = parse_server_error_message("Application error: Invalid choice_id");
/// assert_eq!(msg, "Invalid choice_id");
///
/// // Plain text fallback
/// let msg = parse_server_error_message("plain error text");
/// assert_eq!(msg, "plain error text");
/// ```
pub fn parse_server_error_message(raw: &str) -> String {
	// 1. Try JSON deserialization (wire format)
	if let Ok(e) = serde_json::from_str::<ServerFnError>(raw) {
		return unwrap_nested_or_raw(e.message());
	}
	// 2. Try stripping known Display prefixes
	for prefix in [
		"Network error: ",
		"Serialization error: ",
		"Deserialization error: ",
		"Application error: ",
	] {
		if let Some(msg) = raw.strip_prefix(prefix) {
			return unwrap_nested_or_raw(msg);
		}
	}
	// 2b. Handle "Server error (NNN): " format
	if let Some(rest) = raw.strip_prefix("Server error (")
		&& let Some(idx) = rest.find("): ")
	{
		return unwrap_nested_or_raw(&rest[idx + 3..]);
	}
	// 3. Fallback: return unchanged
	raw.to_string()
}

/// If `msg` is itself a JSON-serialized `ServerFnError` (nested envelope),
/// unwrap it; otherwise return the string as-is.
fn unwrap_nested_or_raw(msg: &str) -> String {
	serde_json::from_str::<ServerFnError>(msg)
		.map(|e| e.message().to_string())
		.unwrap_or_else(|_| msg.to_string())
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	fn test_server_fn_error_creation() {
		// Arrange & Act
		let network_err = ServerFnError::network("Connection timeout");
		let server_err = ServerFnError::server(404, "Not found");

		// Assert
		assert!(matches!(network_err, ServerFnError::Network(_)));
		assert!(matches!(
			server_err,
			ServerFnError::Server { status: 404, .. }
		));
	}

	#[rstest]
	fn test_server_fn_error_display() {
		// Arrange
		let network_err = ServerFnError::network("Connection timeout");
		let server_err = ServerFnError::server(500, "Internal error");

		// Act & Assert
		assert_eq!(network_err.to_string(), "Network error: Connection timeout");
		assert_eq!(server_err.to_string(), "Server error (500): Internal error");
	}

	#[rstest]
	#[case::network(ServerFnError::network("timeout"), "timeout")]
	#[case::serialization(ServerFnError::serialization("bad input"), "bad input")]
	#[case::deserialization(ServerFnError::deserialization("bad json"), "bad json")]
	#[case::server(ServerFnError::server(403, "Forbidden"), "Forbidden")]
	#[case::application(ServerFnError::application("Invalid choice_id"), "Invalid choice_id")]
	fn test_message_returns_inner_text(#[case] err: ServerFnError, #[case] expected: &str) {
		// Act
		let msg = err.message();

		// Assert
		assert_eq!(msg, expected);
	}

	#[rstest]
	fn test_message_returns_empty_string_when_inner_is_empty() {
		// Arrange
		let err = ServerFnError::application("");

		// Act & Assert
		assert_eq!(err.message(), "");
	}

	#[rstest]
	fn test_message_differs_from_display() {
		// Arrange
		let err = ServerFnError::application("Invalid choice_id");

		// Act
		let message = err.message();
		let display = err.to_string();

		// Assert
		assert_ne!(message, display);
		assert_eq!(message, "Invalid choice_id");
		assert_eq!(display, "Application error: Invalid choice_id");
	}

	#[rstest]
	#[case::application(r#"{"Application":"Invalid choice_id"}"#, "Invalid choice_id")]
	#[case::server(r#"{"Server":{"status":403,"message":"Forbidden"}}"#, "Forbidden")]
	#[case::network(r#"{"Network":"Connection timeout"}"#, "Connection timeout")]
	fn test_parse_server_error_message_from_json(#[case] json: &str, #[case] expected: &str) {
		// Act
		let msg = parse_server_error_message(json);

		// Assert
		assert_eq!(msg, expected);
	}

	#[rstest]
	#[case::application("Application error: Invalid choice_id", "Invalid choice_id")]
	#[case::network("Network error: Connection timeout", "Connection timeout")]
	#[case::serialization("Serialization error: bad input", "bad input")]
	#[case::deserialization("Deserialization error: bad json", "bad json")]
	#[case::server("Server error (403): Forbidden", "Forbidden")]
	#[case::server_500("Server error (500): Internal error", "Internal error")]
	fn test_parse_server_error_message_from_display(#[case] display: &str, #[case] expected: &str) {
		// Act
		let msg = parse_server_error_message(display);

		// Assert
		assert_eq!(msg, expected);
	}

	#[rstest]
	#[case::server_wrapping_application(
		r#"Server error (500): {"Application":"Invalid choice_id"}"#,
		"Invalid choice_id"
	)]
	#[case::server_wrapping_network(
		r#"Server error (500): {"Network":"Connection lost"}"#,
		"Connection lost"
	)]
	#[case::json_server_wrapping_application(
		r#"{"Server":{"status":500,"message":"{\"Application\":\"Invalid choice_id\"}"}}"#,
		"Invalid choice_id"
	)]
	fn test_parse_server_error_message_unwraps_nested_json(
		#[case] input: &str,
		#[case] expected: &str,
	) {
		// Act
		let msg = parse_server_error_message(input);

		// Assert
		assert_eq!(msg, expected);
	}

	#[rstest]
	fn test_parse_server_error_message_falls_back_for_invalid_json() {
		// Arrange
		let raw = "plain error text";

		// Act
		let msg = parse_server_error_message(raw);

		// Assert
		assert_eq!(msg, "plain error text");
	}

	#[rstest]
	fn test_parse_server_error_message_falls_back_for_empty_string() {
		// Act
		let msg = parse_server_error_message("");

		// Assert
		assert_eq!(msg, "");
	}

	#[rstest]
	fn test_parse_server_error_message_falls_back_for_non_server_fn_error_json() {
		// Arrange
		let raw = r#"{"foo":"bar"}"#;

		// Act
		let msg = parse_server_error_message(raw);

		// Assert
		assert_eq!(msg, raw);
	}
}
