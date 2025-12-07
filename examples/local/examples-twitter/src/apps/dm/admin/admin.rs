//! Admin configuration for dm app models
//!
//! Provides admin panel registration for DM models.

use reinhardt::admin::panel::{AdminSite, ModelAdminConfig};

/// Register all admin configurations for dm app
pub fn register_admins(site: &AdminSite) {
	let room_admin = ModelAdminConfig::builder()
		.model_name("DMRoom")
		.list_display(vec!["id", "name", "is_group", "created_at"])
		.list_filter(vec!["is_group"])
		.search_fields(vec!["name"])
		.build();

	site.register("DMRoom", room_admin)
		.expect("Failed to register DMRoom admin");

	let message_admin = ModelAdminConfig::builder()
		.model_name("DMMessage")
		.list_display(vec!["id", "room_id", "sender_id", "is_read", "created_at"])
		.list_filter(vec!["is_read"])
		.search_fields(vec!["content"])
		.build();

	site.register("DMMessage", message_admin)
		.expect("Failed to register DMMessage admin");
}
