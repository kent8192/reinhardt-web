//! Server Function Trait and Error Types
//!
//! This module defines the core trait and error types for server functions.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFnError {
	payload: ServerFnErrorPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServerFnErrorPayload {
	kind: ServerFnErrorKind,
	status: Option<u16>,
	message: String,
	field_errors: Vec<ServerFnFieldError>,
}

/// A validation error associated with one input field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFnFieldError {
	field: String,
	message: String,
}

impl ServerFnFieldError {
	/// Returns the input field name.
	pub fn field(&self) -> &str {
		&self.field
	}

	/// Returns the safe message for the field.
	pub fn message(&self) -> &str {
		&self.message
	}
}

/// The category represented by a [`ServerFnError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerFnErrorKind {
	/// Input validation failed.
	Validation,
	/// Authentication or authorization failed.
	Auth,
	/// An application-level error was returned.
	Application,
	/// A server-side HTTP error was returned.
	Server,
	/// A transport-level error occurred.
	Transport,
	/// Deserialization of a response failed.
	Deserialization,
}

impl ServerFnError {
	fn new(kind: ServerFnErrorKind, status: Option<u16>, message: impl Into<String>) -> Self {
		Self {
			payload: ServerFnErrorPayload {
				kind,
				status,
				message: message.into(),
				field_errors: Vec::new(),
			},
		}
	}

	/// Create a validation error with the default message.
	pub fn validation() -> Self {
		Self::validation_with_message("Validation failed", std::iter::empty::<(&str, &str)>())
	}

	/// Create a validation error with field-specific messages.
	pub fn validation_with_message<I, F, M>(message: impl Into<String>, field_errors: I) -> Self
	where
		I: IntoIterator<Item = (F, M)>,
		F: Into<String>,
		M: Into<String>,
	{
		let mut error = Self::new(ServerFnErrorKind::Validation, Some(422), message);
		error.payload.field_errors = field_errors
			.into_iter()
			.map(|(field, message)| ServerFnFieldError {
				field: field.into(),
				message: message.into(),
			})
			.collect();
		error
	}

	/// Create an authentication error.
	pub fn auth(status: u16, message: impl Into<String>) -> Self {
		Self::new(ServerFnErrorKind::Auth, Some(status), message)
	}

	/// Create an application error.
	pub fn application(message: impl Into<String>) -> Self {
		Self::new(ServerFnErrorKind::Application, None, message)
	}

	/// Create a server-side error.
	pub fn server(status: u16, message: impl Into<String>) -> Self {
		Self::new(ServerFnErrorKind::Server, Some(status), message)
	}

	/// Create a transport error.
	pub fn transport(message: impl Into<String>) -> Self {
		Self::new(ServerFnErrorKind::Transport, None, message)
	}

	/// Create a network error.
	pub fn network(msg: impl Into<String>) -> Self {
		Self::transport(msg)
	}

	/// Create a serialization transport error.
	pub fn serialization(msg: impl Into<String>) -> Self {
		Self::transport(msg)
	}

	/// Create a deserialization error.
	pub fn deserialization(msg: impl Into<String>) -> Self {
		Self::new(ServerFnErrorKind::Deserialization, None, msg)
	}

	/// Returns the error category.
	pub fn kind(&self) -> ServerFnErrorKind {
		self.payload.kind
	}

	/// Returns the optional HTTP status code.
	pub fn status(&self) -> Option<u16> {
		self.payload.status
	}

	/// Returns the safe message intended for users.
	pub fn user_message(&self) -> &str {
		&self.payload.message
	}

	/// Returns the safe message intended for users.
	pub fn message(&self) -> &str {
		self.user_message()
	}

	/// Returns field-specific validation errors.
	pub fn field_errors(&self) -> &[ServerFnFieldError] {
		&self.payload.field_errors
	}
}

