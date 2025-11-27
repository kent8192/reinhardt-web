use chrono::Utc;
use reinhardt::prelude::*;
use reinhardt::{endpoint, db::DatabaseConnection, ViewResult};
use serde_json::json;
use validator::Validate;

use super::models::Snippet;
use super::serializers::{SnippetResponse, SnippetSerializer};

/// Helper function to get sample snippets for demonstration
fn get_sample_snippets() -> Vec<Snippet> {
	vec![
		Snippet {
			id: 1,
			title: "Hello World".to_string(),
			code: "fn main() {\n    println!(\"Hello, World!\");\n}".to_string(),
			language: "rust".to_string(),
			created_at: Utc::now(),
		},
		Snippet {
			id: 2,
			title: "Fibonacci".to_string(),
			code: "def fibonacci(n):\n    if n <= 1:\n        return n\n    return fibonacci(n-1) + fibonacci(n-2)".to_string(),
			language: "python".to_string(),
			created_at: Utc::now(),
		},
		Snippet {
			id: 3,
			title: "Quick Sort".to_string(),
			code: "function quickSort(arr) {\n  if (arr.length <= 1) return arr;\n  const pivot = arr[0];\n  const left = arr.slice(1).filter(x => x < pivot);\n  const right = arr.slice(1).filter(x => x >= pivot);\n  return [...quickSort(left), pivot, ...quickSort(right)];\n}".to_string(),
			language: "javascript".to_string(),
			created_at: Utc::now(),
		},
	]
}

/// List all snippets
#[endpoint]
pub async fn list(
	_req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	// Production ORM usage:
	// let snippets = Manager::<Snippet>::new().all().await?;

	// Demo mode: Use sample data
	let snippets = get_sample_snippets();
	let snippet_responses: Vec<SnippetResponse> = snippets
		.iter()
		.map(SnippetResponse::from_model)
		.collect();

	let response_data = json!({
		"snippets": snippet_responses
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Create a new snippet
#[endpoint]
pub async fn create(
	mut req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	// Parse request body
	let body_bytes = req.body();
	let serializer: SnippetSerializer = serde_json::from_slice(body_bytes)?;

	// Validate
	serializer.validate()?;

	// Production ORM usage:
	// let snippet = Manager::<Snippet>::new().create(Snippet {
	//     id: 0, // Auto-generated
	//     title: serializer.title.clone(),
	//     code: serializer.code.clone(),
	//     language: serializer.language.clone(),
	//     created_at: Utc::now(),
	// }).await?;

	// Demo mode: Create a mock snippet with a sample ID
	let snippet = Snippet {
		id: 4, // Mock ID for demo
		title: serializer.title.clone(),
		code: serializer.code.clone(),
		language: serializer.language.clone(),
		created_at: Utc::now(),
	};

	let response_data = json!({
		"message": "Snippet created",
		"snippet": SnippetResponse::from_model(&snippet)
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::CREATED)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Retrieve a specific snippet
#[endpoint]
pub async fn retrieve(
	req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	// Extract snippet_id from path parameters
	let snippet_id = req
		.path_params
		.get("id")
		.ok_or("Missing id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid id format")?;

	// Production ORM usage:
	// let snippet = Manager::<Snippet>::new().get(snippet_id).await?;

	// Demo mode: Find in sample data
	let snippets = get_sample_snippets();
	let snippet = snippets
		.iter()
		.find(|s| s.id == snippet_id)
		.ok_or("Snippet not found")?;

	let response_data = json!({
		"snippet": SnippetResponse::from_model(snippet)
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Update a snippet
#[endpoint]
pub async fn update(
	mut req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	// Extract snippet_id from path parameters
	let snippet_id = req
		.path_params
		.get("id")
		.ok_or("Missing id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid id format")?;

	// Parse request body
	let body_bytes = req.body();
	let serializer: SnippetSerializer = serde_json::from_slice(body_bytes)?;

	// Validate
	serializer.validate()?;

	// Production ORM usage:
	// let snippet = Manager::<Snippet>::new().update(snippet_id, |s| {
	//     s.title = serializer.title.clone();
	//     s.code = serializer.code.clone();
	//     s.language = serializer.language.clone();
	// }).await?;

	// Demo mode: Verify snippet exists and return updated version
	let snippets = get_sample_snippets();
	let existing = snippets
		.iter()
		.find(|s| s.id == snippet_id)
		.ok_or("Snippet not found")?;

	// Create updated snippet
	let updated_snippet = Snippet {
		id: existing.id,
		title: serializer.title.clone(),
		code: serializer.code.clone(),
		language: serializer.language.clone(),
		created_at: existing.created_at,
	};

	let response_data = json!({
		"message": "Snippet updated",
		"snippet": SnippetResponse::from_model(&updated_snippet)
	});

	let json = serde_json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Delete a snippet
#[endpoint]
pub async fn delete(
	req: Request,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	// Extract snippet_id from path parameters
	let snippet_id = req
		.path_params
		.get("id")
		.ok_or("Missing id parameter")?
		.parse::<i64>()
		.map_err(|_| "Invalid id format")?;

	// Production ORM usage:
	// Manager::<Snippet>::new().delete(snippet_id).await?;

	// Demo mode: Verify snippet exists
	let snippets = get_sample_snippets();
	let _existing = snippets
		.iter()
		.find(|s| s.id == snippet_id)
		.ok_or("Snippet not found")?;

	// Return 204 No Content for successful deletion
	Ok(Response::new(StatusCode::NO_CONTENT))
}
