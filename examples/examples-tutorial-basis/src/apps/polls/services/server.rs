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
}

impl fmt::Display for VoteRequestError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ChoiceNotFound => f.write_str("Choice not found"),
			Self::ChoiceQuestionMismatch => f.write_str("Choice does not belong to this question"),
		}
	}
}

impl std::error::Error for VoteRequestError {}

fn map_vote_error(error: anyhow::Error) -> ServerFnError {
	match error.downcast_ref::<VoteRequestError>() {
		Some(VoteRequestError::ChoiceNotFound) => ServerFnError::server(404, "Choice not found"),
		Some(VoteRequestError::ChoiceQuestionMismatch) => {
			ServerFnError::server(400, "Choice does not belong to this question")
		}
		None => ServerFnError::application(error.to_string()),
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
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?
			.ok_or_else(|| anyhow::Error::new(VoteRequestError::ChoiceNotFound))?;

		if *choice.question_id() != request.question_id {
			return Err(anyhow::Error::new(VoteRequestError::ChoiceQuestionMismatch));
		}

		choice
			.vote()
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?;

		Ok(choice)
	})
	.await
	.map_err(map_vote_error)?;

	Ok(ChoiceInfo::from(updated_choice))
}
