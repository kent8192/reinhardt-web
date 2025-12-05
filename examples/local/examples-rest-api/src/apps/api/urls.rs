//! URL configuration for api app

use reinhardt::{path, Method, UnifiedRouter};

pub fn url_patterns() -> UnifiedRouter {
	// Use method chaining for UnifiedRouter builder pattern
	UnifiedRouter::new()
		.function(path!("/articles/"), Method::GET, super::views::list_articles)
		.function(path!("/articles/"), Method::POST, super::views::create_article)
		.function(path!("/articles/{id}/"), Method::GET, super::views::get_article)
		.function(path!("/articles/{id}/"), Method::PUT, super::views::update_article)
		.function(
			path!("/articles/{id}/"),
			Method::DELETE,
			super::views::delete_article,
		)
}
