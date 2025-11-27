use reinhardt::db::orm::{FilterOperator, FilterValue, Manager};
use reinhardt::prelude::*;
use reinhardt::{ViewResult, endpoint};
use serde_json::json;

use super::models::{Choice, Question};

/// Index view - List all polls
#[endpoint]
pub async fn index(_req: Request) -> ViewResult<Response> {
	let manager = Manager::<Question>::new();
	let questions = manager.all().all().await?;
	let latest_questions: Vec<_> = questions.into_iter().take(5).collect();

	let response_data = json!({
		"message": "Polls index",
		"polls": latest_questions
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Detail view - Show a specific poll
#[endpoint]
pub async fn detail(req: Request) -> ViewResult<Response> {
	let question_id = req
		.path_params
		.get("question_id")
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

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Results view - Show poll results
#[endpoint]
pub async fn results(req: Request) -> ViewResult<Response> {
	let question_id = req
		.path_params
		.get("question_id")
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

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Vote view - Handle voting
#[endpoint]
pub async fn vote(req: Request) -> ViewResult<Response> {
	let question_id = req
		.path_params
		.get("question_id")
		.ok_or("Missing question_id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid question_id format")?;

	let body = req.body();
	let body_str = String::from_utf8(body.to_vec()).map_err(|_| "Invalid UTF-8 in body")?;
	let vote_data: serde_json::Value =
		serde_json::from_str(&body_str).map_err(|_| "Invalid JSON in body")?;
	let choice_id = vote_data
		.get("choice_id")
		.and_then(|v| v.as_i64())
		.ok_or("Missing or invalid choice_id in body")?;

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

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
