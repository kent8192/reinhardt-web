//! Shared types between client and server
//!
//! These types are used for communication between the WASM client and the server.

use reinhardt::dto;
use serde::{Deserialize, Serialize};

/// Login request (DTO)
///
/// Sent from the WASM client to the server when submitting the login form.
///
/// The `#[dto]` macro emits `Validate` for both native and WASM builds so
/// clients can run the same field checks before submission, with the server
/// re-running them on receipt.
#[dto]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
	#[validate(length(
		min = 1,
		max = 150,
		message = "Username must be between 1 and 150 characters"
	))]
	pub username: String,

	#[validate(length(min = 1, message = "Password must not be empty"))]
	pub password: String,
}

/// Register request (DTO)
///
/// Sent from the WASM client to the server when submitting the sign-up form.
/// `password_confirmation` is matched against `password` server-side; both
/// fields travel in the clear over HTTPS just like the login form and are
/// never persisted — only the Argon2 hash of `password` is stored.
///
/// Validation wiring is handled by `#[dto]`. Field-level rules (length /
/// non-empty) run on the client and server, while the password-confirmation
/// equality check remains a dedicated
/// [`RegisterRequest::validate_passwords_match`] helper for server-side
/// business validation.
#[dto]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
	#[validate(length(
		min = 1,
		max = 150,
		message = "Username must be between 1 and 150 characters"
	))]
	pub username: String,

	#[validate(length(min = 8, message = "Password must be at least 8 characters"))]
	pub password: String,

	#[validate(length(
		min = 8,
		message = "Password confirmation must be at least 8 characters"
	))]
	pub password_confirmation: String,
}

#[cfg(server)]
impl RegisterRequest {
	/// Confirm that `password` and `password_confirmation` match.
	///
	/// Kept out of the derived `Validate` because the validator crate's
	/// `must_match` argument is positional (string field name), brittle
	/// across versions, and produces an awkward error message at the
	/// struct level rather than against the confirmation field. The
	/// server function calls this immediately after `request.validate()`
	/// so the two checks surface as the same kind of `ServerFnError`.
	pub fn validate_passwords_match(&self) -> Result<(), &'static str> {
		if self.password == self.password_confirmation {
			Ok(())
		} else {
			Err("Passwords do not match")
		}
	}
}

/// Vote request
///
/// Sent from client when user votes for a choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
	pub question_id: i64,
	pub choice_id: i64,
}
