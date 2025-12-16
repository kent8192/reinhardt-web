//! Admin panel configuration
//!
//! Configures and builds the admin interface for examples-twitter.

use reinhardt::UnifiedRouter;
use reinhardt::admin::panel::AdminSite;
use reinhardt::db::DatabaseConnection;

use crate::apps;

/// Build the admin panel router
///
/// Creates an AdminSite, registers all model admins from each app,
/// and returns a UnifiedRouter with all admin endpoints.
///
/// # Endpoints
///
/// - `GET /` - Dashboard (list of registered models)
/// - `GET /favicon.ico` - Favicon
/// - `GET /{model}/` - List model instances
/// - `GET /{model}/{id}/` - Get model instance detail
/// - `POST /{model}/` - Create model instance
/// - `PUT /{model}/{id}/` - Update model instance
/// - `DELETE /{model}/{id}/` - Delete model instance
/// - `POST /{model}/bulk-delete/` - Bulk delete model instances
/// - `GET /{model}/export/` - Export model data
/// - `POST /{model}/import/` - Import model data
pub fn configure_admin(db: DatabaseConnection) -> UnifiedRouter {
	let site = AdminSite::new("Twitter Admin");

	// Register admin configurations from each app
	apps::auth::admin::register_admins(&site);
	apps::profile::admin::register_admins(&site);
	apps::relationship::admin::register_admins(&site);
	apps::dm::admin::register_admins(&site);

	// Build router with favicon
	site.get_router(db)
		.with_favicon_file("branding/logo.png")
		.build()
}

/// Get the admin site without building the router
///
/// Use this when you need more control over the admin configuration.
pub fn get_admin_site() -> AdminSite {
	let site = AdminSite::new("Twitter Admin");

	apps::auth::admin::register_admins(&site);
	apps::profile::admin::register_admins(&site);
	apps::relationship::admin::register_admins(&site);
	apps::dm::admin::register_admins(&site);

	site
}
