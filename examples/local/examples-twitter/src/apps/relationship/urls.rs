//! URL configuration for relationship app (RESTful)

use crate::apps::relationship::views;
use reinhardt::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		// Follow/Unfollow endpoints
		.endpoint(views::follow_user)
		.endpoint(views::unfollow_user)
		// Block/Unblock endpoints
		.endpoint(views::block_user)
		.endpoint(views::unblock_user)
		// List endpoints
		.endpoint(views::fetch_followers)
		.endpoint(views::fetch_followings)
		.endpoint(views::fetch_blockings)
}
