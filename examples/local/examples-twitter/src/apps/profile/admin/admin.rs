//! Admin configuration for profile app models
//!
//! Provides admin panel registration for the Profile model.

use reinhardt::admin::panel::{AdminSite, ModelAdminConfig};

/// Register all admin configurations for profile app
pub fn register_admins(site: &AdminSite) {
	let profile_admin = ModelAdminConfig::builder()
		.model_name("Profile")
		.list_display(vec!["id", "user_id", "bio", "location"])
		.list_filter(vec!["location"])
		.search_fields(vec!["bio", "website"])
		.build();

	site.register("Profile", profile_admin)
		.expect("Failed to register Profile admin");
}
