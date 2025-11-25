use reinhardt::prelude::*;
use reinhardt::{endpoint, db::DatabaseConnection, ViewResult};
use serde_json::json;
use std::sync::Arc;

/// Index view - List all polls
#[endpoint]
pub async fn index(
	_req: Request,
	#[inject] _db: Arc<DatabaseConnection>,
) -> ViewResult<Response> {
	todo!("Database integration - Manager::<Question>::new().all().all().await");

	// Placeholder response
	let response_data = json!({
		"message": "Polls index",
		"polls": []
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Detail view - Show a specific poll
#[endpoint]
pub async fn detail(
	req: Request,
	#[inject] _db: Arc<DatabaseConnection>,
) -> ViewResult<Response> {
	// Extract question_id from path parameters
	let question_id = req.path_params.get("question_id")
		.ok_or("Missing question_id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid question_id format")?;

	todo!("Database integration - Manager::<Question>::new().get(question_id).await and fetch choices");

	let response_data = json!({
		"message": "Poll detail",
		"question_id": question_id
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Results view - Show poll results
#[endpoint]
pub async fn results(
	req: Request,
	#[inject] _db: Arc<DatabaseConnection>,
) -> ViewResult<Response> {
	// Extract question_id from path parameters
	let question_id = req.path_params.get("question_id")
		.ok_or("Missing question_id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid question_id format")?;

	todo!("Database integration - Fetch question and choices with votes using Manager");

	let response_data = json!({
		"message": "Poll results",
		"question_id": question_id
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Vote view - Handle voting
#[endpoint]
pub async fn vote(
	mut req: Request,
	#[inject] _db: Arc<DatabaseConnection>,
) -> ViewResult<Response> {
	// Extract question_id from path parameters
	let question_id = req.path_params.get("question_id")
		.ok_or("Missing question_id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid question_id format")?;

	todo!("Database integration - Parse choice_id from body, increment vote using Manager::<Choice>");

	let response_data = json!({
		"message": "Vote recorded",
		"question_id": question_id
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
