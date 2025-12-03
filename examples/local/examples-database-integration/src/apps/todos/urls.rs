//! URL configuration for todos app

use reinhardt::{Method, UnifiedRouter};

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.function("/", Method::GET, super::views::list_todos)
		.function("/", Method::POST, super::views::create_todo)
		.function("/{id}", Method::GET, super::views::get_todo)
		.function("/{id}", Method::PUT, super::views::update_todo)
		.function("/{id}", Method::DELETE, super::views::delete_todo)
}
