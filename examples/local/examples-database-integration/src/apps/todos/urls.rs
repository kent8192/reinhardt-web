//! URL configuration for todos app

use reinhardt::{Method, UnifiedRouter};

pub fn url_patterns() -> UnifiedRouter {
	let mut router = UnifiedRouter::builder().build();

	// Add CRUD endpoints
	router.function("/", Method::GET, super::views::list_todos);
	router.function("/", Method::POST, super::views::create_todo);
	router.function("/:id", Method::GET, super::views::get_todo);
	router.function("/:id", Method::PUT, super::views::update_todo);
	router.function("/:id", Method::DELETE, super::views::delete_todo);

	router
}
