//! Poll server functions
//!
//! These functions provide the server-side API for the polling application.

use crate::shared::types::{ChoiceInfo, QuestionInfo};
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

// Server-only imports. Each addition here MUST also be wired in `[cfg(server)]`
// at the call site — see the per-handler `use` blocks below for examples.
#[cfg(server)]
use {
	crate::apps::users::models::User,
	reinhardt::Model,
	reinhardt::di::{Depends, FactoryOutput, injectable, injectable_key},
	reinhardt::middleware::session::{SessionData, USER_ID_SESSION_KEY},
};

/// Error variants for the session-based user lookup factory.
///
/// Three failure modes are distinguished so handlers can surface the
/// correct HTTP status and so that an operational outage is not silently
/// rewritten into a fake 401.
#[cfg(server)]
#[derive(Clone, Debug)]
pub enum SessionError {
	Anonymous,
	Inactive,
	Unavailable(String),
}

/// Convert a `SessionError` reference to a `ServerFnError` with the
/// appropriate HTTP status code.
#[cfg(server)]
impl From<&SessionError> for ServerFnError {
	fn from(err: &SessionError) -> Self {
		match err {
			SessionError::Anonymous => ServerFnError::server(401, "Authentication required"),
			SessionError::Inactive => ServerFnError::server(403, "User account is inactive"),
			SessionError::Unavailable(_) => {
				ServerFnError::server(500, "User lookup temporarily unavailable")
			}
		}
	}
}

#[cfg(server)]
#[injectable_key]
pub struct SessionUserKey;

#[cfg(server)]
#[injectable(scope = "request")]
async fn session_user_factory(
	#[inject] session: SessionData,
) -> FactoryOutput<SessionUserKey, Result<User, SessionError>> {
	let Some(user_id) = session.get::<i64>(USER_ID_SESSION_KEY) else {
		return FactoryOutput::new(Err(SessionError::Anonymous));
	};

	let user = match User::objects()
		.filter(User::field_id().eq(user_id))
		.first()
		.await
	{
		Ok(Some(user)) => user,
		Ok(None) => return FactoryOutput::new(Err(SessionError::Anonymous)),
		Err(err) => {
			::tracing::warn!(
				user_id = user_id,
				error = %err,
				"session_user_factory: user lookup failed"
			);
			return FactoryOutput::new(Err(SessionError::Unavailable(err.to_string())));
		}
	};

	if user.is_active {
		FactoryOutput::new(Ok(user))
	} else {
		FactoryOutput::new(Err(SessionError::Inactive))
	}
}

// WASM-only imports
// (None needed - all forms logic is server-side)

/// Get all questions (latest 5)
///
/// Returns the 5 most recent poll questions.
#[server_fn]
pub async fn get_questions(
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<Vec<QuestionInfo>, ServerFnError> {
	use crate::apps::polls::models::Question;
	use reinhardt::Model;

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
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>), ServerFnError> {
	use crate::apps::polls::models::{Choice, Question};
	use reinhardt::Model;

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
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>, i32), ServerFnError> {
	use crate::apps::polls::models::{Choice, Question};
	use reinhardt::Model;

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
	#[inject] db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
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
	#[inject] db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	let request = crate::shared::types::VoteRequest {
		question_id,
		choice_id,
	};

	// Reuse the existing vote logic
	vote_internal(request, db).await
}

/// Internal vote implementation (shared between vote and submit_vote)
#[cfg(server)]
async fn vote_internal(
	request: crate::shared::types::VoteRequest,
	db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;
	use reinhardt::Model;
	use reinhardt::atomic;

	// Wrap read-modify-write in a transaction to prevent race conditions
	let updated_choice = atomic(&db, || async {
		let choice_manager = Choice::objects();

		// Get the choice
		let mut choice = choice_manager
			.get(request.choice_id)
			.first()
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?
			.ok_or_else(|| anyhow::anyhow!("Choice not found"))?;

		// Verify the choice belongs to the question
		if *choice.question_id() != request.question_id {
			return Err(anyhow::anyhow!("Choice does not belong to this question"));
		}

		// Increment vote count AND persist in a single call.
		// `Choice::vote()` returns `Result<(), exception::Error>` after
		// invoking `Model::save()` internally — no separate
		// `manager.update(&choice).await` is needed.
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
// * Authentication is required: keyed `Depends` resolves the
//   request-scoped factory in this module and exposes
//   `.as_ref().map_err(ServerFnError::from)?` for the 401/403/500 surface.
// * For `update_question` and `delete_question`, ownership is enforced by
//   comparing `question.author_id()` with the current user's id; mismatched
//   ownership returns a 403.

/// Create a new question owned by the current user.
#[server_fn]
pub async fn create_question(
	question_text: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUserKey, Result<User, SessionError>>,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;

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
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUserKey, Result<User, SessionError>>,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;

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
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUserKey, Result<User, SessionError>>,
) -> std::result::Result<(), ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;

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

/// Internal helper: load a Question by id and ensure the given user is its
/// author. Returns 401/403/404 as appropriate.
#[cfg(server)]
async fn require_question_author(
	question_id: i64,
	user: &User,
) -> std::result::Result<crate::apps::polls::models::Question, ServerFnError> {
	use crate::apps::polls::models::Question;

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
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUserKey, Result<User, SessionError>>,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;
	let question = require_question_author(question_id, user).await?;

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
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUserKey, Result<User, SessionError>>,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;
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

	let _question = require_question_author(*choice.question_id(), user).await?;

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
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUserKey, Result<User, SessionError>>,
) -> std::result::Result<(), ServerFnError> {
	use crate::apps::polls::models::Choice;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;
	let manager = Choice::objects();
	let choice = manager
		.get(choice_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Choice not found"))?;

	let _question = require_question_author(*choice.question_id(), user).await?;

	manager
		.delete(choice.id())
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(())
}
