//! Server-only polls services.

use super::server_error::{VoteRequestError, map_vote_error};
use crate::apps::polls::models::{Choice, ChoiceInfo};
use crate::shared::types::VoteRequest;
use reinhardt::pages::server_fn::ServerFnError;
use reinhardt::{DatabaseConnection, Model, atomic};
use std::result::Result;

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
