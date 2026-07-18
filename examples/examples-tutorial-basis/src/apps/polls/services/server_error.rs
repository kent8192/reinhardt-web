//! Server-only vote error mapping.

use reinhardt::pages::server_fn::ServerFnError;
use std::fmt;

#[derive(Debug)]
pub(super) enum VoteRequestError {
	ChoiceNotFound,
	ChoiceQuestionMismatch,
	Framework(reinhardt::Error),
}

impl From<reinhardt::Error> for VoteRequestError {
	fn from(error: reinhardt::Error) -> Self {
		Self::Framework(error)
	}
}

impl fmt::Display for VoteRequestError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ChoiceNotFound => f.write_str("Choice not found"),
			Self::ChoiceQuestionMismatch => f.write_str("Choice does not belong to this question"),
			Self::Framework(error) => write!(f, "{error}"),
		}
	}
}

impl std::error::Error for VoteRequestError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Framework(error) => Some(error),
			Self::ChoiceNotFound | Self::ChoiceQuestionMismatch => None,
		}
	}
}

pub(super) fn map_vote_error(error: VoteRequestError) -> ServerFnError {
	match error {
		VoteRequestError::ChoiceNotFound => ServerFnError::server(404, "Choice not found"),
		VoteRequestError::ChoiceQuestionMismatch => {
			ServerFnError::server(400, "Choice does not belong to this question")
		}
		VoteRequestError::Framework(error) => {
			let status = error.status_code();
			tracing::error!(
				status,
				error = %error,
				"Vote request failed with a framework error"
			);

			match status {
				400 => ServerFnError::server(400, "Invalid request"),
				401 => ServerFnError::server(401, "Authentication required"),
				403 => ServerFnError::server(403, "Permission denied"),
				404 => ServerFnError::server(404, "Resource not found"),
				405 => ServerFnError::server(405, "Method not allowed"),
				409 => ServerFnError::server(409, "Request conflict"),
				503 => ServerFnError::server(503, "Service temporarily unavailable"),
				_ => ServerFnError::server(500, "Internal server error"),
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt::core::exception::{DatabaseError, DatabaseErrorKind, Error};
	use rstest::rstest;

	#[rstest]
	#[case(DatabaseErrorKind::Connection, 503, "Service temporarily unavailable")]
	#[case(DatabaseErrorKind::Query, 500, "Internal server error")]
	fn framework_database_error_is_retained_server_side_and_redacted_for_clients(
		#[case] kind: DatabaseErrorKind,
		#[case] expected_status: u16,
		#[case] expected_message: &str,
	) {
		let sensitive_detail =
			"postgres://admin:secret@db.internal/polls SELECT * FROM private_votes";
		let framework_error = Error::from(DatabaseError::new(kind, sensitive_detail));
		let vote_error = VoteRequestError::from(framework_error);

		let source = std::error::Error::source(&vote_error)
			.expect("framework error should remain available as the server-side source");
		assert_eq!(
			source.to_string(),
			format!("Database error: {sensitive_detail}")
		);

		let client_error = map_vote_error(vote_error);
		match &client_error {
			ServerFnError::Server { status, message } => {
				assert_eq!(*status, expected_status);
				assert_eq!(message, expected_message);
			}
			other => panic!("unexpected client error variant: {other:?}"),
		}

		let serialized =
			serde_json::to_string(&client_error).expect("server function error should serialize");
		assert!(!serialized.contains(sensitive_detail));
	}
}
