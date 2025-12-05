use chrono::Utc;
use reinhardt::core::serde::json;
use reinhardt::{delete, get, post, put};
use reinhardt::{Json, Path, Request, Response, StatusCode, ViewResult};
use json::json;
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
///
/// GET /snippets/
/// Success response: 200 OK with array of snippets
#[get("/snippets/")]
pub async fn list(_req: Request) -> ViewResult<Response> {
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

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Create a new snippet
///
/// POST /snippets/
/// Request body: JSON with title, code, language fields
/// Success response: 201 Created with created snippet
/// Error responses:
/// - 422 Unprocessable Entity: Validation errors
#[post("/snippets/")]
pub async fn create(
	Json(serializer): Json<SnippetSerializer>,
) -> ViewResult<Response> {
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

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::CREATED)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Retrieve a specific snippet
///
/// GET /snippets/{id}/
/// Success response: 200 OK with snippet data
/// Error responses:
/// - 404 Not Found: Snippet not found
#[get("/snippets/{id}/")]
pub async fn retrieve(
	Path(snippet_id): Path<i64>,
) -> ViewResult<Response> {
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

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Update a snippet
///
/// PUT /snippets/{id}/
/// Request body: JSON with title, code, language fields
/// Success response: 200 OK with updated snippet
/// Error responses:
/// - 404 Not Found: Snippet not found
/// - 422 Unprocessable Entity: Validation errors
#[put("/snippets/{id}/")]
pub async fn update(
	Path(snippet_id): Path<i64>,
	Json(serializer): Json<SnippetSerializer>,
) -> ViewResult<Response> {
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

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Delete a snippet
///
/// DELETE /snippets/{id}/
/// Success response: 204 No Content
/// Error responses:
/// - 404 Not Found: Snippet not found
#[delete("/snippets/{id}/")]
pub async fn delete(
	Path(snippet_id): Path<i64>,
) -> ViewResult<Response> {
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
