//! URL configuration for example-rest-api project (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt::{Request, Response, StatusCode, Method, UnifiedRouter, Error, Result};
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

	let json = serde_json::to_string(&users)?;
	Ok(Response::new(StatusCode::OK))
}

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.function("/api/users", Method::GET, list_users)
		.include("/api", crate::apps::api::urls::url_patterns())
}
