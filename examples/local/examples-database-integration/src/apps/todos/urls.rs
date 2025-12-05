//! URL configuration for todos app

use reinhardt::{path, Method, UnifiedRouter};

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.function(path!("/"), Method::GET, super::views::list_todos)
		.function(path!("/"), Method::POST, super::views::create_todo)
		.function(path!("/{id}/"), Method::GET, super::views::get_todo)
		.function(path!("/{id}/"), Method::PUT, super::views::update_todo)
		.function(path!("/{id}/"), Method::DELETE, super::views::delete_todo)
}
