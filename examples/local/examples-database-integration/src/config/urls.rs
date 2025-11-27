//! URL configuration for database-integration example (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt::{Method, Request, Response, Result, StatusCode, UnifiedRouter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
	pub id: u64,
	pub name: String,
	pub email: String,
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

	let _json = serde_json::to_string(&users)?;
	Ok(Response::new(StatusCode::OK))
}

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.function("/api/users", Method::GET, list_users)
		.include("/api/todos", crate::apps::todos::urls::url_patterns())
}
