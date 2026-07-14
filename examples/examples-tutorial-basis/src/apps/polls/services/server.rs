//! Server-only polls services.

use crate::apps::polls::models::{Choice, ChoiceInfo};
use crate::shared::types::VoteRequest;
use reinhardt::pages::server_fn::ServerFnError;
use reinhardt::{DatabaseConnection, Model, atomic};
use std::fmt;
use std::result::Result;

#[derive(Debug)]
enum VoteRequestError {
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

fn map_vote_error(error: VoteRequestError) -> ServerFnError {
	match error {
		VoteRequestError::ChoiceNotFound => ServerFnError::server(404, "Choice not found"),
		VoteRequestError::ChoiceQuestionMismatch => {
			ServerFnError::server(400, "Choice does not belong to this question")
		}
		VoteRequestError::Framework(error) => ServerFnError::application(error.to_string()),
	}
}

/// Shared vote implementation used by both typed and form-backed server
/// functions.
pub async fn vote_internal(
	request: VoteRequest,
	db: DatabaseConnection,
) -> Result<ChoiceInfo, ServerFnError> {
	let updated_choice = atomic(&db, || async {
		let choice_manager = Choice::objects();

		let mut choice = choice_manager
			.get(request.choice_id)
			.first()
			.await?
			.ok_or(VoteRequestError::ChoiceNotFound)?;

		if choice.question_id() != request.question_id {
			return Err(VoteRequestError::ChoiceQuestionMismatch);
		}

		choice.vote().await?;

		Ok(choice)
	})
	.await
	.map_err(map_vote_error)?;

	Ok(ChoiceInfo::from(updated_choice))
}
