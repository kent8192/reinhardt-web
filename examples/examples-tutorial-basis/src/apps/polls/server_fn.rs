//! Poll server functions
//!
//! These functions provide the server-side API for the polling application.
use crate::shared::types::{ChoiceInfo, QuestionInfo, VoteRequest};
use reinhardt::pages::server_fn::{ServerFnError, server_fn};
#[cfg(native)]
use {
	crate::apps::polls::di::SessionUser, crate::apps::users::models::User, reinhardt::Model,
	reinhardt::di::Depends,
};
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
	let question_manager = Question::objects();
	let question = question_manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;
	use reinhardt::db::orm::{FilterOperator, FilterValue};
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
	let question_manager = Question::objects();
	let question = question_manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(e.to_string()))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;
	use reinhardt::db::orm::{FilterOperator, FilterValue};
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
	request: VoteRequest,
	#[inject] db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	vote_internal(request, db).await
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
/// Create a new question owned by the current user.
///
/// Ideal implementation (without the form! String workaround tracked in #4397):
///   pub async fn create_question(
///       question_text: String,
///       _csrf_token: String,
///       #[inject] _db: reinhardt::DatabaseConnection,
///       #[inject] session_user: Depends<SessionUser>,
///   ) -> std::result::Result<QuestionInfo, ServerFnError> { ... }
#[server_fn]
pub async fn create_question(
	question_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUser>,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;
	let user = session_user.require_active()?;
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
	#[inject] session_user: Depends<SessionUser>,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;
	let user = session_user.require_active()?;
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
	#[inject] session_user: Depends<SessionUser>,
) -> std::result::Result<(), ServerFnError> {
	use crate::apps::polls::models::Question;
	let user = session_user.require_active()?;
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
	#[inject] session_user: Depends<SessionUser>,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;
	let user = session_user.require_active()?;
	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;
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
	choice_id: String,
	choice_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUser>,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;
	let user = session_user.require_active()?;
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
	choice_id: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<SessionUser>,
) -> std::result::Result<(), ServerFnError> {
	use crate::apps::polls::models::Choice;
	let user = session_user.require_active()?;
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
	let _question = require_question_author(*choice.question_id(), user).await?;
	manager
		.delete(choice.id())
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;
	Ok(())
}
