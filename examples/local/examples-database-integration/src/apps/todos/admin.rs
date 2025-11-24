//! Admin configuration for todos app

use reinhardt::admin::panel::ModelAdminConfig;

/// Admin configuration for Todo model
///
/// This configures the admin panel display for Todo items:
/// - List view with id, title, completion status, and creation date
/// - Filtering by completion status
/// - Search by title and description
/// - Sorted by creation date (newest first)
/// - 50 items per page
pub struct TodoAdmin;

impl TodoAdmin {
	/// Returns the ModelAdminConfig for Todo model
	///
	/// # Example
	///
	/// ```rust,ignore
	/// use reinhardt_panel::AdminSite;
	/// use crate::apps::todos::admin::TodoAdmin;
	///
	/// let mut admin = AdminSite::new("Todo Management");
	/// admin.register("Todo", TodoAdmin::config())?;
	/// ```
	pub fn config() -> ModelAdminConfig {
		ModelAdminConfig::builder()
			.model_name("Todo")
			.list_display(vec!["id", "title", "completed", "created_at"])
			.list_filter(vec!["completed"])
			.search_fields(vec!["title", "description"])
			.ordering(vec!["-created_at"])
			.list_per_page(50)
			.build()
	}
}
