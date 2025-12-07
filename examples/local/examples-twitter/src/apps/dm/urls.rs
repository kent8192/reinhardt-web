//! URL configuration for dm app (RESTful)

use crate::apps::dm::views;
use reinhardt::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		// Room endpoints
		.endpoint(views::list_rooms)
		.endpoint(views::get_room)
		.endpoint(views::create_room)
		.endpoint(views::delete_room)
		// Message endpoints
		.endpoint(views::list_messages)
		.endpoint(views::send_message)
		.endpoint(views::get_message)
}
