//! Shared types between client and server
//!
//! These types are used for communication between the WASM client and the server.
//! All types must be serializable with serde.

use chrono::{DateTime, Utc};
use reinhardt::shared_schema;
use serde::{Deserialize, Serialize};

/// User information (DTO)
///
/// Returned by the authentication server functions. Mirrors the public-facing
/// subset of `apps::users::models::User`.
#[shared_schema]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
	pub id: i64,
	pub username: String,
	pub is_active: bool,
}

/// Login request (DTO)
///
/// Sent from the WASM client to the server when submitting the login form.
///
/// The `#[shared_schema]` macro emits `Validate` (and an OpenAPI `Schema`)
/// derive behind `cfg(native)` so the WASM client does not pull in the
/// validator-crate machinery — the server is the only side that runs
/// `request.validate()` before hitting the database.
#[shared_schema]
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
/// Validation gating is handled by `#[shared_schema]` (same rationale as on
/// [`LoginRequest`]). Field-level rules (length / non-empty) run through
/// `request.validate()`; the password-confirmation equality check is
/// expressed as a dedicated [`RegisterRequest::validate_passwords_match`]
/// helper because the validator crate's `must_match` is brittle across
/// versions (mirroring the pattern in `examples-twitter`).
#[shared_schema]
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

#[cfg(native)]
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

/// Question information (DTO)
///
/// This is a Data Transfer Object that represents a poll question.
/// It's used for client-server communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInfo {
	pub id: i64,
	pub question_text: String,
	pub pub_date: DateTime<Utc>,
	/// User ID of the question's author. Used by the client to decide
	/// whether to render the Edit / Delete buttons; the server re-checks
	/// ownership before performing any mutation.
	pub author_id: i64,
}

/// Choice information (DTO)
///
/// This is a Data Transfer Object that represents a poll choice.
/// It's used for client-server communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceInfo {
	pub id: i64,
	pub question_id: i64,
	pub choice_text: String,
	pub votes: i32,
}

/// Vote request
///
/// Sent from client when user votes for a choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
	pub question_id: i64,
	pub choice_id: i64,
}

// Server-side conversions from models to DTOs
// These are only compiled on the server side

#[cfg(native)]
impl From<crate::apps::polls::models::Question> for QuestionInfo {
	fn from(question: crate::apps::polls::models::Question) -> Self {
		QuestionInfo {
			id: question.id(),
			question_text: question.question_text().to_string(),
			pub_date: question.pub_date(),
			author_id: *question.author_id(),
		}
	}
}

#[cfg(native)]
impl From<crate::apps::users::models::User> for UserInfo {
	fn from(user: crate::apps::users::models::User) -> Self {
		// `#[user]` injects `skip_getter` on the convention fields
		// (username, is_active, …), so we read them as struct fields rather
		// than through accessor methods.
		UserInfo {
			id: user.id(),
			username: user.username.clone(),
			is_active: user.is_active,
		}
	}
}

#[cfg(native)]
impl From<crate::apps::polls::models::Choice> for ChoiceInfo {
	fn from(choice: crate::apps::polls::models::Choice) -> Self {
		ChoiceInfo {
			id: choice.id(),
			question_id: *choice.question_id(),
			choice_text: choice.choice_text().to_string(),
			votes: choice.votes(),
		}
	}
}
