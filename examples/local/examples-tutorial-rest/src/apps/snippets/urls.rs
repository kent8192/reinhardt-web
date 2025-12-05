use reinhardt::prelude::*;
use reinhardt::{path, Method};

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		// List all snippets: GET /api/snippets/
		.function(path!("/"), Method::GET, super::views::list)
		// Create snippet: POST /api/snippets/
		.function(path!("/"), Method::POST, super::views::create)
		// Retrieve snippet: GET /api/snippets/{id}/
		.function(path!("/{id}/"), Method::GET, super::views::retrieve)
		// Update snippet: PUT /api/snippets/{id}/
		.function(path!("/{id}/"), Method::PUT, super::views::update)
		// Delete snippet: DELETE /api/snippets/{id}/
		.function(path!("/{id}/"), Method::DELETE, super::views::delete)
}
