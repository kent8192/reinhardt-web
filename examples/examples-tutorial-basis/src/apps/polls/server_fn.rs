//! Poll server functions
//!
//! These functions provide the server-side API for the polling application.

use crate::apps::polls::server::models::{ChoiceInfo, QuestionInfo};
use reinhardt::pages::server_fn::{ServerFnError, server_fn};
use std::result::Result;

#[cfg(server)]
use {
	crate::apps::polls::server::models::{Choice, Question},
	crate::apps::polls::services::server::vote_internal,
	crate::apps::users::server::models::User,
	crate::shared::types::VoteRequest,
	reinhardt::CurrentUser,
	reinhardt::DatabaseConnection,
	reinhardt::Model,
};

/// Get all questions (latest 5)
///
/// Returns the 5 most recent poll questions.
#[server_fn]
pub async fn get_questions(
	#[inject] _db: DatabaseConnection,
) -> Result<Vec<QuestionInfo>, ServerFnError> {
	let manager = Question::objects();
	let questions = manager
		.all()
		.all()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?;

	// Take latest 5 questions
	let latest: Vec<QuestionInfo> = questions
		.into_iter()
		.take(5)
		.map(QuestionInfo::from)
		.collect();

	Ok(latest)
}

/// Get question detail with choices
///
/// Returns the question and all its choices.
#[server_fn]
pub async fn get_question_detail(
	question_id: i64,
	#[inject] _db: DatabaseConnection,
) -> Result<(QuestionInfo, Vec<ChoiceInfo>), ServerFnError> {
	// Get question
	let question_manager = Question::objects();
	let question = question_manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	// Get choices using the typed builder.
	let choice_manager = Choice::objects();
	let choices = choice_manager
		.filter(Choice::field_question_id().eq(question_id))
		.all()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?;

	let question_info = QuestionInfo::from(question);
	let choice_infos: Vec<ChoiceInfo> = choices.into_iter().map(ChoiceInfo::from).collect();

	Ok((question_info, choice_infos))
}

/// Get question results
///
/// Returns the question and all its choices with vote counts.
#[server_fn]
pub async fn get_question_results(
	question_id: i64,
	#[inject] _db: DatabaseConnection,
) -> Result<(QuestionInfo, Vec<ChoiceInfo>, i32), ServerFnError> {
	// Get question
	let question_manager = Question::objects();
	let question = question_manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	// Get choices using the typed builder.
	let choice_manager = Choice::objects();
	let choices = choice_manager
		.filter(Choice::field_question_id().eq(question_id))
		.all()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?;

	// Calculate total votes
	let total_votes: i32 = choices.iter().map(|c| c.votes()).sum();

	let question_info = QuestionInfo::from(question);
	let choice_infos: Vec<ChoiceInfo> = choices.into_iter().map(ChoiceInfo::from).collect();

	Ok((question_info, choice_infos, total_votes))
}

/// Vote for a choice
///
/// Increments the vote count for the selected choice.
#[server_fn]
pub async fn vote(
	request: crate::shared::types::VoteRequest,
	#[inject] db: DatabaseConnection,
) -> Result<ChoiceInfo, ServerFnError> {
	vote_internal(request, db).await
}

/// Submit vote via form! macro
///
/// Wrapper function that accepts individual typed field values from form!'s submit
/// path and calls the underlying vote function.
///
/// CSRF is supplied by the `#[server_fn]` client stub through `X-CSRFToken`
/// and verified by middleware before this handler runs.
#[server_fn]
pub async fn submit_vote(
	question_id: i64,
	choice_id: i64,
	#[inject] db: DatabaseConnection,
) -> Result<ChoiceInfo, ServerFnError> {
	let request = VoteRequest {
		question_id,
		choice_id,
	};

	// Reuse the existing vote logic
	vote_internal(request, db).await
}

// =========================================================================
// Question CUD (Phase 2)
// =========================================================================
//
// All three mutations below follow the same conventions:
//
// * `form!` submits field values with the types declared by the field
//   definitions, so `HiddenField<i64>` reaches these handlers as `i64`.
//   CSRF is supplied by the `#[server_fn]` client stub through `X-CSRFToken`
//   and verified by middleware.
// * Authentication is required: `CurrentUser<User>` resolves the full user
//   through the framework auth extractor before the handler body runs.
// * For `update_question` and `delete_question`, ownership is enforced by
//   comparing `question.author_id()` with the current user's id; mismatched
//   ownership returns a 403.

/// Create a new question owned by the current user.
#[server_fn]
pub async fn create_question(
	question_text: String,
	#[inject] _db: DatabaseConnection,
	#[inject] CurrentUser(user): CurrentUser<User>,
) -> Result<QuestionInfo, ServerFnError> {
	require_active_user(&user)?;

	let trimmed = question_text.trim();
	if trimmed.is_empty() || trimmed.len() > 200 {
		return Err(ServerFnError::server(
			400,
			"Question text must be between 1 and 200 characters",
		));
	}

	let manager = Question::objects();
	let new_question = Question::build()
		.question_text(trimmed)
		.author(user.id())
		.finish();
	let saved = manager
		.create(&new_question)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(QuestionInfo::from(saved))
}

