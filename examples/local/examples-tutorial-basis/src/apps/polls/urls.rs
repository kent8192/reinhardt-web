use reinhardt::prelude::*;
use reinhardt::{path, Method};

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		// Index view: /polls/
		.function(path!("/"), Method::GET, super::views::index)
		// Detail view: /polls/{question_id}/
		.function(path!("/{question_id}/"), Method::GET, super::views::detail)
		// Results view: /polls/{question_id}/results/
		.function(path!("/{question_id}/results/"), Method::GET, super::views::results)
		// Vote view: /polls/{question_id}/vote/
		.function(path!("/{question_id}/vote/"), Method::POST, super::views::vote)
}
