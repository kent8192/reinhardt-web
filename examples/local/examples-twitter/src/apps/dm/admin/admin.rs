//! Admin configuration for dm app models
//!
//! Note: Admin integration requires async_trait dependency.
//! For now, we provide a placeholder implementation.

use reinhardt::admin::panel::AdminSite;

/// Register all admin configurations for dm app
pub fn register_admins(_site: &AdminSite) {
	// TODO: Add admin registration when async_trait is available
	// site.register("DMRoom", DMRoomAdmin);
	// site.register("DMMessage", DMMessageAdmin);
}
