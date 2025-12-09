//! URL configuration for profile app (RESTful)

use crate::apps::profile::views;
use reinhardt::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.with_namespace("profile")
		.endpoint(views::fetch_profile)
		.endpoint(views::create_profile)
		.endpoint(views::patch_profile)
}