/// Update a question's text. Only the author may update.
#[server_fn]
pub async fn update_question(
	question_id: i64,
	question_text: String,
	#[inject] _db: DatabaseConnection,
	#[inject] CurrentUser(user): CurrentUser<User>,
) -> Result<QuestionInfo, ServerFnError> {
	require_active_user(&user)?;

	let trimmed = question_text.trim();
	if trimmed.is_empty() || trimmed.len() > 200 {
		return Err(ServerFnError::server(
			400,
			"Question text must be between 1 and 200 characters",
		));
	}

	let manager = Question::objects();
	let mut question = manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	if *question.author_id() != user.id() {
		return Err(ServerFnError::server(
			403,
			"Only the question's author can edit it",
		));
	}

	question.question_text = trimmed.to_string();

	let updated = manager
		.update(&question)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(QuestionInfo::from(updated))
}

/// Delete a question. Only the author may delete.
#[server_fn]
pub async fn delete_question(
	question_id: i64,
	#[inject] _db: DatabaseConnection,
	#[inject] CurrentUser(user): CurrentUser<User>,
) -> Result<(), ServerFnError> {
	require_active_user(&user)?;

	let manager = Question::objects();
	let question = manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	if *question.author_id() != user.id() {
		return Err(ServerFnError::server(
			403,
			"Only the question's author can delete it",
		));
	}

	manager
		.delete(question.id())
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(())
}

// =========================================================================
// Choice CUD (Phase 3)
// =========================================================================
//
// Choice has no own author field — ownership is derived from the parent
// Question. Each mutation loads the Question first, verifies that the
// caller authored it, then mutates the Choice.

#[cfg(server)]
fn require_active_user(user: &User) -> Result<(), ServerFnError> {
	if user.is_active {
		Ok(())
	} else {
		Err(ServerFnError::server(403, "User account is inactive"))
	}
}

/// Internal helper: load a Question by id and ensure the given user is its
/// author. Returns 401/403/404 as appropriate.
#[cfg(server)]
async fn require_question_author(question_id: i64, user: &User) -> Result<Question, ServerFnError> {
	let question = Question::objects()
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	if *question.author_id() != user.id() {
		return Err(ServerFnError::server(
			403,
			"Only the question's author can manage its choices",
		));
	}

	Ok(question)
}

/// Create a new Choice on a Question. Only the question's author may add
/// choices.
#[server_fn]
pub async fn create_choice(
	question_id: i64,
	choice_text: String,
	#[inject] _db: DatabaseConnection,
	#[inject] CurrentUser(user): CurrentUser<User>,
) -> Result<ChoiceInfo, ServerFnError> {
	require_active_user(&user)?;
	let question = require_question_author(question_id, &user).await?;

	let trimmed = choice_text.trim();
	if trimmed.is_empty() || trimmed.len() > 200 {
		return Err(ServerFnError::server(
			400,
			"Choice text must be between 1 and 200 characters",
		));
	}

	let manager = Choice::objects();
	let new_choice = Choice::build()
		.choice_text(trimmed)
		.votes(0)
		.question(question.id())
		.finish();
	let saved = manager
		.create(&new_choice)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(ChoiceInfo::from(saved))
}

/// Update a Choice's text. Only the parent question's author may update.
#[server_fn]
pub async fn update_choice(
	choice_id: i64,
	choice_text: String,
	#[inject] _db: DatabaseConnection,
	#[inject] CurrentUser(user): CurrentUser<User>,
) -> Result<ChoiceInfo, ServerFnError> {
	require_active_user(&user)?;
	let trimmed = choice_text.trim();
	if trimmed.is_empty() || trimmed.len() > 200 {
		return Err(ServerFnError::server(
			400,
			"Choice text must be between 1 and 200 characters",
		));
	}

	let manager = Choice::objects();
	let mut choice = manager
		.get(choice_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Choice not found"))?;

	let _question = require_question_author(*choice.question_id(), &user).await?;

	choice.choice_text = trimmed.to_string();
	let updated = manager
		.update(&choice)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(ChoiceInfo::from(updated))
}

/// Delete a Choice. Only the parent question's author may delete.
#[server_fn]
pub async fn delete_choice(
	choice_id: i64,
	#[inject] _db: DatabaseConnection,
	#[inject] CurrentUser(user): CurrentUser<User>,
) -> Result<(), ServerFnError> {
	require_active_user(&user)?;
	let manager = Choice::objects();
	let choice = manager
		.get(choice_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Choice not found"))?;

	let _question = require_question_author(*choice.question_id(), &user).await?;

	manager
		.delete(choice.id())
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(())
}
