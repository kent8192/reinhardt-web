//! Admin router integration
//!
//! This module provides router integration for admin panel,
//! generating UnifiedRouter from AdminSite configuration.
//!
//! All endpoints are registered automatically using the `.endpoint()` method
//! with HTTP method macros from handlers module.

use crate::AdminSite;
use reinhardt_urls::routers::UnifiedRouter;
use std::sync::Arc;

/// Admin router builder
///
/// Builds a UnifiedRouter from an AdminSite with all CRUD endpoints.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_admin_api::{AdminSite, admin_routes};
///
/// let site = AdminSite::new("My Admin");
/// // ... register models ...
///
/// let router = admin_routes();
/// ```
pub fn admin_routes() -> UnifiedRouter {
	// TODO: Implement when Server Functions are integrated
	// This will be replaced with reinhardt-pages routing
	UnifiedRouter::new().with_namespace("admin")
	// Old REST API handlers (from api/ crate) will be replaced with Server Functions
	// .endpoint(crate::handlers::dashboard)
	// .endpoint(crate::handlers::favicon)
	// .endpoint(crate::handlers::list)
	// .endpoint(crate::handlers::detail)
	// .endpoint(crate::handlers::create)
	// .endpoint(crate::handlers::update)
	// .endpoint(crate::handlers::delete)
	// .endpoint(crate::handlers::bulk_delete)
	// .endpoint(crate::handlers::export)
	// .endpoint(crate::handlers::import)
}

/// Admin router builder (for backward compatibility)
///
/// This struct is kept for backward compatibility with existing code.
/// New code should use `admin_routes()` function directly.
pub struct AdminRouter {
	site: Arc<AdminSite>,
}

impl AdminRouter {
	/// Create a new admin router builder from Arc-wrapped site
	pub fn from_arc(site: Arc<AdminSite>) -> Self {
		Self { site }
	}

	/// Set favicon from file path
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_admin_api::{AdminSite, AdminRouter};
	/// use std::sync::Arc;
	///
	/// let site = Arc::new(AdminSite::new("Admin"));
	/// let router = AdminRouter::from_arc(site)
	///     .with_favicon_file("static/favicon.ico")
	///     .build();
	/// ```
	///
	/// # Panics
	///
	/// Panics if the file cannot be read.
	pub fn with_favicon_file(self, path: impl AsRef<std::path::Path>) -> Self {
		let data = std::fs::read(path.as_ref()).expect("Failed to read favicon file");
		self.site.set_favicon(data);
		self
	}

	/// Set favicon from raw bytes
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// // Cannot run: favicon.ico file does not exist
	/// use reinhardt_admin_api::{AdminSite, AdminRouter};
	/// use std::sync::Arc;
	///
	/// let favicon_bytes = include_bytes!("favicon.ico").to_vec();
	/// let site = Arc::new(AdminSite::new("Admin"));
	/// let router = AdminRouter::from_arc(site)
	///     .with_favicon_bytes(favicon_bytes)
	///     .build();
	/// ```
	pub fn with_favicon_bytes(self, data: Vec<u8>) -> Self {
		self.site.set_favicon(data);
		self
	}

	/// Build the UnifiedRouter with all admin endpoints
	///
	/// Generated endpoints:
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
	pub fn routes(&self) -> UnifiedRouter {
		admin_routes()
	}

	/// Build the UnifiedRouter (alias for routes())
	pub fn build(self) -> UnifiedRouter {
		admin_routes()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_admin_routes_creates_router() {
		let router = admin_routes();
		// Verify router is created with admin namespace
		assert_eq!(router.namespace(), Some("admin"));
	}

	#[test]
	fn test_admin_router_backward_compat() {
		let site = Arc::new(AdminSite::new("Test Admin"));
		let router_builder = AdminRouter::from_arc(site);
		let router = router_builder.routes();
		// Verify router is created
		assert_eq!(router.namespace(), Some("admin"));
	}
}
