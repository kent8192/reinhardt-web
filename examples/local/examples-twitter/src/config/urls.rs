//! URL configuration for examples-twitter project (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

use crate::apps;
use reinhardt::prelude::*;
use std::sync::Arc;

use super::views;

pub fn url_patterns() -> Arc<UnifiedRouter> {
	let mut router = UnifiedRouter::new()
		// Health check endpoint
		.endpoint(views::health_check)
		// Auth routes
		.endpoint(apps::auth::views::register)
		// Profile routes
		.endpoint(apps::profile::views::fetch_profile)
		.endpoint(apps::profile::views::create_profile)
		.endpoint(apps::profile::views::patch_profile)
		// Relationship routes
		.endpoint(apps::relationship::views::follow_user)
		.endpoint(apps::relationship::views::unfollow_user)
		.endpoint(apps::relationship::views::block_user)
		.endpoint(apps::relationship::views::unblock_user)
		.endpoint(apps::relationship::views::fetch_followers)
		.endpoint(apps::relationship::views::fetch_followings)
		.endpoint(apps::relationship::views::fetch_blockings)
		// DM routes
		.endpoint(apps::dm::views::list_rooms)
		.endpoint(apps::dm::views::get_room)
		.endpoint(apps::dm::views::create_room)
		.endpoint(apps::dm::views::delete_room)
		.endpoint(apps::dm::views::list_messages)
		.endpoint(apps::dm::views::send_message)
		.endpoint(apps::dm::views::get_message);

	// Register all routes before returning
	router.register_all_routes();

	Arc::new(router)
}
