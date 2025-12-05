//! URL configuration for example-rest-api project (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt::{Method, Request, Response, Result, StatusCode, UnifiedRouter};
use reinhardt::core::serde::json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
	pub id: u64,
	pub name: String,
	pub email: String,
}

/// Root endpoint handler
async fn root(_req: Request) -> Result<Response> {
	Ok(Response::new(StatusCode::OK).with_body("Welcome to REST API"))
}

/// Health check endpoint handler
async fn health(_req: Request) -> Result<Response> {
	let body = json::json!({"status": "ok"});
	let json = json::to_string(&body)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

async fn list_users(_req: Request) -> Result<Response> {
	let users = vec![
		User {
			id: 1,
			name: "Alice".to_string(),
			email: "alice@example.com".to_string(),
		},
		User {
			id: 2,
			name: "Bob".to_string(),
			email: "bob@example.com".to_string(),
		},
	];

	let json = json::to_string(&users)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.function("/", Method::GET, root)
		.function("/health", Method::GET, health)
		.function("/api/users", Method::GET, list_users)
		.include("/api/", crate::apps::api::urls::url_patterns())
}
