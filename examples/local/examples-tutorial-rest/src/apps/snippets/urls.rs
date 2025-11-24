use reinhardt::prelude::*;
use reinhardt::Method;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		// List all snippets: GET /api/snippets/
		.function("/", Method::GET, super::views::list)
		// Create snippet: POST /api/snippets/
		.function("/", Method::POST, super::views::create)
		// Retrieve snippet: GET /api/snippets/<id>/
		.function("/:id/", Method::GET, super::views::retrieve)
		// Update snippet: PUT /api/snippets/<id>/
		.function("/:id/", Method::PUT, super::views::update)
		// Delete snippet: DELETE /api/snippets/<id>/
		.function("/:id/", Method::DELETE, super::views::delete)
}
