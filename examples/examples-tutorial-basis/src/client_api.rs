//! Cross-target wrappers for client-side server function calls.
//!
//! Browser builds delegate to the generated `#[server_fn]` client stubs. Native
//! builds keep app client modules compilable without requiring injected runtime
//! values in UI-only data loaders.

use crate::shared::types::{ChoiceInfo, QuestionInfo, UserInfo};
use reinhardt::pages::server_fn::ServerFnError;

#[cfg(client)]
pub async fn get_questions() -> std::result::Result<Vec<QuestionInfo>, ServerFnError> {
	crate::apps::polls::server_fn::get_questions().await
}

#[cfg(not(client))]
pub async fn get_questions() -> std::result::Result<Vec<QuestionInfo>, ServerFnError> {
	Ok(Vec::new())
}

#[cfg(client)]
pub async fn get_question_detail(
	question_id: i64,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>), ServerFnError> {
	crate::apps::polls::server_fn::get_question_detail(question_id).await
}

#[cfg(not(client))]
pub async fn get_question_detail(
	_question_id: i64,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>), ServerFnError> {
	Err(ServerFnError::server(
		501,
		"Client data loader is not available on this target",
	))
}

#[cfg(client)]
pub async fn get_question_results(
	question_id: i64,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>, i32), ServerFnError> {
	crate::apps::polls::server_fn::get_question_results(question_id).await
}

#[cfg(not(client))]
pub async fn get_question_results(
	_question_id: i64,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>, i32), ServerFnError> {
	Err(ServerFnError::server(
		501,
		"Client data loader is not available on this target",
	))
}

#[cfg(client)]
pub async fn current_user() -> std::result::Result<Option<UserInfo>, ServerFnError> {
	crate::apps::users::server_fn::current_user().await
}

#[cfg(not(client))]
pub async fn current_user() -> std::result::Result<Option<UserInfo>, ServerFnError> {
	Ok(None)
}
