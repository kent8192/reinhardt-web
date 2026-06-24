//! Server-only polls services.

use crate::apps::polls::server::models::{Choice, ChoiceInfo};
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
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?
			.ok_or_else(|| anyhow::anyhow!("Choice not found"))?;

		if *choice.question_id() != request.question_id {
			return Err(anyhow::anyhow!("Choice does not belong to this question"));
		}

		choice
			.vote()
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?;

		Ok(choice)
	})
	.await
	.map_err(|e| ServerFnError::application(e.to_string()))?;

	Ok(ChoiceInfo::from(updated_choice))
}
