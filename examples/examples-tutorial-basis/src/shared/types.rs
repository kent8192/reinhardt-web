//! Shared types between client and server
//!
//! These types are used for communication between the WASM client and the server.
//! All types must be serializable with serde.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User information (DTO)
///
/// Returned by the authentication server functions. Mirrors the public-facing
/// subset of `apps::users::models::User`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
	pub id: i64,
	pub username: String,
	pub is_active: bool,
}

/// Login request (DTO)
///
/// Sent from the WASM client to the server when submitting the login form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
	pub username: String,
	pub password: String,
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
		UserInfo {
			id: user.id(),
			username: user.username().to_string(),
			is_active: user.is_active(),
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
