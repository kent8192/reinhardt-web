use reinhardt::prelude::*;
use reinhardt::{endpoint, ViewResult};
use reinhardt::core::serde::json;
use reinhardt::db::orm::{FilterOperator, FilterValue, Manager};
use reinhardt::db::DatabaseConnection;
use json::json;
use serde::Deserialize;

use super::models::{Choice, Question};

/// Request body for voting
#[derive(Debug, Deserialize)]
pub struct VoteRequest {
	pub choice_id: i64,
}

/// Index view - List all polls
#[endpoint]
pub async fn index(
	_req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	let manager = Manager::<Question>::new();
	let questions = manager.all().all().await?;
	let latest_questions: Vec<_> = questions.into_iter().take(5).collect();

	let response_data = json!({
		"message": "Polls index",
		"polls": latest_questions
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Detail view - Show a specific poll
///
/// GET /polls/{question_id}/
#[endpoint]
pub async fn detail(
	req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	let question_id = req.path_params.get("question_id")
		.ok_or("Missing question_id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid question_id format")?;

	let question_manager = Manager::<Question>::new();
	let question = question_manager
		.get(question_id)
		.first()
		.await?
		.ok_or("Question not found")?;

	let choice_manager = Manager::<Choice>::new();
	let choices = choice_manager
		.filter(
			"question_id",
			FilterOperator::Eq,
			FilterValue::Int(question_id),
		)
		.all()
		.await?;

	let response_data = json!({
		"question": question,
		"choices": choices
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Results view - Show poll results
///
/// GET /polls/{question_id}/results/
#[endpoint]
pub async fn results(
	req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	let question_id = req.path_params.get("question_id")
		.ok_or("Missing question_id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid question_id format")?;

	let question_manager = Manager::<Question>::new();
	let question = question_manager
		.get(question_id)
		.first()
		.await?
		.ok_or("Question not found")?;

	let choice_manager = Manager::<Choice>::new();
	let choices = choice_manager
		.filter(
			"question_id",
			FilterOperator::Eq,
			FilterValue::Int(question_id),
		)
		.all()
		.await?;

	let total_votes: i32 = choices.iter().map(|c| c.votes).sum();

	let response_data = json!({
		"question": question,
		"choices": choices,
		"total_votes": total_votes
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Vote view - Handle voting
///
/// POST /polls/{question_id}/vote/
#[endpoint]
pub async fn vote(
	req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	let question_id = req.path_params.get("question_id")
		.ok_or("Missing question_id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid question_id format")?;

	// Parse request body
	let body_bytes = req.body();
	let vote_req: VoteRequest = json::from_slice(body_bytes)?;

	let choice_id = vote_req.choice_id;

	let choice_manager = Manager::<Choice>::new();
	let mut choice = choice_manager
		.get(choice_id)
		.first()
		.await?
		.ok_or("Choice not found")?;

	if choice.question_id != question_id {
		return Err("Choice does not belong to this question".into());
	}

	choice.votes += 1;
	let updated_choice = choice_manager.update(&choice).await?;

	let response_data = json!({
		"message": "Vote recorded successfully",
		"question_id": question_id,
		"choice_id": choice_id,
		"choice_text": updated_choice.choice_text,
		"new_vote_count": updated_choice.votes
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
