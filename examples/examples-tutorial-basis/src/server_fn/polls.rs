//! Poll server functions
//!
//! These functions provide the server-side API for the polling application.

use crate::shared::types::{ChoiceInfo, QuestionInfo, VoteRequest};
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

// Server-only imports
#[cfg(native)]
use {
	crate::apps::users::models::User,
	crate::shared::forms::create_vote_form,
	reinhardt::Model,
	reinhardt::db::orm::{FilterOperator, FilterValue},
	reinhardt::forms::wasm_compat::{FormExt, FormMetadata},
	reinhardt::middleware::session::SessionData,
};

/// Resolve the currently authenticated user from the session, or return a
/// 401 ServerFnError. Shared by every authenticated mutation handler below
/// so that the "load user_id from session, look up the row, 401 if absent"
/// dance lives in exactly one place.
#[cfg(native)]
async fn require_user(session: &SessionData) -> std::result::Result<User, ServerFnError> {
	let user_id = session
		.get::<i64>("user_id")
		.ok_or_else(|| ServerFnError::server(401, "Authentication required"))?;

	let user = User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::Int(user_id),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(401, "Authentication required"))?;

	if !user.is_active() {
		return Err(ServerFnError::server(403, "User account is inactive"));
	}

	Ok(user)
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
	use reinhardt::db::orm::{FilterOperator, FilterValue};

	// Get question
	let question_manager = Question::objects();
	let question = question_manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	// Get choices
	let choice_manager = Choice::objects();
	let choices = choice_manager
		.filter(
			Choice::field_question_id(),
			FilterOperator::Eq,
			FilterValue::Int(question_id),
		)
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
	use reinhardt::db::orm::{FilterOperator, FilterValue};

	// Get question
	let question_manager = Question::objects();
	let question = question_manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	// Get choices
	let choice_manager = Choice::objects();
	let choices = choice_manager
		.filter(
			Choice::field_question_id(),
			FilterOperator::Eq,
			FilterValue::Int(question_id),
		)
		.all()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?;

	// Calculate total votes
	let total_votes: i32 = choices.iter().map(|c| c.votes()).sum();

	let question_info = QuestionInfo::from(question);
	let choice_infos: Vec<ChoiceInfo> = choices.into_iter().map(ChoiceInfo::from).collect();

	Ok((question_info, choice_infos, total_votes))
}

/// Resolve a `Choice`'s parent `Question` id.
///
/// The choice CUD pages (`choice_edit`, `choice_delete_confirm`) only know the
/// `choice_id`, but their "Cancel" link should navigate back to the parent
/// poll's detail page. This thin lookup exposes the parent FK without forcing
/// the client to load the full question/choice payload.
#[server_fn]
pub async fn get_choice_parent(
	choice_id: i64,
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<i64, ServerFnError> {
	use crate::apps::polls::models::Choice;
	use reinhardt::Model;

	let choice = Choice::objects()
		.get(choice_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?
		.ok_or_else(|| ServerFnError::server(404, "Choice not found"))?;

	Ok(*choice.question_id())
}

/// Vote for a choice
///
/// Increments the vote count for the selected choice.
#[server_fn]
pub async fn vote(
	request: VoteRequest,
	#[inject] db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	vote_internal(request, db).await
}

/// Get vote form metadata for WASM client rendering
///
/// Returns form metadata with CSRF token for the voting form.
#[cfg(native)]
#[server_fn]
pub async fn get_vote_form_metadata() -> std::result::Result<FormMetadata, ServerFnError> {
	let form = create_vote_form();
	Ok(form.to_metadata())
}

/// Submit vote via form! macro
///
/// Wrapper function that accepts individual field values from form! macro's submit.
/// Converts String field values to the required types and calls the underlying vote function.
///
/// The trailing `_csrf_token: String` argument is supplied by `form!`'s
/// `strip_arguments` block (reinhardt-web#3971). Actual CSRF verification is
/// performed by the server-side CSRF middleware before this handler runs;
/// receiving the value here keeps the WASM client stub's positional argument
/// list aligned with the server signature.
#[server_fn]
pub async fn submit_vote(
	question_id: String,
	choice_id: String,
	_csrf_token: String,
	#[inject] db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;
	let choice_id: i64 = choice_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid choice_id"))?;

	let request = VoteRequest {
		question_id,
		choice_id,
	};

	// Reuse the existing vote logic
	vote_internal(request, db).await
}

/// Internal vote implementation (shared between vote and submit_vote)
#[cfg(native)]
async fn vote_internal(
	request: VoteRequest,
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

		// Increment vote count
		choice.vote();

		// Update in database
		let updated = choice_manager
			.update(&choice)
			.await
			.map_err(|e| anyhow::anyhow!(e.to_string()))?;

		Ok(updated)
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
// * Every form field is received as `String` because `form!` currently
//   serializes all fields as strings on submit. This is tracked upstream as
//   reinhardt-web#4397 — once that ships, the `String` + `.parse()` dance
//   below can be replaced with the typed signatures shown next to each
//   handler. The trailing `_csrf_token: String` parameter is appended by the
//   `form!` macro for non-GET forms; the CSRF middleware verifies it before
//   the handler runs.
// * The session is required: `require_user` returns a 401 to unauthenticated
//   clients before any database write happens.
// * For `update_question` and `delete_question`, ownership is enforced by
//   comparing `question.author_id()` with the current user's id; mismatched
//   ownership returns a 403.

/// Create a new question owned by the current user.
///
/// Ideal implementation (without the form! String workaround tracked in #4397):
///   pub async fn create_question(
///       question_text: String,
///       _csrf_token: String,
///       #[inject] _db: reinhardt::DatabaseConnection,
///       #[inject] session: SessionData,
///   ) -> std::result::Result<QuestionInfo, ServerFnError> { ... }
#[server_fn]
pub async fn create_question(
	question_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = require_user(&session).await?;

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
///
/// Ideal implementation (without the form! String workaround tracked in #4397):
///   pub async fn update_question(
///       question_id: i64,
///       question_text: String,
///       _csrf_token: String,
///       ...
///   ) -> std::result::Result<QuestionInfo, ServerFnError> { ... }
#[server_fn]
pub async fn update_question(
	question_id: String,
	question_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = require_user(&session).await?;

	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;

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
///
/// Ideal implementation (without the form! String workaround tracked in #4397):
///   pub async fn delete_question(
///       question_id: i64,
///       _csrf_token: String,
///       ...
///   ) -> std::result::Result<(), ServerFnError> { ... }
#[server_fn]
pub async fn delete_question(
	question_id: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<(), ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = require_user(&session).await?;

	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;

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
// caller authored it, then mutates the Choice. The same form! String ABI
// constraint (#4397) used by the Question CUD handlers above applies here.

/// Internal helper: load a Question by id and ensure the given user is its
/// author. Returns 401/403/404 as appropriate.
#[cfg(native)]
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
	question_id: String,
	choice_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;

	let user = require_user(&session).await?;
	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;
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
	choice_id: String,
	choice_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;

	let user = require_user(&session).await?;
	let choice_id: i64 = choice_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid choice_id"))?;

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
	choice_id: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<(), ServerFnError> {
	use crate::apps::polls::models::Choice;

	let user = require_user(&session).await?;
	let choice_id: i64 = choice_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid choice_id"))?;

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
