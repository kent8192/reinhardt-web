//! Admin configuration for auth app models
//!
//! Provides admin panel registration for the User model.

use reinhardt::admin::panel::{AdminSite, ModelAdminConfig};

/// Register all admin configurations for auth app
pub fn register_admins(site: &AdminSite) {
	let user_admin = ModelAdminConfig::builder()
		.model_name("User")
		.list_display(vec!["id", "username", "email", "is_active"])
		.list_filter(vec!["is_active"])
		.search_fields(vec!["username", "email"])
		.build();

	site.register("User", user_admin)
		.expect("Failed to register User admin");
}