impl std::fmt::Display for ServerFnError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.user_message())
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct WireServerFnError {
	version: u8,
	kind: WireServerFnErrorKind,
	status: Option<u16>,
	message: String,
	field_errors: Vec<WireServerFnFieldError>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireServerFnErrorKind {
	Validation,
	Auth,
	Application,
	Server,
	Transport,
	Deserialization,
}

#[derive(Debug, Serialize, Deserialize)]
struct WireServerFnFieldError {
	field: String,
	message: String,
}

impl Serialize for ServerFnError {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		WireServerFnError {
			version: 1,
			kind: self.payload.kind.into(),
			status: self.payload.status,
			message: self.payload.message.clone(),
			field_errors: self
				.payload
				.field_errors
				.iter()
				.map(|error| WireServerFnFieldError {
					field: error.field.clone(),
					message: error.message.clone(),
				})
				.collect(),
		}
		.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for ServerFnError {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let wire = WireServerFnError::deserialize(deserializer)?;
		if wire.version != 1 {
			return Err(serde::de::Error::custom(format!(
				"unsupported ServerFnError version: {}",
				wire.version
			)));
		}
		Ok(Self {
			payload: ServerFnErrorPayload {
				kind: wire.kind.into(),
				status: wire.status,
				message: wire.message,
				field_errors: wire
					.field_errors
					.into_iter()
					.map(|error| ServerFnFieldError {
						field: error.field,
						message: error.message,
					})
					.collect(),
			},
		})
	}
}

impl From<ServerFnErrorKind> for WireServerFnErrorKind {
	fn from(kind: ServerFnErrorKind) -> Self {
		match kind {
			ServerFnErrorKind::Validation => Self::Validation,
			ServerFnErrorKind::Auth => Self::Auth,
			ServerFnErrorKind::Application => Self::Application,
			ServerFnErrorKind::Server => Self::Server,
			ServerFnErrorKind::Transport => Self::Transport,
			ServerFnErrorKind::Deserialization => Self::Deserialization,
		}
	}
}

impl From<WireServerFnErrorKind> for ServerFnErrorKind {
	fn from(kind: WireServerFnErrorKind) -> Self {
		match kind {
			WireServerFnErrorKind::Validation => Self::Validation,
			WireServerFnErrorKind::Auth => Self::Auth,
			WireServerFnErrorKind::Application => Self::Application,
			WireServerFnErrorKind::Server => Self::Server,
			WireServerFnErrorKind::Transport => Self::Transport,
			WireServerFnErrorKind::Deserialization => Self::Deserialization,
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
		assert_eq!(network_err.kind(), ServerFnErrorKind::Transport);
		assert_eq!(server_err.status(), Some(404));
	}

	#[rstest]
	fn test_server_fn_error_display() {
		// Arrange
		let network_err = ServerFnError::network("Connection timeout");
		let server_err = ServerFnError::server(500, "Internal error");

		// Act & Assert
		assert_eq!(network_err.to_string(), "Connection timeout");
		assert_eq!(server_err.to_string(), "Internal error");
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
	fn test_message_matches_display() {
		// Arrange
		let err = ServerFnError::application("Invalid choice_id");

		// Act
		let message = err.message();
		let display = err.to_string();

		// Assert
		assert_eq!(message, display);
		assert_eq!(message, "Invalid choice_id");
		assert_eq!(display, "Invalid choice_id");
	}

	#[test]
	fn validation_error_serializes_to_version_one_envelope() {
		let error = ServerFnError::validation_with_message(
			"Please correct the submitted values",
			[("choice_id", "Select a choice")],
		);

		let value: serde_json::Value = serde_json::to_value(&error).unwrap();

		assert_eq!(value["version"], 1);
		assert_eq!(value["kind"], "validation");
		assert_eq!(value["status"], 422);
		assert_eq!(value["message"], "Please correct the submitted values");
		assert_eq!(value["field_errors"][0]["field"], "choice_id");
		assert_eq!(value["field_errors"][0]["message"], "Select a choice");
	}

	#[test]
	fn structured_error_round_trips_without_enum_tags() {
		let original = ServerFnError::auth(403, "Permission denied");
		let bytes = serde_json::to_vec(&original).unwrap();
		let decoded: ServerFnError = serde_json::from_slice(&bytes).unwrap();

		assert_eq!(decoded, original);
		assert_eq!(decoded.kind(), ServerFnErrorKind::Auth);
		assert_eq!(decoded.status(), Some(403));
		assert_eq!(decoded.user_message(), "Permission denied");
		assert!(!String::from_utf8(bytes).unwrap().contains("Auth"));
	}

	#[test]
	fn transport_aliases_use_the_transport_kind() {
		assert_eq!(
			ServerFnError::network("offline").kind(),
			ServerFnErrorKind::Transport
		);
		assert_eq!(
			ServerFnError::serialization("bad input").kind(),
			ServerFnErrorKind::Transport
		);
	}

	#[test]
	fn unknown_version_is_rejected() {
		let value = serde_json::json!({
			"version": 2,
			"kind": "application",
			"status": null,
			"message": "unsupported",
			"field_errors": [],
		});

		assert!(serde_json::from_value::<ServerFnError>(value).is_err());
	}

	#[test]
	fn validation_supports_empty_and_multiple_field_errors() {
		// Arrange
		let empty = ServerFnError::validation();

		// Act
		let empty_field_errors = empty.field_errors();

		// Assert
		assert_eq!(empty.status(), Some(422));
		let expected_empty_field_errors: &[ServerFnFieldError] = &[];
		assert_eq!(empty_field_errors, expected_empty_field_errors);

		// Arrange
		let multiple = ServerFnError::validation_with_message(
			"Invalid form",
			[("name", "Required"), ("email", "Invalid address")],
		);

		// Act
		let multiple_field_errors = multiple.field_errors();

		// Assert
		assert_eq!(multiple_field_errors.len(), 2);
		assert_eq!(multiple_field_errors[0].field(), "name");
		assert_eq!(multiple_field_errors[0].message(), "Required");
		assert_eq!(multiple_field_errors[1].field(), "email");
		assert_eq!(multiple_field_errors[1].message(), "Invalid address");
	}

	#[rstest]
	#[case::application(r#"{"version":1,"kind":"application","status":null,"message":"Invalid choice_id","field_errors":[]}"#, "Invalid choice_id")]
	#[case::server(
		r#"{"version":1,"kind":"server","status":403,"message":"Forbidden","field_errors":[]}"#,
		"Forbidden"
	)]
	#[case::network(r#"{"version":1,"kind":"transport","status":null,"message":"Connection timeout","field_errors":[]}"#, "Connection timeout")]
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
		r#"Server error (500): {"version":1,"kind":"application","status":null,"message":"Invalid choice_id","field_errors":[]}"#,
		"Invalid choice_id"
	)]
	#[case::server_wrapping_network(
		r#"Server error (500): {"version":1,"kind":"transport","status":null,"message":"Connection lost","field_errors":[]}"#,
		"Connection lost"
	)]
	#[case::json_server_wrapping_application(
		r#"{"version":1,"kind":"server","status":500,"message":"{\"version\":1,\"kind\":\"application\",\"status\":null,\"message\":\"Invalid choice_id\",\"field_errors\":[]}","field_errors":[]}"#,
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
